use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use pkmc_defs::block::{Block, BlockEntity};
use pkmc_util::{packet::ConnectionSender, Position};

pub mod anvil;
pub mod chunk_loader;

pub const CHUNK_SIZE: usize = 16;
pub const SECTION_SIZE: usize = 16;
pub const SECTION_BLOCKS: usize = 4096;
pub const SECTION_BIOMES_SIZE: usize = 4;
pub const SECTION_BIOMES: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum WorldBlock {
    Block(Block),
    BlockEntity(BlockEntity),
}

impl WorldBlock {
    pub fn as_block(&self) -> &Block {
        match self {
            WorldBlock::Block(ref block) => block,
            WorldBlock::BlockEntity(ref block_entity) => &block_entity.block,
        }
    }

    pub fn into_block(self) -> Block {
        match self {
            WorldBlock::Block(block) => block,
            WorldBlock::BlockEntity(block_entity) => block_entity.block,
        }
    }

    pub fn as_block_entity(&self) -> Option<&BlockEntity> {
        match self {
            WorldBlock::Block(..) => None,
            WorldBlock::BlockEntity(ref block_entity) => Some(block_entity),
        }
    }
}

pub trait World: Debug {
    type Error: std::error::Error;
    type Viewer;

    fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<Self::Viewer>>;

    fn get_block(&mut self, position: Position) -> Result<Option<WorldBlock>, Self::Error>;
    fn set_block(&mut self, position: Position, block: WorldBlock) -> Result<(), Self::Error>;

    fn update(&mut self) -> Result<(), Self::Error>;
}
