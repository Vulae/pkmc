use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    hash::Hash,
    io::{Seek, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use pkmc_defs::{
    biome::Biome,
    block::{Block, BlockEntity},
    packet,
};
use pkmc_generated::consts::{
    PALETTED_DATA_BIOMES_DIRECT, PALETTED_DATA_BIOMES_INDIRECT, PALETTED_DATA_BLOCKS_DIRECT,
    PALETTED_DATA_BLOCKS_INDIRECT,
};
use pkmc_util::{
    nbt::{from_nbt, NBTError, NBT},
    packet::{
        calculate_bpe, to_paletted_data, to_paletted_data_precomputed, to_paletted_data_singular,
        ConnectionError, ConnectionSender,
    },
    IdTable, PackedArray, Position, ReadExt, Transmutable, Vec3, WeakList,
};
use serde::Deserialize;
use thiserror::Error;

use crate::world::{chunk_loader::ChunkPosition, SECTION_SIZE};

use super::{
    chunk_loader::ChunkLoader, World, WorldBlock, WorldViewer, CHUNK_SIZE, SECTION_BIOMES,
    SECTION_BLOCKS,
};

pub const REGION_SIZE: usize = 32;
pub const CHUNKS_PER_REGION: usize = REGION_SIZE * REGION_SIZE;

// Each time the world updates & sends new data to client, we either send sections or chunks.
// NOTE: When sending sections, the client calculates lighting instead of server.
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_SECTIONS: usize = 4;
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_BLOCKS: usize = 1024;

#[derive(Error, Debug)]
pub enum AnvilError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error("Region chunk unknown compression \"{0}\"")]
    RegionUnknownCompression(u8),
    #[error("Region chunk unsupported compression \"{0}\"")]
    RegionUnsupportedCompression(String),
    #[error(transparent)]
    NBTError(#[from] NBTError),
}

fn default_paletted_data<T: Default>() -> Box<[T]> {
    vec![T::default()].into_boxed_slice()
}

#[derive(Debug, Deserialize)]
struct PalettedData<T: Debug + Default, const N: usize, const I_S: u8, const I_E: u8> {
    #[serde(default = "default_paletted_data")]
    palette: Box<[T]>,
    #[serde(default)]
    data: Box<[i64]>,
}

impl<T: Debug + Default, const N: usize, const I_S: u8, const I_E: u8>
    PalettedData<T, N, I_S, I_E>
{
    fn bpe(palette_count: usize) -> u8 {
        match palette_count {
            0 => panic!(),
            1 => 0,
            // NOTE: Data stored inside the world files doesn't have direct paletting.
            palette_count => PackedArray::bits_per_entry(palette_count as u64 - 1).max(I_S),
        }
    }

    fn palette_index(&self, index: usize) -> usize {
        debug_assert!(index < N);
        match Self::bpe(self.palette.len()) {
            0 => 0,
            bpe => PackedArray::from_inner(self.data.as_ref().transmute(), bpe, N)
                .get(index)
                .unwrap() as usize,
        }
    }

    fn get(&self, index: usize) -> &T {
        let palette_index = self.palette_index(index);
        debug_assert!(palette_index < self.palette.len());
        &self.palette[palette_index]
    }
}

impl<T: Debug + Default + Eq + Clone + Hash, const N: usize, const I_S: u8, const I_E: u8>
    PalettedData<T, N, I_S, I_E>
{
    fn set(&mut self, index: usize, value: T) -> bool {
        if *self.get(index) == value {
            return false;
        }

        if let Some(palette_index) = self.palette.iter().position(|v| *v == value) {
            match Self::bpe(self.palette.len()) {
                // Previous check should have caught this.
                0 => unreachable!(),
                bpe => {
                    PackedArray::from_inner(self.data.as_mut().transmute(), bpe, N)
                        .set(index, palette_index as u64);
                    return true;
                }
            }
        }

        let mut parsed: [T; N] = (0..N)
            .map(|i| self.get(i))
            .cloned()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        parsed[index] = value;

        let mut palette = HashMap::new();
        parsed.iter().for_each(|v| {
            let count = palette.len();
            palette.entry(v.clone()).or_insert(count);
        });

        match Self::bpe(palette.len()) {
            0 => {
                self.palette = palette.into_keys().collect();
                self.data = Vec::new().into_boxed_slice();
            }
            bpe => {
                let mut data = PackedArray::new(bpe, N);

                let remaining = data
                    .consume(parsed.iter().map(|v| *palette.get(v).unwrap() as u64))
                    .count();
                debug_assert_eq!(remaining, 0);

                self.palette = palette
                    .into_iter()
                    .sorted_by(|(_, a), (_, b)| a.cmp(b))
                    .map(|(k, _)| k)
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                self.data = data.into_inner().to_vec().into_boxed_slice().transmute();
            }
        }

        true
    }
}

const PALETTED_DATA_BLOCKS_INDIRECT_START: u8 = *PALETTED_DATA_BLOCKS_INDIRECT.start();
const PALETTED_DATA_BLOCKS_INDIRECT_END: u8 = *PALETTED_DATA_BLOCKS_INDIRECT.end();
type ChunkSectionBlockStates = PalettedData<
    Block,
    SECTION_BLOCKS,
    PALETTED_DATA_BLOCKS_INDIRECT_START,
    PALETTED_DATA_BLOCKS_INDIRECT_END,
>;

impl ChunkSectionBlockStates {
    fn get_block_index(x: u8, y: u8, z: u8) -> usize {
        debug_assert!((x as usize) < SECTION_SIZE);
        debug_assert!((y as usize) < SECTION_SIZE);
        debug_assert!((z as usize) < SECTION_SIZE);
        (y as usize) * SECTION_SIZE * SECTION_SIZE + (z as usize) * SECTION_SIZE + (x as usize)
    }

    fn get_block(&self, x: u8, y: u8, z: u8) -> &Block {
        self.get(Self::get_block_index(x, y, z))
    }

    fn set_block(&mut self, x: u8, y: u8, z: u8, block: Block) -> bool {
        self.set(Self::get_block_index(x, y, z), block)
    }

    fn write(&self, mut writer: impl Write) -> Result<(), AnvilError> {
        //if self.palette.len() == 1 {
        //    let id = self.palette[0]
        //        .id_with_default_fallback()
        //        .unwrap_or_else(|| Block::air().id().unwrap());
        //    writer.write_all(
        //        &if generated::block::is_air(id) {
        //            0u16
        //        } else {
        //            4096
        //        }
        //        .to_be_bytes(),
        //    )?;
        //    writer.write_all(&to_paletted_data_singular(id)?)?;
        //    return Ok(());
        //}

        let block_ids = self
            .palette
            .iter()
            .map(|b| {
                b.id_with_default_fallback()
                    .unwrap_or_else(|| Block::air().id().unwrap())
            })
            .collect::<Box<[i32]>>();

        let block_count = (0..SECTION_BLOCKS)
            .filter(|i| !pkmc_generated::block::is_air(block_ids[self.palette_index(*i)]))
            .count();

        writer.write_all(&(block_count as u16).to_be_bytes())?;

        const FORCE_CHUNK_REENCODE: bool = false;

        if !FORCE_CHUNK_REENCODE
            // NOTE: Data stored in the anvil format doesn't have direct paletting.
            // So we need to re-encode the data if there's too many palette values.
            && calculate_bpe(block_ids.len()) <= PALETTED_DATA_BLOCKS_INDIRECT_END
        {
            writer.write_all(&to_paletted_data_precomputed(
                &block_ids,
                &self.data,
                PALETTED_DATA_BLOCKS_INDIRECT,
                PALETTED_DATA_BLOCKS_DIRECT,
            )?)?;
        } else {
            writer.write_all(&to_paletted_data(
                &(0..SECTION_BLOCKS)
                    .map(|i| block_ids[self.palette_index(i)])
                    .collect::<Box<[_]>>(),
                PALETTED_DATA_BLOCKS_INDIRECT,
                PALETTED_DATA_BLOCKS_DIRECT,
            )?)?;
        }

        Ok(())
    }
}

const PALETTED_DATA_BIOMES_INDIRECT_START: u8 = *PALETTED_DATA_BIOMES_INDIRECT.start();
const PALETTED_DATA_BIOMES_INDIRECT_END: u8 = *PALETTED_DATA_BIOMES_INDIRECT.end();
type ChunkSectionBiomes = PalettedData<
    Biome,
    SECTION_BIOMES,
    PALETTED_DATA_BIOMES_INDIRECT_START,
    PALETTED_DATA_BIOMES_INDIRECT_END,
>;

impl ChunkSectionBiomes {
    fn write(&self, mut writer: impl Write, mapper: &IdTable<Biome>) -> Result<(), AnvilError> {
        let biome_ids = self
            .palette
            .iter()
            .map(|b| {
                b.id(mapper)
                    .unwrap_or_else(|| Biome::default().id(mapper).unwrap())
            })
            .collect::<Box<[_]>>();

        writer.write_all(&to_paletted_data(
            &(0..SECTION_BIOMES)
                .map(|i| biome_ids[self.palette_index(i)])
                .collect::<Box<[_]>>(),
            PALETTED_DATA_BIOMES_INDIRECT,
            PALETTED_DATA_BIOMES_DIRECT,
        )?)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ChunkSection {
    #[serde(rename = "Y")]
    y: i8,
    block_states: Option<ChunkSectionBlockStates>,
    biomes: Option<ChunkSectionBiomes>,
}

#[derive(Debug, Deserialize, Clone)]
struct AnvilBlockEntity {
    id: String,
    #[allow(unused)]
    #[serde(rename = "keepPacked", default)]
    keep_packed: bool,
    x: i32,
    y: i16,
    z: i32,
    #[serde(flatten)]
    data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AnvilChunk {
    //#[serde(rename = "DataVersion")]
    //data_version: i32,
    #[serde(rename = "xPos")]
    x_pos: i32,
    #[serde(rename = "zPos")]
    z_pos: i32,
    #[serde(rename = "yPos")]
    y_pos: Option<i8>,
    #[serde(skip, default)]
    section_y_pos: i8,
    //#[serde(rename = "Status")]
    //status: String,
    //#[serde(rename = "LastUpdate")]
    //last_update: i64,
    sections: Vec<ChunkSection>,
    block_entities: Vec<AnvilBlockEntity>,
    #[serde(skip, default)]
    parsed_block_entities: HashMap<(u8, i16, u8), BlockEntity>,
}

impl AnvilChunk {
    fn initialize(&mut self, section_y_range: std::ops::RangeInclusive<i8>) {
        if let Some(y_pos) = self.y_pos {
            assert!(*section_y_range.start() == y_pos);
        }
        self.section_y_pos = *section_y_range.start();

        // Insert missing sections
        section_y_range.for_each(|section_y| {
            if self.sections.iter().any(|section| section.y == section_y) {
                return;
            }
            self.sections.push(ChunkSection {
                y: section_y,
                block_states: None,
                biomes: None,
            })
        });

        // Sometimes sections are unsorted.
        // And also the inserting of sections in the above code may also cause it to become unsorted.
        self.sections.sort_by(|a, b| a.y.cmp(&b.y));

        self.parsed_block_entities = self
            .block_entities
            .iter()
            .map(|b| {
                let bx = b.x.rem_euclid(CHUNK_SIZE as i32) as u8;
                let bz = b.z.rem_euclid(CHUNK_SIZE as i32) as u8;
                (
                    (bx, b.y, bz),
                    BlockEntity::new(
                        self.get_tile_block(bx, b.y, bz).unwrap(),
                        b.id.clone(),
                        NBT::try_from(serde_json::Value::from_iter(b.data.clone())).unwrap(),
                    ),
                )
            })
            .collect();
    }

    fn get_section(&self, section_y: i8) -> Option<&ChunkSection> {
        self.sections.iter().find(|section| section.y == section_y)
    }

    fn get_section_mut(&mut self, section_y: i8) -> Option<&mut ChunkSection> {
        //self.sections
        //    .iter_mut()
        //    .find(|section| section.y == section_y)
        self.sections
            .get_mut((section_y - self.section_y_pos) as usize)
    }

    fn get_tile_block(&self, block_x: u8, block_y: i16, block_z: u8) -> Option<Block> {
        debug_assert!((block_x as usize) < SECTION_SIZE);
        debug_assert!((block_z as usize) < SECTION_SIZE);
        Some(
            self.get_section(block_y.div_euclid(SECTION_SIZE as i16) as i8)?
                .block_states
                .as_ref()?
                .get_block(
                    block_x,
                    (block_y.rem_euclid(SECTION_SIZE as i16)) as u8,
                    block_z,
                )
                .clone(),
        )
    }

    fn get_block(&self, block_x: u8, block_y: i16, block_z: u8) -> Option<WorldBlock> {
        // TODO: WorldBlock::BlockEntity
        self.get_tile_block(block_x, block_y, block_z)
            .map(WorldBlock::Block)
    }

    fn set_block(&mut self, block_x: u8, block_y: i16, block_z: u8, block: WorldBlock) -> bool {
        debug_assert!((block_x as usize) < SECTION_SIZE);
        debug_assert!((block_z as usize) < SECTION_SIZE);

        let block = match block {
            WorldBlock::Block(block) => {
                self.parsed_block_entities
                    .remove(&(block_x, block_y, block_z));
                block
            }
            WorldBlock::BlockEntity(block_entity) => {
                let block = block_entity.block.clone();

                self.parsed_block_entities
                    .insert((block_x, block_y, block_z), block_entity);

                block
            }
        };

        let Some(section) = self.get_section_mut(block_y.div_euclid(SECTION_SIZE as i16) as i8)
        else {
            return false;
        };
        let Some(block_states) = section.block_states.as_mut() else {
            return false;
        };

        block_states.set_block(
            block_x,
            (block_y.rem_euclid(SECTION_SIZE as i16)) as u8,
            block_z,
            block,
        )
    }

    fn block_entities(&self) -> &HashMap<(u8, i16, u8), BlockEntity> {
        &self.parsed_block_entities
    }

    fn to_packet(
        &self,
        biome_mapper: &IdTable<Biome>,
    ) -> Result<packet::play::LevelChunkWithLight, AnvilError> {
        Ok(packet::play::LevelChunkWithLight {
            chunk_x: self.x_pos,
            chunk_z: self.z_pos,
            chunk_data: packet::play::LevelChunkData {
                heightmaps: NBT::Compound(HashMap::new()),
                data: {
                    let mut writer = Vec::new();

                    self.sections.iter().try_for_each(|section| {
                        if let Some(block_states) = &section.block_states {
                            block_states.write(&mut writer)?;
                        } else {
                            writer.write_all(&0u16.to_be_bytes())?;
                            writer.write_all(&to_paletted_data_singular(
                                Block::air().id().unwrap(),
                            )?)?;
                        }
                        if let Some(biomes) = &section.biomes {
                            biomes.write(&mut writer, biome_mapper)?;
                        } else {
                            writer.write_all(&to_paletted_data_singular(
                                Biome::default().id(biome_mapper).unwrap(),
                            )?)?;
                        }
                        Ok::<_, AnvilError>(())
                    })?;

                    writer.into_boxed_slice()
                },
                block_entities: self
                    .block_entities()
                    .iter()
                    .map(|((x, y, z), b)| packet::play::BlockEntity {
                        x: *x,
                        z: *z,
                        y: *y,
                        r#type: b.block_entity_id().unwrap(),
                        data: b.data.clone(),
                    })
                    .collect(),
            },
            // TODO: Light data
            light_data: packet::play::LevelLightData::full_bright(self.sections.len()),
        })
    }
}

#[derive(Debug)]
#[allow(unused)]
struct Region {
    file: File,
    region_x: i32,
    region_z: i32,
    locations: [(u32, u32); CHUNKS_PER_REGION],
    loaded_chunks: HashMap<(u8, u8), Option<AnvilChunk>>,
}

impl Region {
    fn load(mut file: File, region_x: i32, region_z: i32) -> Result<Self, AnvilError> {
        let mut locations = [(0, 0); REGION_SIZE * REGION_SIZE];
        file.rewind()?;
        locations.iter_mut().try_for_each(|(offset, length)| {
            let data = u32::from_be_bytes(file.read_const()?);
            *offset = ((data & 0xFFFFFF00) >> 8) * 0x1000;
            *length = (data & 0x000000FF) * 0x1000;
            Ok::<_, AnvilError>(())
        })?;
        Ok(Self {
            file,
            region_x,
            region_z,
            locations,
            loaded_chunks: HashMap::new(),
        })
    }

    fn read(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Box<[u8]>>, AnvilError> {
        let (offset, length) =
            self.locations[(chunk_x as usize) + (chunk_z as usize) * REGION_SIZE];
        if offset == 0 || length == 0 {
            return Ok(None);
        }
        self.file.seek(std::io::SeekFrom::Start(offset as u64))?;
        let length = u32::from_be_bytes(self.file.read_const()?);
        if length <= 1 {
            return Ok(None);
        }
        let compression_type = u8::from_be_bytes(self.file.read_const()?);
        let compressed_data = self.file.read_var((length as usize) - 1)?;
        match compression_type {
            1 => Err(AnvilError::RegionUnsupportedCompression("GZip".to_owned())),
            2 => Ok(Some(
                flate2::read::ZlibDecoder::new(std::io::Cursor::new(compressed_data)).read_all()?,
            )),
            3 => Ok(Some(compressed_data)),
            4 => Err(AnvilError::RegionUnsupportedCompression("LZ4".to_owned())),
            127 => {
                let mut data = std::io::Cursor::new(&compressed_data);
                let string_length = u16::from_be_bytes(data.read_const()?);
                let string_buf = data.read_var(string_length as usize)?;
                Err(AnvilError::RegionUnsupportedCompression(format!(
                    "Custom {}",
                    String::from_utf8_lossy(&string_buf)
                )))
            }
            _ => Err(AnvilError::RegionUnknownCompression(compression_type)),
        }
    }

    fn read_nbt(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<(String, NBT)>, AnvilError> {
        Ok(self
            .read(chunk_x, chunk_z)?
            .map(|data| NBT::read(std::io::Cursor::new(data)))
            .transpose()?)
    }

    fn prepare_chunk(
        &mut self,
        chunk_x: u8,
        chunk_z: u8,
        section_y_range: std::ops::RangeInclusive<i8>,
    ) -> Result<(), AnvilError> {
        if self.loaded_chunks.contains_key(&(chunk_x, chunk_z)) {
            return Ok(());
        }

        match self
            .read_nbt(chunk_x, chunk_z)?
            .map(|nbt| from_nbt::<AnvilChunk>(nbt.1))
            .transpose()?
        {
            Some(mut chunk) => {
                chunk.initialize(section_y_range);
                self.loaded_chunks.insert((chunk_x, chunk_z), Some(chunk));
            }
            None => {
                self.loaded_chunks.insert((chunk_x, chunk_z), None);
            }
        }

        Ok(())
    }

    fn get_chunk(&self, chunk_x: u8, chunk_z: u8) -> Option<&AnvilChunk> {
        self.loaded_chunks
            .get(&(chunk_x, chunk_z))
            .and_then(|i| i.as_ref())
    }

    fn get_chunk_mut(&mut self, chunk_x: u8, chunk_z: u8) -> Option<&mut AnvilChunk> {
        self.loaded_chunks
            .get_mut(&(chunk_x, chunk_z))
            .and_then(|i| i.as_mut())
    }
}

#[derive(Debug, Default)]
struct SectionDiff {
    // TODO: Don't use hashmap for this.
    change: HashMap<(u8, u8, u8), i32>,
}

impl SectionDiff {
    fn set(&mut self, x: u8, y: u8, z: u8, id: i32) {
        assert!((x as usize) < SECTION_SIZE);
        assert!((y as usize) < SECTION_SIZE);
        assert!((z as usize) < SECTION_SIZE);
        self.change.insert((x, y, z), id);
    }

    fn num_blocks(&self) -> usize {
        self.change.len()
    }

    fn to_packet_data(&self) -> Vec<(u8, u8, u8, i32)> {
        self.change
            .iter()
            .map(|((x, y, z), v)| (*x, *y, *z, *v))
            .collect()
    }
}

#[derive(Debug)]
pub struct AnvilWorld {
    root: PathBuf,
    identifier: String,
    loaded_regions: HashMap<(i32, i32), Option<Region>>,
    section_y_range: std::ops::RangeInclusive<i8>,
    biome_mapper: IdTable<Biome>,
    viewers: WeakList<Mutex<WorldViewer>>,
    diffs: HashMap<(i32, i32), HashMap<i16, SectionDiff>>,
}

impl AnvilWorld {
    pub fn new<P: Into<PathBuf>>(
        root: P,
        identifier: &str,
        section_y_range: std::ops::RangeInclusive<i8>,
        biome_mapper: IdTable<Biome>,
    ) -> Self {
        Self {
            root: root.into(),
            identifier: identifier.to_owned(),
            loaded_regions: HashMap::new(),
            section_y_range,
            biome_mapper,
            viewers: WeakList::new(),
            diffs: HashMap::new(),
        }
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    fn prepare_region(&mut self, region_x: i32, region_z: i32) -> Result<(), AnvilError> {
        if self.loaded_regions.contains_key(&(region_x, region_z)) {
            return Ok(());
        }

        let mut path = self.root.clone();
        path.push("region");
        path.push(format!("r.{}.{}.mca", region_x, region_z));

        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.loaded_regions.insert((region_x, region_z), None);
                return Ok(());
            }
            result => result,
        }?;

        self.loaded_regions.insert(
            (region_x, region_z),
            Some(Region::load(file, region_x, region_z)?),
        );

        Ok(())
    }

    fn get_region(&self, region_x: i32, region_z: i32) -> Option<&Region> {
        self.loaded_regions
            .get(&(region_x, region_z))
            .and_then(|i| i.as_ref())
    }

    fn get_region_mut(&mut self, region_x: i32, region_z: i32) -> Option<&mut Region> {
        self.loaded_regions
            .get_mut(&(region_x, region_z))
            .and_then(|i| i.as_mut())
    }

    fn prepare_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<(), AnvilError> {
        let region_x = chunk_x.div_euclid(REGION_SIZE as i32);
        let region_z = chunk_z.div_euclid(REGION_SIZE as i32);

        self.prepare_region(region_x, region_z)?;

        let section_y_range = self.section_y_range();
        if let Some(region) = self.get_region_mut(region_x, region_z) {
            region.prepare_chunk(
                chunk_x.wrapping_rem_euclid(REGION_SIZE as i32) as u8,
                chunk_z.wrapping_rem_euclid(REGION_SIZE as i32) as u8,
                section_y_range,
            )?;
        }

        Ok(())
    }

    fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&AnvilChunk> {
        let region = self.get_region(
            chunk_x.div_euclid(REGION_SIZE as i32),
            chunk_z.div_euclid(REGION_SIZE as i32),
        )?;
        let chunk = region.get_chunk(
            (chunk_x.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
            (chunk_z.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
        )?;
        Some(chunk)
    }

    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut AnvilChunk> {
        let region = self.get_region_mut(
            chunk_x.div_euclid(REGION_SIZE as i32),
            chunk_z.div_euclid(REGION_SIZE as i32),
        )?;
        let chunk = region.get_chunk_mut(
            (chunk_x.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
            (chunk_z.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
        )?;
        Some(chunk)
    }

    fn section_y_range(&self) -> std::ops::RangeInclusive<i8> {
        self.section_y_range.clone()
    }
}

impl World for AnvilWorld {
    type Error = AnvilError;

    fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<WorldViewer>> {
        let viewer = WorldViewer {
            connection,
            loader: ChunkLoader::new(6),
            position: Vec3::new(0.0, 100.0, 0.0),
        };
        self.viewers.push(Mutex::new(viewer))
    }

    fn update_viewers(&mut self) -> Result<(), Self::Error> {
        self.viewers.cleanup();

        let mut viewers = self.viewers.lock();

        self.diffs
            .drain()
            .try_for_each(|((chunk_x, chunk_z), sections)| {
                let chunk_position = ChunkPosition::new(chunk_x, chunk_z);
                if sections.len() >= UPDATE_SECTION_CHUNK_SWITCH_NUM_SECTIONS
                    || sections.values().fold(0, |t, s| t + s.num_blocks())
                        >= UPDATE_SECTION_CHUNK_SWITCH_NUM_BLOCKS
                {
                    // Just resend the whole chunk
                    viewers
                        .iter_mut()
                        .for_each(|viewer| viewer.loader.force_reload(chunk_position));
                    Ok(())
                } else {
                    // Resend each section
                    sections.into_iter().try_for_each(|(section_y, diff)| {
                        let packet = packet::play::UpdateSectionBlocks {
                            section: Position::new(chunk_x, section_y, chunk_z),
                            blocks: diff.to_packet_data(),
                        };
                        viewers
                            .iter()
                            .filter(|viewer| viewer.loader.has_loaded(chunk_position))
                            .try_for_each(|viewer| viewer.connection().send(&packet))
                    })
                }
            })?;

        viewers.iter_mut().try_for_each(|viewer| {
            let center = ChunkPosition::new(
                (viewer.position.x / 16.0) as i32,
                (viewer.position.z / 16.0) as i32,
            );
            if viewer.loader.update_center(Some(center)) {
                viewer
                    .connection()
                    .send(&packet::play::SetChunkCacheCenter {
                        chunk_x: center.chunk_x,
                        chunk_z: center.chunk_z,
                    })?;
            }

            while let Some(to_unload) = viewer.loader.next_to_unload() {
                viewer.connection().send(&packet::play::ForgetLevelChunk {
                    chunk_x: to_unload.chunk_x,
                    chunk_z: to_unload.chunk_z,
                })?;
            }

            if let Some(to_load) = viewer.loader.next_to_load() {
                self.prepare_chunk(to_load.chunk_x, to_load.chunk_z)?;
                if let Some(chunk) = self.get_chunk(to_load.chunk_x, to_load.chunk_z) {
                    viewer
                        .connection()
                        .send(&chunk.to_packet(&self.biome_mapper)?)?;
                } else {
                    viewer
                        .connection()
                        .send(&packet::play::LevelChunkWithLight::generate_test(
                            to_load.chunk_x,
                            to_load.chunk_z,
                            self.section_y_range().count(),
                        )?)?;
                }
            }

            Ok::<(), Self::Error>(())
        })?;

        Ok(())
    }

    fn get_block(&mut self, position: Position) -> Result<Option<WorldBlock>, Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.prepare_chunk(chunk_x, chunk_z)?;
        let Some(chunk) = self.get_chunk(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        ) else {
            return Ok(None);
        };
        Ok(chunk.get_block(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
        ))
    }

    fn set_block(&mut self, position: Position, block: WorldBlock) -> Result<(), Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.prepare_chunk(chunk_x, chunk_z)?;
        let Some(chunk) = self.get_chunk_mut(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        ) else {
            return Ok(());
        };
        if chunk.set_block(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
            block.clone(),
        ) {
            self.diffs
                .entry((
                    position.x.div_euclid(SECTION_SIZE as i32),
                    position.z.div_euclid(SECTION_SIZE as i32),
                ))
                .or_default()
                .entry(position.y.div_euclid(SECTION_SIZE as i16))
                .or_default()
                .set(
                    position.x.rem_euclid(SECTION_SIZE as i32) as u8,
                    position.y.rem_euclid(SECTION_SIZE as i16) as u8,
                    position.z.rem_euclid(SECTION_SIZE as i32) as u8,
                    block
                        .as_block()
                        .id_with_default_fallback()
                        .unwrap_or_else(|| Block::air().id().unwrap()),
                );
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use pkmc_defs::block::BLOCKS_TO_IDS;
    use pkmc_util::Position;

    use crate::world::{anvil::AnvilWorld, World as _};

    use super::AnvilError;

    #[test]
    fn test_debug_mode_world() -> Result<(), AnvilError> {
        // 1.21.4 debug world
        // https://minecraft.wiki/w/Debug_mode
        const WORLD_PATH: &str = "./src/world/anvil-test-server/world/";
        println!(
            "Testing debug world: {:?}",
            std::fs::canonicalize(WORLD_PATH)?
        );

        let mut world = AnvilWorld::new(
            WORLD_PATH,
            "minecraft:overworld",
            -4..=20,
            Default::default(),
        );

        let max_block_id = *BLOCKS_TO_IDS.values().max().unwrap();

        let start = std::time::Instant::now();
        println!("Total block ids: {}", max_block_id);

        let block_grid_width = (max_block_id as f32).sqrt().ceil() as i32;
        let _block_grid_height = (max_block_id as f32 / block_grid_width as f32).ceil() as i32;

        for block_id in 0..max_block_id {
            if block_id % 1024 == 0 {
                println!("Checking {} / {}", block_id, max_block_id);
            }

            let grid_x = block_id % block_grid_width;
            let grid_z = block_id / block_grid_width;

            let position = Position::new(1 + grid_z * 2, 70, 1 + grid_x * 2);

            let Some(block) = world.get_block(position)?.map(|b| b.into_block()) else {
                panic!("Expected loaded block at {:?}", position);
            };

            if block.id() != Some(block_id) {
                panic!(
                    "Block at {:?} is {:?} with ID {:?}, but our ID is {}",
                    position,
                    block,
                    block.id(),
                    block_id
                );
            }
        }

        println!(
            "Checked all {} block ids in {:.2} seconds",
            max_block_id,
            std::time::Instant::now()
                .duration_since(start)
                .as_secs_f64()
        );

        Ok(())
    }
}
