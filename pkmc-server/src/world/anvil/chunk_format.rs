#![allow(unused)]

use pkmc_defs::{biome::Biome, block::DynamicBlock};
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug};

#[derive(Deserialize)]
pub struct PalettedData<T: Debug> {
    #[serde(default)]
    pub palette: Box<[T]>,
    #[serde(default)]
    pub data: Box<[i64]>,
}

#[derive(Deserialize)]
pub struct ChunkSection {
    #[serde(rename = "Y")]
    pub y: i8,
    pub block_states: Option<PalettedData<DynamicBlock>>,
    pub biomes: Option<PalettedData<Biome>>,
}

#[derive(Deserialize)]
pub struct BlockEntity {
    pub id: String,
    #[serde(rename = "keepPacked", default)]
    pub keep_packed: bool,
    pub x: i32,
    pub y: i16,
    pub z: i32,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Chunk {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: Option<i8>,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    pub sections: Vec<ChunkSection>,
    pub block_entities: Vec<BlockEntity>,
}
