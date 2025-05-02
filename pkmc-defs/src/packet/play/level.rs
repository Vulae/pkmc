use std::{collections::HashMap, io::Write};

use pkmc_generated::{block::Block, registry::BlockEntityType};
use pkmc_util::{
    connection::{
        paletted_container::to_paletted_data_singular, ClientboundPacket, ConnectionError,
        PacketEncoder as _,
    },
    nbt::NBT,
    BitSet, PackedArray, Position, Transmutable,
};

#[derive(Debug)]
pub struct SetChunkCacheCenter {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for SetChunkCacheCenter {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.chunk_x)?;
        writer.encode(self.chunk_z)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetChunkChacheRadius(pub i32);

impl ClientboundPacket for SetChunkChacheRadius {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0)?;
        Ok(())
    }
}

pub struct LevelChunkHeightmaps {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LevelChunkHeightmapType {
    WorldSurfaceWorldgen,
    WorldSurface,
    OceanFloorWorldgen,
    OceanFloor,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl LevelChunkHeightmapType {
    fn to_id(self) -> i32 {
        match self {
            LevelChunkHeightmapType::WorldSurfaceWorldgen => 0,
            LevelChunkHeightmapType::WorldSurface => 1,
            LevelChunkHeightmapType::OceanFloorWorldgen => 2,
            LevelChunkHeightmapType::OceanFloor => 3,
            LevelChunkHeightmapType::MotionBlocking => 4,
            LevelChunkHeightmapType::MotionBlockingNoLeaves => 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LevelChunkHeightmap {
    world_height: u16,
    packed: PackedArray<Vec<u64>>,
}

impl LevelChunkHeightmap {
    pub fn new(world_height: u16) -> Self {
        Self {
            world_height,
            packed: PackedArray::new(PackedArray::bits_per_entry(world_height as u64), 256),
        }
    }

    pub fn set_height(&mut self, x: u8, z: u8, height: u16) {
        assert!(x < 16);
        assert!(z < 16);
        assert!(height < self.world_height);
        self.packed
            .set((x as usize) * 16 + (z as usize), height as u64);
    }
}

#[derive(Debug)]
pub struct BlockEntity {
    pub x: u8,
    pub z: u8,
    pub y: i16,
    pub r#type: BlockEntityType,
    pub data: NBT,
}

#[derive(Debug)]
pub struct LevelChunkData {
    // TODO: Is this entirely correct?
    pub heightmaps: HashMap<LevelChunkHeightmapType, LevelChunkHeightmap>,
    pub data: Box<[u8]>,
    pub block_entities: Vec<BlockEntity>,
}

impl LevelChunkData {
    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // TODO: Heightmaps
        writer.encode(self.heightmaps.len() as i32)?;
        for (r#type, heightmap) in self.heightmaps.iter() {
            writer.encode(r#type.to_id())?;
            let encoded = heightmap.packed.inner();
            writer.encode(encoded.len() as i32)?;
            encoded
                .iter()
                .try_for_each(|v| writer.write_all(&v.to_be_bytes()))?;
        }
        writer.encode(self.data.len() as i32)?;
        writer.write_all(&self.data)?;
        writer.encode(self.block_entities.len() as i32)?;
        for block_entity in self.block_entities.iter() {
            debug_assert!(block_entity.x <= 15);
            debug_assert!(block_entity.z <= 15);
            writer.write_all(&((block_entity.x << 4) | block_entity.z).to_be_bytes())?;
            writer.write_all(&block_entity.y.to_be_bytes())?;
            writer.encode(block_entity.r#type.to_id())?;
            writer.encode(&block_entity.data)?;
        }
        //println!("{:#?}", self.block_entities);
        Ok(())
    }
}

#[derive(Debug)]
pub struct LevelLightData {
    pub num_sections: usize,
    pub sky_lights_arrays: Box<[Option<[u8; 2048]>]>,
    pub block_lights_arrays: Box<[Option<[u8; 2048]>]>,
}

impl LevelLightData {
    pub fn full_dark(num_sections: usize) -> Self {
        Self {
            num_sections,
            sky_lights_arrays: vec![None; num_sections + 2].into_boxed_slice(),
            block_lights_arrays: vec![None; num_sections + 2].into_boxed_slice(),
        }
    }

    pub fn full_bright(num_sections: usize) -> Self {
        Self {
            num_sections,
            sky_lights_arrays: vec![Some([0xFF; 2048]); num_sections + 2].into_boxed_slice(),
            block_lights_arrays: vec![Some([0xFF; 2048]); num_sections + 2].into_boxed_slice(),
        }
    }

    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        assert_eq!(self.sky_lights_arrays.len(), self.num_sections + 2);
        assert_eq!(self.block_lights_arrays.len(), self.num_sections + 2);

        let mut sky_light_bitset = BitSet::new(self.num_sections + 2);
        self.sky_lights_arrays
            .iter()
            .enumerate()
            .for_each(|(i, a)| sky_light_bitset.set(i, a.is_some()));
        writer.encode(&sky_light_bitset)?;

        let mut block_light_bitset = BitSet::new(self.num_sections + 2);
        self.block_lights_arrays
            .iter()
            .enumerate()
            .for_each(|(i, a)| block_light_bitset.set(i, a.is_some()));
        writer.encode(&block_light_bitset)?;

        // ?????
        writer.encode(0)?;
        writer.encode(0)?;

        writer.encode(self.sky_lights_arrays.iter().flatten().count() as i32)?;
        for sky_light_array in self.sky_lights_arrays.iter().flatten() {
            writer.encode(2048)?;
            writer.write_all(sky_light_array)?;
        }

        writer.encode(self.block_lights_arrays.iter().flatten().count() as i32)?;
        for block_light_array in self.block_lights_arrays.iter().flatten() {
            writer.encode(2048)?;
            writer.write_all(block_light_array)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub chunk_data: LevelChunkData,
    pub light_data: LevelLightData,
}

impl LevelChunkWithLight {
    pub fn generate_test(chunk_x: i32, chunk_z: i32, num_sections: usize) -> std::io::Result<Self> {
        Ok(Self {
            chunk_x,
            chunk_z,
            chunk_data: LevelChunkData {
                heightmaps: HashMap::new(),
                data: {
                    let mut writer = Vec::new();

                    for i in 0..num_sections {
                        let block = match i {
                            0 => {
                                if (chunk_x + chunk_z) % 2 == 0 {
                                    Block::PinkConcrete
                                } else {
                                    Block::LightBlueConcrete
                                }
                            }
                            _ => Block::Air,
                        };
                        // Num non-air blocks
                        writer.write_all(&(!block.is_air() as i16 * 4096).to_be_bytes())?;

                        // Blocks
                        writer.write_all(&to_paletted_data_singular(block.into_id())?)?;
                        // Biome
                        writer.write_all(&to_paletted_data_singular(0)?)?;
                    }

                    writer.into_boxed_slice()
                },
                block_entities: Vec::new(),
            },
            light_data: LevelLightData::full_dark(num_sections),
        })
    }
}

impl ClientboundPacket for LevelChunkWithLight {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        self.chunk_data.write(&mut writer)?;
        self.light_data.write(&mut writer)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ForgetLevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for ForgetLevelChunk {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_FORGET_LEVEL_CHUNK;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BlockUpdate {
    pub location: Position,
    pub block: Block,
}

impl ClientboundPacket for BlockUpdate {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_BLOCK_UPDATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.location)?;
        writer.encode(self.block.into_id())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BlockEntityData {
    pub location: Position,
    pub r#type: BlockEntityType,
    pub data: NBT,
}

impl ClientboundPacket for BlockEntityData {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_BLOCK_ENTITY_DATA;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.location)?;
        writer.encode(self.r#type.to_id())?;
        writer.encode(&self.data)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateSectionBlocks {
    pub section: Position,
    pub blocks: Vec<(u8, u8, u8, Block)>,
}

impl ClientboundPacket for UpdateSectionBlocks {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        let v: u64 = Transmutable::<u64>::transmute((self.section.x as i64) << 42)
            | (Transmutable::<u64>::transmute((self.section.y as i64) << 44) >> 44)
            | (Transmutable::<u64>::transmute((self.section.z as i64) << 42) >> 22);
        writer.write_all(&v.to_be_bytes())?;

        writer.encode(self.blocks.len() as i32)?;
        for (bx, by, bz, id) in self.blocks.iter() {
            debug_assert!(*bx <= 15);
            debug_assert!(*by <= 15);
            debug_assert!(*bz <= 15);
            let encoded_position: u64 = ((*bx as u64) << 8) | ((*bz as u64) << 4) | (*by as u64);
            writer.encode(
                ((id.into_id() as i64) << 12) | Transmutable::<i64>::transmute(encoded_position),
            )?;
        }
        Ok(())
    }
}
