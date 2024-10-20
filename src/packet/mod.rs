pub mod login;
pub mod play;
pub mod reader;
pub mod server_list;
pub mod writer;

use std::{collections::HashMap, hash::Hash};

use anyhow::Result;
use writer::PacketWriter;

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

pub trait Paletteable: Hash + Eq {
    // TODO: Should this be result?
    fn palette_value(&self) -> Result<i32>;
}

pub fn to_paletted_container<T: Paletteable>(values: &[T], direct_bpe: u8) -> Result<Box<[u8]>> {
    let mut palette = HashMap::new();
    values.iter().fold(0, |index, value| {
        if palette.contains_key(value) {
            index
        } else {
            palette.insert(value, index);
            index + 1
        }
    });

    let mut writer = PacketWriter::new_empty();

    // ceil(log2(count))
    let bpe: u8 = match palette.len() {
        0 => panic!(),
        1 => 0,
        entry_count => (usize::BITS - entry_count.leading_zeros()).try_into()?,
    };

    if bpe == 0 {
        writer.write_unsigned_byte(0)?;
        writer.write_var_int(values[0].palette_value()?)?;
        writer.write_var_int(0)?;
    } else if bpe < direct_bpe {
        // Indirect (Every entry is index into values)
        unimplemented!()
    } else {
        // Direct (Every entry is from values, not indexed)
        unimplemented!()
    }

    Ok(writer.into_inner().into_boxed_slice())
}
