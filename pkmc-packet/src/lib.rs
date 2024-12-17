pub mod connection;
pub mod reader;
pub mod writer;

pub use connection::Connection;
pub use reader::ReadExtPacket;
pub use writer::WriteExtPacket;

use std::{collections::HashMap, io::Write as _};

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

pub fn to_paletted_container(
    values: &[i32],
    #[allow(unused)] min_direct_bpe: u8,
    max_direct_bpe: u8,
) -> std::io::Result<Box<[u8]>> {
    let mut palette: HashMap<&i32, usize> = HashMap::new();
    values.iter().fold(0, |index, value| {
        if palette.contains_key(value) {
            index
        } else {
            palette.insert(value, index);
            index + 1
        }
    });

    let mut writer = Vec::new();

    // ceil(log2(count))
    let bpe: u8 = match palette.len() {
        0 => panic!(),
        1 => 0,
        entry_count => {
            // TODO: Why does the wiki say to have a min direct bpe?
            TryInto::<u8>::try_into(usize::BITS - entry_count.leading_zeros() - 1).unwrap()
            //.max(min_direct_bpe)
        }
    };

    writer.write_all(&bpe.to_be_bytes())?;

    if bpe == 0 {
        // Single valued (Every entry is same)
        writer.write_varint(values[0])?;
        writer.write_varint(0)?; // Indices array is always empty on single valued
    } else if bpe <= max_direct_bpe {
        // Indirect (Every entry is index into values)
        writer.write_varint(palette.len() as i32)?;
        for (palette_value, _palette_index) in palette.iter() {
            //print!("{:?}: {}, ", palette_value, palette_index);
            writer.write_varint(**palette_value)?;
        }
        //println!();

        let mut data_array = DataArray::new(bpe as usize, values.len());
        values.iter().enumerate().for_each(|(entry_index, value)| {
            let value_index = palette.get(value).unwrap();
            data_array.set(entry_index, *value_index);
        });

        let packed = data_array.into_inner();
        writer.write_varint(packed.len() as i32)?;
        //println!(
        //    "BPE: {}, NUM ENTRIES: {}, NUM LONGS: {}",
        //    bpe,
        //    values.len(),
        //    packed.len()
        //);
        packed
            .iter()
            .try_for_each(|v| writer.write_all(&v.to_be_bytes()))?;
    } else {
        // Direct (Every entry is from values, not indexed)
        writer.write_varint(values.len().div_ceil(2) as i32)?;
        for value in values.iter() {
            writer.write_varint(*value)?;
        }
    }

    Ok(writer.into_boxed_slice())
}

#[derive(Debug)]
struct DataArray {
    bits_per_entry: usize,
    num_entries: usize,
    entries_per_long: usize,
    packed: Box<[u64]>,
}

impl DataArray {
    pub fn new(bits_per_entry: usize, num_entries: usize) -> Self {
        Self {
            bits_per_entry,
            num_entries,
            entries_per_long: 64 / bits_per_entry,
            packed: vec![0; (num_entries * bits_per_entry).div_ceil(64)].into_boxed_slice(),
        }
    }

    pub fn into_inner(self) -> Box<[u64]> {
        self.packed
    }

    fn index_offset(&self, index: usize) -> (usize, usize) {
        (
            index / self.entries_per_long,
            (index % self.entries_per_long) * self.bits_per_entry,
        )
    }

    pub fn set(&mut self, index: usize, value: usize) {
        let (index, offset) = self.index_offset(index);
        assert!(index < self.num_entries);
        assert!(value < (1 << self.bits_per_entry));
        self.packed[index] |= (value as u64) << offset;
    }

    #[allow(dead_code)]
    pub fn get(&mut self, index: usize) -> usize {
        let (index, offset) = self.index_offset(index);
        assert!(index < self.num_entries);
        ((self.packed[index] >> offset) & ((1 << self.bits_per_entry) - 1)) as usize
    }
}

#[test]
fn data_array_test_simple() {
    let mut data = DataArray::new(13, 2);
    data.set(0, 123);
    data.set(1, 456);
    assert_eq!(data.get(0), 123);
    assert_eq!(data.get(1), 456);
}

#[test]
fn data_array_test() {
    let test_data = [
        1, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 16, 9, 14, 10, 12, 0, 2,
    ];
    let mut data = DataArray::new(5, 24);
    test_data
        .iter()
        .enumerate()
        .for_each(|(i, v)| data.set(i, *v));
    test_data.iter().enumerate().for_each(|(i, v)| {
        assert_eq!(data.get(i), *v);
    });
    assert_eq!(
        &data.into_inner().to_vec(),
        &[0x0020863148418841, 0x01018A7260F68C87]
    );
}
