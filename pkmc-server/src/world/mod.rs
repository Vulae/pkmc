use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use chunk_loader::ChunkLoader;
use pkmc_defs::packet;
use pkmc_generated::block::Block;
use pkmc_util::{
    Position, Vec3,
    connection::{ConnectionError, ConnectionSender},
};

pub mod anvil;
pub mod chunk_loader;

pub const CHUNK_SIZE: usize = 16;
pub const SECTION_SIZE: usize = 16;
pub const SECTION_BLOCKS: usize = 4096;
pub const SECTION_BIOMES_SIZE: usize = 4;
pub const SECTION_BIOMES: usize = 64;

#[derive(Debug)]
pub struct WorldViewer {
    connection: ConnectionSender,
    pub loader: ChunkLoader,
    pub position: Vec3<f64>,
}

impl WorldViewer {
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

pub trait World: Debug {
    type Error: std::error::Error;

    fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<WorldViewer>>;
    fn update_viewers(&mut self) -> Result<(), Self::Error>;

    fn get_block(&mut self, position: Position) -> Result<Option<Block>, Self::Error>;
    fn set_block(&mut self, position: Position, block: Block) -> Result<(), Self::Error>;
}
