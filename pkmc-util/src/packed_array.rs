#[derive(Debug, Clone)]
pub struct PackedArray<T> {
    bits_per_entry: u8,
    num_entries: usize,
    entries_per_long: u8,
    entry_mask: u64,
    packed: T,
}

impl PackedArray<Vec<u64>> {
    /// If this returns 0, you should not be using PackedArray
    pub const fn bits_per_entry(max_value: u64) -> u8 {
        match max_value {
            0 => 0,
            1 => 1,
            _ => (u64::BITS - max_value.leading_zeros()) as u8,
        }
    }

    pub const fn packed_size(bits_per_entry: u8, num_entries: usize) -> usize {
        u64::div_ceil(
            num_entries as u64,
            (u64::BITS / bits_per_entry as u32) as u64,
        ) as usize
    }
    pub fn new(bits_per_entry: u8, num_entries: usize) -> Self {
        Self::from_inner(
            vec![0; PackedArray::packed_size(bits_per_entry, num_entries)],
            bits_per_entry,
            num_entries,
        )
    }
}

impl<T> PackedArray<T>
where
    T: AsRef<[u64]>,
{
    pub fn from_inner(packed: T, bits_per_entry: u8, num_entries: usize) -> Self {
        // NOTE: For some reason, in EXTREMELY rare cases, Minecraft uses more space than needed.
        assert!(packed.as_ref().len() >= PackedArray::packed_size(bits_per_entry, num_entries));
        Self {
            bits_per_entry,
            num_entries,
            entries_per_long: 64 / bits_per_entry,
            entry_mask: (1 << bits_per_entry) - 1,
            packed,
        }
    }

    pub fn into_inner(self) -> T {
        self.packed
    }

    #[inline(always)]
    fn index_offset(&self, index: usize) -> (usize, u64) {
        (
            index / (self.entries_per_long as usize),
            ((index as u64) % (self.entries_per_long as u64)) * (self.bits_per_entry as u64),
        )
    }

    pub fn get(&self, index: usize) -> Option<u64> {
        if index >= self.num_entries {
            return None;
        }
        let (index, offset) = self.index_offset(index);
        Some((self.packed.as_ref()[index] >> offset) & self.entry_mask)
    }

    /// Panics if out of bounds
    pub fn get_unchecked(&self, index: usize) -> u64 {
        assert!(index < self.num_entries);
        self.get(index).unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.num_entries).map(|i| self.get_unchecked(i))
    }
}

impl<T> PackedArray<T>
where
    T: AsRef<[u64]> + AsMut<[u64]>,
{
    pub fn set(&mut self, index: usize, value: u64) {
        if index >= self.num_entries || value > self.entry_mask {
            return;
        }
        let (index, offset) = self.index_offset(index);
        let packed_value = self.packed.as_mut().get_mut(index).unwrap();
        *packed_value &= !(self.entry_mask << offset);
        *packed_value |= value << offset;
    }

    /// Panics if out of bounds or if value is too large
    pub fn set_unchecked(&mut self, index: usize, value: u64) {
        assert!(index < self.num_entries);
        assert!(value <= self.entry_mask);
        self.set(index, value);
    }

    /// Consumes the iterator placing values in self
    /// If self doesn't fit all values, returns remaining values not consumed.
    pub fn consume<I>(&mut self, mut iter: I) -> I
    where
        I: Iterator<Item = u64>,
    {
        for (index, value) in iter.by_ref().enumerate() {
            self.set_unchecked(index, value);
            if index >= self.num_entries {
                break;
            }
        }
        iter
    }
}

#[cfg(test)]
mod test {
    use crate::PackedArray;

    fn test_packed_array(values: &[u64], bpe: u8, longs: &[u64]) {
        let mut packed = PackedArray::new(bpe, values.len());
        assert!(packed.consume(values.iter().cloned()).count() == 0);
        values.iter().enumerate().for_each(|(i, v)| {
            assert_eq!(packed.get_unchecked(i), *v);
        });
        assert_eq!(packed.into_inner(), longs);
    }

    #[test]
    fn packed_array_test() {
        test_packed_array(
            &[
                1, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 16, 9, 14, 10, 12, 0, 2,
            ],
            5,
            &[0x0020863148418841, 0x01018A7260F68C87],
        );
    }
}
