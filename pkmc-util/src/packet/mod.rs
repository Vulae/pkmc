mod connection;
mod packet;
mod paletted_container;
mod reader;
mod writer;

pub use connection::*;
pub use packet::*;
pub use paletted_container::*;
pub use reader::*;
pub use writer::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unsupported packet {0}: {1:#X}")]
    UnsupportedPacket(String, i32),
    #[error("Invalid raw packet ID for parser (expected: {0}, found: {1})")]
    InvalidRawPacketIDForParser(i32, i32),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct BitSet {
    length: usize,
    data: Box<[u64]>,
}

impl BitSet {
    pub fn new(length: usize) -> Self {
        Self {
            length,
            data: vec![0; length.div_ceil(64)].into_boxed_slice(),
        }
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.length {
            None
        } else {
            Some((self.data[index >> 6] & (1 << (index & 0b00111111))) != 0)
        }
    }

    pub fn set(&mut self, index: usize, set: bool) {
        if index < self.length {
            if set {
                self.data[index >> 6] |= 1 << (index & 0b00111111);
            } else {
                self.data[index >> 6] &= !(1 << (index & 0b00111111));
            }
        }
    }

    pub fn num_longs(&self) -> usize {
        self.data.len()
    }

    pub fn longs_iter(&self) -> impl Iterator<Item = u64> + use<'_> {
        self.data.iter().cloned()
    }

    pub fn into_inner(self) -> Box<[u64]> {
        self.data
    }
}
