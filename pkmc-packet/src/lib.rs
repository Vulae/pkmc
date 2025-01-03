pub mod connection;
pub mod packet;
pub mod paletted_container;
pub mod reader;
pub mod writer;

pub use connection::{Connection, ConnectionError};
pub use packet::{ClientboundPacket, RawPacket, ServerboundPacket};
pub use reader::ReadExtPacket;
pub use writer::WriteExtPacket;

pub use paletted_container::to_paletted_data;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BitSet {
    data: Box<[u64]>,
}

impl BitSet {
    pub fn new(num_bits: usize) -> Self {
        Self {
            data: vec![0; (num_bits + 63) / 64].into_boxed_slice(),
        }
    }

    pub fn get(&self, index: usize) -> bool {
        (self.data[index >> 6] & (1 << (index & 0b00111111))) != 0
    }

    pub fn set(&mut self, index: usize, set: bool) {
        if set {
            self.data[index >> 6] |= 1 << (index & 0b00111111);
        } else {
            self.data[index >> 6] &= !(1 << (index & 0b00111111));
        }
    }

    pub fn num_longs(&self) -> usize {
        self.data.len()
    }

    pub fn longs_iter(&self) -> impl Iterator<Item = &u64> {
        self.data.iter()
    }
}
