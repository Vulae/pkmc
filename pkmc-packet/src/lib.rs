pub mod connection;
pub mod reader;
pub mod writer;

pub use connection::Connection;
pub use reader::PacketReader;
pub use writer::PacketWriter;

use std::{collections::HashMap, hash::Hash};

use anyhow::Result;

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

pub fn to_paletted_container<T: Paletteable>(
    values: &[T],
    min_direct_bpe: u8,
    max_direct_bpe: u8,
) -> Result<Box<[u8]>> {
    let mut palette: HashMap<&T, usize> = HashMap::new();
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
        entry_count => {
            TryInto::<u8>::try_into(usize::BITS - entry_count.leading_zeros())?.max(min_direct_bpe)
        }
    };

    writer.write_unsigned_byte(bpe)?;

    if bpe == 0 {
        // Single valued (Every entry is same)
        writer.write_var_int(values[0].palette_value()?)?;
        writer.write_var_int(0)?; // Indices array is always empty on single valued
    } else if bpe <= max_direct_bpe {
        unimplemented!()
        // Indirect (Every entry is index into values)
        //writer.write_var_int(palette.len() as usize)?;
        //for value in values.iter() {}
    } else {
        // Direct (Every entry is from values, not indexed)
        for value in values.iter() {
            writer.write_var_int(value.palette_value()?)?;
        }
    }

    Ok(writer.into_inner().into_boxed_slice())
}
