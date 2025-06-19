use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use itertools::Itertools as _;
use pkmc_defs::{biome::Biome, dimension::Dimension, packet};
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
        paletted_container::{
            calculate_bpe, to_paletted_data, to_paletted_data_precomputed,
            to_paletted_data_singular,
        },
        ConnectionSender,
    },
    nbt::{self, NBT},
    IdTable, PackedArray, Position, Transmutable as _, Vec3, WeakList,
};

use crate::level::{
    chunk_loader::{ChunkLoader, ChunkPosition},
    section_index_block_pos, section_pos_block_index, Level, LevelViewer, CHUNK_SIZE,
    SECTION_BIOMES, SECTION_BLOCKS, SECTION_BLOCKS_SIZE,
};

use super::{
    chunk_format,
    region::{ChunkParser, ChunkReader},
    AnvilError,
};

// Each time the world updates & sends new data to client, we either send sections or chunks.
// Note that when sending sections, the client calculates lighting instead of server.
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_SECTIONS: usize = 4;
pub const UPDATE_SECTION_CHUNK_SWITCH_NUM_BLOCKS: usize = 1024;

const FORCE_SECTION_REENCODE: bool = false;

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

        // Data is already in the palette, so we just set the palette index.
        if let Some(palette_index) = self.palette.iter().position(|v| *v == value) {
            let bpe = Self::bpe(self.palette.len());
            PackedArray::from_inner(self.data.as_mut().transmute(), bpe, N)
                .set(index, palette_index as u64);
            return true;
        }

        // Data isn't already in the palette, so we need to rebuild palette & data.
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
        match self.palette.as_ref() {
            [] => panic!("Cannot write empty paletted data"),
            [block] => {
                writer.write_all(&(if block.is_air() { 0u16 } else { 4096u16 }).to_be_bytes())?;
                writer.write_all(&to_paletted_data_singular(block.into_id())?)?;
                return Ok(());
            }
            [..] => {}
        }

        let block_ids = self
            .palette
            .iter()
            .map(|b| b.into_id())
            .collect::<Box<[i32]>>();

        let block_count = (0..SECTION_BLOCKS)
            .filter(|i| !self.palette[self.palette_index(*i)].is_air())
            .count();

        writer.write_all(&(block_count as u16).to_be_bytes())?;

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
        match self.palette.as_ref() {
            [] => panic!("Cannot write empty paletted data"),
            [biome] => {
                writer.write_all(&to_paletted_data_singular(
                    biome.id(mapper).unwrap_or_default(),
                )?)?;
                return Ok(());
            }
            [..] => {}
        }

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
                    if b.id == "DUMMY" {
                        return Ok(None);
                    }
                    Ok(Some((
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
                    )))
                })
                .collect::<Result<Vec<_>, AnvilError>>()?
                .into_iter()
                .flatten()
                .collect(),
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
struct AnvilChunkParser {
    section_y_range: std::ops::RangeInclusive<i8>,
}

impl ChunkParser for AnvilChunkParser {
    type Chunk = Chunk;
    fn parse(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        nbt: (String, NBT),
    ) -> Result<Option<Self::Chunk>, AnvilError> {
        debug_assert!(nbt.0.is_empty());
        let deserialized_chunk: chunk_format::Chunk = nbt::from_nbt(nbt.1)?;
        if !matches!(
            deserialized_chunk.status.as_ref(),
            // All the commented don't have full world generation (As in blocks, not entities or lighting).
            // minecraft:empty is kept because when upgrading a world all chunks are set to empty even though they may not be.
            "minecraft:empty"
            //    | "minecraft:structures_starts"
            //    | "minecraft:structures_references"
            //    | "minecraft:biomes"
            //    | "minecraft:noise"
            //    | "minecraft:surface"
            //    | "minecraft:carvers"
            //    | "minecraft:features"
            | "minecraft:initialize_light" | "minecraft:light" | "minecraft:spawn" | "minecraft:full"
        ) {
            return Ok(None);
        }
        let parsed_chunk = Chunk::new(deserialized_chunk, self.section_y_range.clone())?;
        assert_eq!(parsed_chunk.chunk_x, chunk_x);
        assert_eq!(parsed_chunk.chunk_z, chunk_z);
        Ok(Some(parsed_chunk))
    }
}

#[derive(Debug)]
pub struct AnvilLevel {
    dimension: Dimension,
    loader: ChunkReader<AnvilChunkParser>,
    biome_mapper: IdTable<Biome>,
    section_y_range: std::ops::RangeInclusive<i8>,
    viewers: WeakList<Mutex<LevelViewer>>,
    diffs: HashMap<(i32, i32), HashMap<i16, SectionDiff>>,
}

impl AnvilLevel {
    pub fn new<P: Into<PathBuf>>(
        root: P,
        dimension: Dimension,
        section_y_range: std::ops::RangeInclusive<i8>,
        biome_mapper: IdTable<Biome>,
    ) -> Self {
        Self {
            dimension,
            loader: ChunkReader::new(
                root.into(),
                AnvilChunkParser {
                    section_y_range: section_y_range.clone(),
                },
            ),
            biome_mapper,
            section_y_range,
            viewers: WeakList::new(),
            diffs: HashMap::new(),
        }
    }

    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }
}

impl Level for AnvilLevel {
    type Error = AnvilError;

    fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<LevelViewer>> {
        let viewer = LevelViewer {
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
                self.loader
                    .prepare_chunk(to_load.chunk_x, to_load.chunk_z)?;
                if let Some(chunk) = self.loader.get_chunk(to_load.chunk_x, to_load.chunk_z) {
                    viewer
                        .connection()
                        .send(&chunk.to_packet(&self.biome_mapper)?)?;
                } else {
                    viewer
                        .connection()
                        .send(&packet::play::LevelChunkWithLight::generate_test(
                            to_load.chunk_x,
                            to_load.chunk_z,
                            self.section_y_range.len(),
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
        self.loader.prepare_chunk(chunk_x, chunk_z)?;
        let Some(chunk) = self.loader.get_chunk(
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

    fn set_block(&mut self, position: Position, block: Block) -> Result<(), Self::Error> {
        let chunk_x = position.x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = position.z.div_euclid(CHUNK_SIZE as i32);
        self.loader.prepare_chunk(chunk_x, chunk_z)?;
        let Some(chunk) = self.loader.get_mut_chunk(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        ) else {
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
        self.loader.prepare_chunk(chunk_x, chunk_z)?;
        let Some(chunk) = self.loader.get_mut_chunk(
            position.x.div_euclid(CHUNK_SIZE as i32),
            position.z.div_euclid(CHUNK_SIZE as i32),
        ) else {
            return Ok(None);
        };
        Ok(chunk.query_block_entity(
            (position.x.rem_euclid(CHUNK_SIZE as i32)) as u8,
            position.y,
            (position.z.rem_euclid(CHUNK_SIZE as i32)) as u8,
        ))
    }
}
