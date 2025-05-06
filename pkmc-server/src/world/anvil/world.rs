use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    hash::Hash,
    io::{Seek as _, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use itertools::Itertools as _;
use pkmc_defs::{biome::Biome, packet};
use pkmc_generated::{
    block::Block,
    consts::{
        PALETTED_DATA_BIOMES_DIRECT, PALETTED_DATA_BIOMES_INDIRECT, PALETTED_DATA_BLOCKS_DIRECT,
        PALETTED_DATA_BLOCKS_INDIRECT,
    },
    registry::BlockEntityType,
};
use pkmc_util::{
    connection::{
        paletted_container::{calculate_bpe, to_paletted_data, to_paletted_data_precomputed},
        ConnectionSender,
    },
    nbt::{from_nbt, NBT},
    IdTable, PackedArray, Position, ReadExt as _, Transmutable as _, Vec3, WeakList,
};

use crate::world::{
    chunk_loader::{ChunkLoader, ChunkPosition},
    section_index_block_pos, section_pos_block_index, World, WorldViewer, CHUNK_SIZE,
    SECTION_BIOMES, SECTION_BLOCKS, SECTION_BLOCKS_SIZE,
};

use super::{chunk_format, AnvilError};

pub const REGION_SIZE: usize = 32;
pub const CHUNKS_PER_REGION: usize = REGION_SIZE * REGION_SIZE;

// Each time the world updates & sends new data to client, we either send sections or chunks.
// Note that when sending sections, the client calculates lighting instead of server.
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_SECTIONS: usize = 4;
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_BLOCKS: usize = 1024;

#[derive(Debug)]
struct PalettedData<T: Debug, const N: usize, const I_S: u8, const I_E: u8> {
    palette: Box<[T]>,
    data: Box<[i64]>,
}

impl<T: Debug + Default, const N: usize, const I_S: u8, const I_E: u8> Default
    for PalettedData<T, N, I_S, I_E>
{
    fn default() -> Self {
        Self {
            palette: vec![T::default()].into_boxed_slice(),
            data: vec![].into_boxed_slice(),
        }
    }
}

impl<T: Debug, const N: usize, const I_S: u8, const I_E: u8> PalettedData<T, N, I_S, I_E> {
    fn bpe(palette_count: usize) -> u8 {
        match palette_count {
            0 => panic!(),
            1 => 0,
            // Data stored inside the world files doesn't have direct paletting.
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

impl<T: Debug + Clone + Eq + Hash, const N: usize, const I_S: u8, const I_E: u8>
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
    fn write(&self, mut writer: impl Write) -> Result<(), AnvilError> {
        // TODO: Special case for if there's only 1 block state.

        let block_ids = self
            .palette
            .iter()
            .map(|b| b.into_id())
            .collect::<Box<[i32]>>();

        let block_count = (0..SECTION_BLOCKS)
            .filter(|i| !self.palette[self.palette_index(*i)].is_air())
            .count();

        writer.write_all(&(block_count as u16).to_be_bytes())?;

        const FORCE_SECTION_REENCODE: bool = false;

        if !FORCE_SECTION_REENCODE
            // Data stored in the anvil format doesn't have direct paletting.
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
            .map(|b| b.id(mapper).unwrap_or_default())
            .collect::<Box<[i32]>>();

        const FORCE_SECTION_REENCODE: bool = false;

        if !FORCE_SECTION_REENCODE
            // Data stored in the anvil format doesn't have direct paletting.
            // So we need to re-encode the data if there's too many palette values.
            && calculate_bpe(biome_ids.len()) <= PALETTED_DATA_BIOMES_INDIRECT_END
        {
            writer.write_all(&to_paletted_data_precomputed(
                &biome_ids,
                &self.data,
                PALETTED_DATA_BIOMES_INDIRECT,
                PALETTED_DATA_BIOMES_DIRECT,
            )?)?;
        } else {
            writer.write_all(&to_paletted_data(
                &(0..SECTION_BIOMES)
                    .map(|i| biome_ids[self.palette_index(i)])
                    .collect::<Box<[_]>>(),
                PALETTED_DATA_BIOMES_INDIRECT,
                PALETTED_DATA_BIOMES_DIRECT,
            )?)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
struct ChunkSection {
    blocks: ChunkSectionBlockStates,
    biomes: ChunkSectionBiomes,
}

#[derive(Debug)]
pub struct AnvilBlockEntity {
    // Set every time this block entity is queried.
    r#type: BlockEntityType,
    pub components: HashMap<String, NBT>,
    pub data: NBT,
}

impl AnvilBlockEntity {
    fn new(r#type: BlockEntityType) -> Self {
        AnvilBlockEntity {
            r#type,
            components: HashMap::new(),
            data: NBT::Compound(HashMap::new()),
        }
    }

    pub fn r#type(&self) -> BlockEntityType {
        self.r#type
    }
}

#[derive(Debug)]
struct Chunk {
    chunk_x: i32,
    chunk_z: i32,
    sections_y_start: i8,
    sections: Box<[ChunkSection]>,
    block_entities: HashMap<(u8, i16, u8), AnvilBlockEntity>,
}

impl Chunk {
    fn new(
        parsed: chunk_format::Chunk,
        section_y_range: std::ops::RangeInclusive<i8>,
    ) -> Result<Self, AnvilError> {
        if let Some(y_pos) = parsed.y_pos {
            assert_eq!(*section_y_range.start(), y_pos);
        }

        Ok(Chunk {
            chunk_x: parsed.x_pos,
            chunk_z: parsed.z_pos,
            sections_y_start: *section_y_range.start(),
            sections: section_y_range
                .map(|section_y| {
                    let Some(section) = parsed
                        .sections
                        .iter()
                        .find(|section| section.y == section_y)
                    else {
                        return ChunkSection::default();
                    };
                    ChunkSection {
                        blocks: section
                            .block_states
                            .as_ref()
                            .map(|i| PalettedData {
                                palette: i
                                    .palette
                                    .iter()
                                    .map(|v| {
                                        v.to_block().unwrap_or_else(|| {
                                            println!(
                                                "Invalid block in chunk {} {}: {:#?}",
                                                parsed.x_pos, parsed.z_pos, v,
                                            );
                                            Block::Air
                                        })
                                    })
                                    .collect(),
                                data: i.data.clone(),
                            })
                            .unwrap_or_default(),
                        biomes: section
                            .biomes
                            .as_ref()
                            .map(|i| PalettedData {
                                palette: i.palette.clone(),
                                data: i.data.clone(),
                            })
                            .unwrap_or_default(),
                    }
                })
                .collect(),
            block_entities: parsed
                .block_entities
                .into_iter()
                .map(|b| {
                    Ok((
                        (
                            b.x.rem_euclid(CHUNK_SIZE as i32) as u8,
                            b.y,
                            b.z.rem_euclid(CHUNK_SIZE as i32) as u8,
                        ),
                        AnvilBlockEntity {
                            r#type: BlockEntityType::from_str(&b.id)
                                .ok_or(AnvilError::InvalidBlockEntityType(b.id))?,
                            components: b.components,
                            data: NBT::Compound(b.data),
                        },
                    ))
                })
                .collect::<Result<_, AnvilError>>()?,
        })
    }

    fn get_block(&self, x: u8, y: i16, z: u8) -> Option<Block> {
        debug_assert!((x as usize) < SECTION_BLOCKS_SIZE);
        debug_assert!((z as usize) < SECTION_BLOCKS_SIZE);
        let section = self.sections.get(
            (y.div_euclid(SECTION_BLOCKS_SIZE as i16) - (self.sections_y_start as i16)) as usize,
        )?;
        Some(*section.blocks.get(section_pos_block_index(
            x,
            y.rem_euclid(SECTION_BLOCKS_SIZE as i16) as u8,
            z,
        )))
    }

    fn set_block(&mut self, x: u8, y: i16, z: u8, block: Block) -> bool {
        debug_assert!((x as usize) < SECTION_BLOCKS_SIZE);
        debug_assert!((z as usize) < SECTION_BLOCKS_SIZE);
        let Some(section) = self.sections.get_mut(
            (y.div_euclid(SECTION_BLOCKS_SIZE as i16) - (self.sections_y_start as i16)) as usize,
        ) else {
            return false;
        };
        if section.blocks.set(
            section_pos_block_index(x, y.rem_euclid(SECTION_BLOCKS_SIZE as i16) as u8, z),
            block,
        ) {
            self.block_entities.remove(&(x, y, z));
            true
        } else {
            false
        }
    }

    fn query_block_entity(&mut self, x: u8, y: i16, z: u8) -> Option<&mut AnvilBlockEntity> {
        let block = self.get_block(x, y, z)?;
        if let Some(block_entity_type) = block.block_entity_type() {
            Some(
                self.block_entities
                    .entry((x, y, z))
                    .or_insert_with(|| AnvilBlockEntity::new(block_entity_type)),
            )
        } else {
            self.block_entities.remove(&(x, y, z));
            None
        }
    }

    fn to_packet(
        &self,
        biome_mapper: &IdTable<Biome>,
    ) -> Result<packet::play::LevelChunkWithLight, AnvilError> {
        Ok(packet::play::LevelChunkWithLight {
            chunk_x: self.chunk_x,
            chunk_z: self.chunk_z,
            chunk_data: packet::play::LevelChunkData {
                heightmaps: HashMap::new(),
                data: {
                    let mut writer = Vec::new();

                    self.sections.iter().try_for_each(|section| {
                        section.blocks.write(&mut writer)?;
                        section.biomes.write(&mut writer, biome_mapper)?;
                        Ok::<_, AnvilError>(())
                    })?;

                    writer.into_boxed_slice()
                },
                block_entities: self
                    .block_entities
                    .iter()
                    .flat_map(|((x, y, z), block_entity)| {
                        block_entity
                            .r#type()
                            .nbt_visible()
                            .then(|| packet::play::BlockEntity {
                                x: *x,
                                y: *y,
                                z: *z,
                                r#type: block_entity.r#type(),
                                data: &block_entity.data,
                            })
                    })
                    .collect(),
            },
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

    fn load_chunk(
        &mut self,
        chunk_x: u8,
        chunk_z: u8,
        section_y_range: std::ops::RangeInclusive<i8>,
    ) -> Result<Option<Chunk>, AnvilError> {
        self.read_nbt(chunk_x, chunk_z)?
            .map(|(_, nbt)| from_nbt::<chunk_format::Chunk>(nbt))
            .transpose()?
            .map(|parsed| Chunk::new(parsed, section_y_range))
            .transpose()
    }
}

#[derive(Debug)]
struct SectionDiff {
    change: [Option<Block>; SECTION_BLOCKS],
}

impl Default for SectionDiff {
    fn default() -> Self {
        Self {
            change: [None; SECTION_BLOCKS],
        }
    }
}

impl SectionDiff {
    fn set(&mut self, x: u8, y: u8, z: u8, block: Block) {
        self.change[section_pos_block_index(x, y, z)] = Some(block);
    }

    fn num_blocks(&self) -> usize {
        self.change.iter().flatten().count()
    }

    fn into_packet_data(self) -> Vec<(u8, u8, u8, Block)> {
        self.change
            .into_iter()
            .enumerate()
            .flat_map(|(i, b)| Some((section_index_block_pos(i), b?)))
            .map(|((x, y, z), block)| (x, y, z, block))
            .collect()
    }
}

#[derive(Debug)]
pub struct AnvilWorld {
    root: PathBuf,
    identifier: String,
    regions: HashMap<(i32, i32), Option<Region>>,
    chunks: HashMap<(i32, i32), Option<Chunk>>,
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
            regions: HashMap::new(),
            chunks: HashMap::new(),
            section_y_range,
            biome_mapper,
            viewers: WeakList::new(),
            diffs: HashMap::new(),
        }
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    fn section_y_range(&self) -> std::ops::RangeInclusive<i8> {
        self.section_y_range.clone()
    }

    fn prepare_region(&mut self, region_x: i32, region_z: i32) -> Result<(), AnvilError> {
        if self.regions.contains_key(&(region_x, region_z)) {
            return Ok(());
        }
        self.regions.insert((region_x, region_z), None);

        let mut path = self.root.clone();
        path.push("region");
        path.push(format!("r.{}.{}.mca", region_x, region_z));

        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(());
            }
            result => result,
        }?;

        self.regions.insert(
            (region_x, region_z),
            Some(Region::load(file, region_x, region_z)?),
        );

        Ok(())
    }

    fn prepare_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<(), AnvilError> {
        if self.chunks.contains_key(&(chunk_x, chunk_z)) {
            return Ok(());
        }
        self.chunks.insert((chunk_x, chunk_z), None);

        let region_x = chunk_x.div_euclid(REGION_SIZE as i32);
        let region_z = chunk_z.div_euclid(REGION_SIZE as i32);

        self.prepare_region(region_x, region_z)?;

        let range = self.section_y_range();

        let Some(Some(region)) = self.regions.get_mut(&(region_x, region_z)) else {
            return Ok(());
        };

        if let Some(chunk) = region.load_chunk(
            chunk_x.rem_euclid(REGION_SIZE as i32) as u8,
            chunk_z.rem_euclid(REGION_SIZE as i32) as u8,
            range,
        )? {
            self.chunks.insert((chunk_x, chunk_z), Some(chunk));
        }

        Ok(())
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
                            blocks: diff.into_packet_data(),
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
                if let Some(Some(chunk)) = self.chunks.get(&(to_load.chunk_x, to_load.chunk_z)) {
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

    fn get_block(&mut self, position: Position) -> Result<Option<Block>, Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.prepare_chunk(chunk_x, chunk_z)?;
        let Some(Some(chunk)) = self.chunks.get(&(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        )) else {
            return Ok(None);
        };
        Ok(chunk.get_block(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
        ))
    }

    fn set_block(&mut self, position: Position, block: Block) -> Result<(), Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.prepare_chunk(chunk_x, chunk_z)?;
        let Some(Some(chunk)) = self.chunks.get_mut(&(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        )) else {
            return Ok(());
        };
        if chunk.set_block(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
            block,
        ) {
            self.diffs
                .entry((
                    position.x.div_euclid(SECTION_BLOCKS_SIZE as i32),
                    position.z.div_euclid(SECTION_BLOCKS_SIZE as i32),
                ))
                .or_default()
                .entry(position.y.div_euclid(SECTION_BLOCKS_SIZE as i16))
                .or_default()
                .set(
                    position.x.rem_euclid(SECTION_BLOCKS_SIZE as i32) as u8,
                    position.y.rem_euclid(SECTION_BLOCKS_SIZE as i16) as u8,
                    position.z.rem_euclid(SECTION_BLOCKS_SIZE as i32) as u8,
                    block,
                );
        }
        Ok(())
    }

    type BlockData = AnvilBlockEntity;
    fn query_block_data(
        &mut self,
        position: Position,
    ) -> Result<Option<&mut AnvilBlockEntity>, Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.prepare_chunk(chunk_x, chunk_z)?;
        let Some(Some(chunk)) = self.chunks.get_mut(&(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        )) else {
            return Ok(None);
        };
        Ok(chunk.query_block_entity(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
        ))
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
        // 1.21.5 debug world
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

            let Some(block) = world.get_block(position)? else {
                panic!("Expected loaded block at {:?}", position);
            };

            if block.into_id() != block_id {
                panic!(
                    "Block at {:?} is {:?} with ID {:?}, but our ID is {}",
                    position,
                    block,
                    block.into_id(),
                    block_id,
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
