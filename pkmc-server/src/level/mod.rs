use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use chunk_loader::ChunkLoader;
use pkmc_defs::packet;
use pkmc_generated::block::Block;
use pkmc_util::{
    connection::{ConnectionError, ConnectionSender},
    Position, Vec3,
};

pub mod anvil;
pub mod chunk_loader;

pub const CHUNK_SIZE: usize = 16;
pub const SECTION_BLOCKS_SIZE: usize = 16;
pub const SECTION_BLOCKS: usize = 4096;
pub const SECTION_BIOMES_SIZE: usize = 4;
pub const SECTION_BIOMES: usize = 64;

pub const fn section_pos_block_index(x: u8, y: u8, z: u8) -> usize {
    debug_assert!((x as usize) < SECTION_BLOCKS_SIZE);
    debug_assert!((y as usize) < SECTION_BLOCKS_SIZE);
    debug_assert!((z as usize) < SECTION_BLOCKS_SIZE);
    (y as usize) * SECTION_BLOCKS_SIZE * SECTION_BLOCKS_SIZE
        + (z as usize) * SECTION_BLOCKS_SIZE
        + (x as usize)
}

pub const fn section_index_block_pos(i: usize) -> (u8, u8, u8) {
    debug_assert!(i < SECTION_BLOCKS_SIZE);
    (
        (i % SECTION_BLOCKS_SIZE) as u8,
        ((i / (SECTION_BLOCKS_SIZE * SECTION_BLOCKS_SIZE)) % SECTION_BLOCKS_SIZE) as u8,
        ((i / SECTION_BLOCKS_SIZE) % SECTION_BLOCKS_SIZE) as u8,
    )
}

pub const fn section_pos_biome_index(x: u8, y: u8, z: u8) -> usize {
    debug_assert!((x as usize) < SECTION_BIOMES_SIZE);
    debug_assert!((y as usize) < SECTION_BIOMES_SIZE);
    debug_assert!((z as usize) < SECTION_BIOMES_SIZE);
    (y as usize) * SECTION_BIOMES_SIZE * SECTION_BIOMES_SIZE
        + (z as usize) * SECTION_BIOMES_SIZE
        + (x as usize)
}

pub const fn section_index_biome_pos(i: usize) -> (u8, u8, u8) {
    debug_assert!(i < SECTION_BIOMES_SIZE);
    (
        (i % SECTION_BIOMES_SIZE) as u8,
        ((i / (SECTION_BIOMES_SIZE * SECTION_BIOMES_SIZE)) % SECTION_BIOMES_SIZE) as u8,
        ((i / SECTION_BIOMES_SIZE) % SECTION_BIOMES_SIZE) as u8,
    )
}

#[derive(Debug)]
pub struct LevelViewer {
    connection: ConnectionSender,
    pub loader: ChunkLoader,
    pub position: Vec3<f64>,
}

impl LevelViewer {
    pub fn fake(connection: ConnectionSender) -> Self {
        Self {
            connection,
            loader: ChunkLoader::new(0),
            position: Vec3::zero(),
        }
    }

    pub fn connection(&self) -> &ConnectionSender {
        &self.connection
    }

    pub fn unload_all_chunks(&mut self) -> Result<(), ConnectionError> {
        for chunk in self.loader.unload_all() {
            self.connection().send(&packet::play::ForgetLevelChunk {
                chunk_x: chunk.chunk_x,
                chunk_z: chunk.chunk_z,
            })?;
        }
        Ok(())
    }
}

pub trait Level: Debug {
    type Error: std::error::Error;

    fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<LevelViewer>>;
    fn update_viewers(&mut self) -> Result<(), Self::Error>;

    fn get_block(&mut self, position: Position) -> Result<Option<Block>, Self::Error>;
    fn set_block(&mut self, position: Position, block: Block) -> Result<(), Self::Error>;

    type BlockData: std::fmt::Debug;
    fn query_block_data(
        &mut self,
        position: Position,
    ) -> Result<Option<&mut Self::BlockData>, Self::Error>;
}
