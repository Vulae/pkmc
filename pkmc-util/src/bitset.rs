use std::io::{Read, Write};

use crate::{
    packet::{PacketDecodable, PacketDecoder as _, PacketEncodable, PacketEncoder as _},
    ReadExt as _,
};

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

impl PacketEncodable for &BitSet {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.encode(self.data.len() as i32)?;
        self.data
            .iter()
            .try_for_each(|v| writer.write_all(&v.to_be_bytes()))?;
        Ok(())
    }
}

impl PacketDecodable for BitSet {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        let long_count = reader.decode::<i32>()? as usize;
        Ok(BitSet::from_inner(
            long_count * 64,
            (0..long_count)
                .map(|_| Ok::<_, std::io::Error>(u64::from_be_bytes(reader.read_const()?)))
                .collect::<Result<Box<[_]>, _>>()?,
        ))
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

impl<const N: usize> PacketEncodable for &FixedBitSet<N> {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&self.data)
    }
}

impl<const N: usize> PacketDecodable for FixedBitSet<N> {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        Ok(Self::from_inner(reader.read_var(N.div_ceil(8))?))
    }
}
