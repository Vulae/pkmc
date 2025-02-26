mod connection;
pub mod handler;
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

    pub fn from_inner(length: usize, inner: Box<[u64]>) -> Self {
        assert!(inner.len() == length.div_ceil(64));
        Self {
            length,
            data: inner,
        }
    }

    pub fn into_inner(self) -> Box<[u64]> {
        self.data
    }

    pub fn inner(&self) -> &[u64] {
        &self.data
    }

    pub fn num_bits(&self) -> usize {
        self.length
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        (index < self.length).then(|| (self.data[index >> 6] & (1 << (index & 0b111111))) != 0)
    }

    pub fn set(&mut self, index: usize, set: bool) {
        if index >= self.length {
            return;
        }
        if set {
            self.data[index >> 6] |= 1 << (index & 0b111111);
        } else {
            self.data[index >> 6] &= !(1 << (index & 0b111111));
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FixedBitSet<const N: usize> {
    data: Box<[u8]>,
}

impl<const N: usize> Default for FixedBitSet<N> {
    fn default() -> Self {
        Self {
            data: vec![0u8; N.div_ceil(8)].into_boxed_slice(),
        }
    }
}

impl<const N: usize> FixedBitSet<N> {
    pub fn from_inner(inner: Box<[u8]>) -> Self {
        assert!(inner.len() == N.div_ceil(8));
        Self { data: inner }
    }

    pub fn into_inner(self) -> Box<[u8]> {
        self.data
    }

    pub fn inner(&self) -> &[u8] {
        &self.data
    }

    pub const fn num_bits() -> usize {
        N
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        (index < N).then(|| (self.data[index >> 3] & (1 << (index & 0b111))) != 0)
    }

    pub fn set(&mut self, index: usize, set: bool) {
        if index >= N {
            return;
        }
        if set {
            self.data[index >> 3] |= 1 << (index & 0b111);
        } else {
            self.data[index >> 3] &= !(1 << index & 0b111);
        }
    }
}
