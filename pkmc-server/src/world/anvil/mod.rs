use pkmc_util::{connection::ConnectionError, nbt::NBTError};
use thiserror::Error;

mod chunk_format;
mod world;

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
    #[error("Invalid block entity type \"{0}\"")]
    InvalidBlockEntityType(String),
}

pub use world::*;
