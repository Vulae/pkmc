#[derive(Debug, Clone)]
pub struct PackedArray {
    bits_per_entry: u8,
    num_entries: usize,
    entries_per_long: u8,
    entry_mask: u64,
    packed: Box<[u64]>,
}

impl PackedArray {
    /// If this returns 0, you should not be using PackedArray
    pub const fn bits_per_entry(max_value: u64) -> u8 {
        match max_value {
            0 => 0,
            1 => 1,
            _ => (u64::BITS - max_value.leading_zeros()) as u8,
        }
    }

    pub fn packed_size(bits_per_entry: u8, num_entries: usize) -> usize {
        u64::div_ceil(
            num_entries as u64,
            (u64::BITS / bits_per_entry as u32) as u64,
        ) as usize
    }
}

impl PackedArray {
    pub fn from_inner(packed: Box<[u64]>, bits_per_entry: u8, num_entries: usize) -> Self {
        //assert_eq!(
        //    PackedArray::packed_size(bits_per_entry, num_entries),
        //    packed.len(),
        //);
        assert!(packed.len() >= PackedArray::packed_size(bits_per_entry, num_entries));
        Self {
            bits_per_entry,
            num_entries,
            entries_per_long: 64 / bits_per_entry,
            entry_mask: (1 << bits_per_entry) - 1,
            packed,
        }
    }

    pub fn new(bits_per_entry: u8, num_entries: usize) -> Self {
        Self::from_inner(
            vec![0; PackedArray::packed_size(bits_per_entry, num_entries)].into_boxed_slice(),
            bits_per_entry,
            num_entries,
        )
    }

    pub fn into_inner(self) -> Box<[u64]> {
        self.packed
    }

    fn index_offset(&self, index: usize) -> (usize, u64) {
        (
            index / (self.entries_per_long as usize),
            ((index as u64) % (self.entries_per_long as u64)) * (self.bits_per_entry as u64),
        )
    }

    pub fn set(&mut self, index: usize, value: u64) {
        if index >= self.num_entries || value > self.entry_mask {
            return;
        }
        let (index, offset) = self.index_offset(index);
        self.packed[index] |= value << offset;
    }

    pub fn set_unchecked(&mut self, index: usize, value: u64) {
        assert!(index < self.num_entries);
        assert!(value <= self.entry_mask);
        self.set(index, value);
    }

    pub fn get(&mut self, index: usize) -> Option<u64> {
        if index >= self.num_entries {
            return None;
        }
        let (index, offset) = self.index_offset(index);
        Some((self.packed[index] >> offset) & self.entry_mask)
    }

    pub fn get_unchecked(&mut self, index: usize) -> u64 {
        assert!(index < self.num_entries);
        self.get(index).unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::PackedArray;

    #[test]
    fn packed_array_test() {
        let test_data = [
            1, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 16, 9, 14, 10, 12, 0, 2,
        ];
        let mut data = PackedArray::new(5, 24);
        test_data
            .iter()
            .enumerate()
            .for_each(|(i, v)| data.set_unchecked(i, *v));
        test_data.iter().enumerate().for_each(|(i, v)| {
            assert_eq!(data.get_unchecked(i), *v);
        });
        assert_eq!(
            &data.into_inner().to_vec(),
            &[0x0020863148418841, 0x01018A7260F68C87]
        );
    }
}
