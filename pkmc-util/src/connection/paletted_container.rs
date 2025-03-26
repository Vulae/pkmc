use itertools::Itertools as _;

use crate::{connection::PacketEncoder as _, PackedArray};
use std::{collections::HashMap, io::Write};

pub const fn calculate_bpe(num_unique_values: usize) -> u8 {
    debug_assert!(
        num_unique_values != 0,
        "calculate_bpe cannot calculate bpe with 0 unique values."
    );
    (u64::BITS - (num_unique_values - 1).leading_zeros()) as u8
}

#[derive(Debug)]
struct SinglePalettedContainer {
    value: i32,
}

impl SinglePalettedContainer {
    fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&0u8.to_be_bytes())?;
        writer.encode(self.value)?;
        Ok(())
    }
}

#[derive(Debug)]
struct IndirectPalettedContainer<'a> {
    bpe: u8,
    palette: HashMap<i32, usize>,
    values: &'a [i32],
}

impl IndirectPalettedContainer<'_> {
    fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&self.bpe.to_be_bytes())?;

        writer.encode(self.palette.len() as i32)?;
        self.palette
            .iter()
            .sorted_by(|(_, a), (_, b)| a.cmp(b))
            .try_for_each(|(v, _)| writer.encode(*v))?;

        let mut packed = PackedArray::new(self.bpe, self.values.len());
        let remaining = packed.consume(
            self.values
                .iter()
                .map(|value| self.palette.get(value).cloned().unwrap() as u64),
        );
        debug_assert!(remaining.count() == 0);
        let packed = packed.into_inner();

        packed.iter().try_for_each(|v| {
            writer.write_all(&v.to_be_bytes())?;
            Ok::<_, std::io::Error>(())
        })?;

        Ok(())
    }
}

#[derive(Debug)]
#[allow(unused)]
struct DirectPalettedContainer<'a> {
    bpe: u8,
    palette: HashMap<i32, usize>,
    values: &'a [i32],
}

impl DirectPalettedContainer<'_> {
    fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&self.bpe.to_be_bytes())?;

        let mut packed = PackedArray::new(self.bpe, self.values.len());
        let remaining = packed.consume(self.values.iter().map(|v| *v as u64));
        debug_assert!(remaining.count() == 0);
        let packed = packed.into_inner();

        packed.iter().try_for_each(|v| {
            writer.write_all(&v.to_be_bytes())?;
            Ok::<_, std::io::Error>(())
        })?;

        Ok(())
    }
}

#[derive(Debug)]
enum PalettedContainerEnum<'a> {
    Single(SinglePalettedContainer),
    Indirect(IndirectPalettedContainer<'a>),
    Direct(DirectPalettedContainer<'a>),
}

impl<'a> PalettedContainerEnum<'a> {
    fn from_values(
        values: &'a [i32],
        indirect_size_range: std::ops::RangeInclusive<u8>,
        direct_size: u8,
    ) -> Self {
        debug_assert!(*indirect_size_range.start() > 0);
        debug_assert!(*indirect_size_range.end() < direct_size);

        let mut palette = HashMap::new();
        values.iter().for_each(|v| {
            let count = palette.len();
            palette.entry(*v).or_insert(count);
        });

        match calculate_bpe(palette.len()) {
            0 => Self::Single(SinglePalettedContainer {
                value: *palette.keys().next().unwrap(),
            }),
            bpe if bpe <= *indirect_size_range.end() => Self::Indirect(IndirectPalettedContainer {
                bpe: bpe.clamp(*indirect_size_range.start(), *indirect_size_range.end()),
                palette,
                values,
            }),
            bpe if bpe <= direct_size => Self::Direct(DirectPalettedContainer {
                bpe: direct_size,
                palette,
                values,
            }),
            _ => panic!("Paletted container palette size is bigger than direct size."),
        }
    }

    fn write(&self, writer: impl Write) -> std::io::Result<()> {
        match self {
            PalettedContainerEnum::Single(single_paletted_container) => {
                single_paletted_container.write(writer)
            }
            PalettedContainerEnum::Indirect(indirect_paletted_container) => {
                indirect_paletted_container.write(writer)
            }
            PalettedContainerEnum::Direct(direct_paletted_container) => {
                direct_paletted_container.write(writer)
            }
        }
    }
}

pub fn to_paletted_data(
    values: &[i32],
    indirect_size_range: std::ops::RangeInclusive<u8>,
    direct_size: u8,
) -> std::io::Result<Box<[u8]>> {
    let paletted = PalettedContainerEnum::from_values(values, indirect_size_range, direct_size);
    let mut writer = Vec::new();
    paletted.write(&mut writer)?;
    Ok(writer.into_boxed_slice())
}

pub fn to_paletted_data_singular(value: i32) -> std::io::Result<Box<[u8]>> {
    let mut writer = Vec::new();
    SinglePalettedContainer { value }.write(&mut writer)?;
    Ok(writer.into_boxed_slice())
}

pub fn to_paletted_data_precomputed(
    palette: &[i32],
    packed_indices: &[i64],
    indirect_size_range: std::ops::RangeInclusive<u8>,
    direct_size: u8,
) -> std::io::Result<Box<[u8]>> {
    debug_assert!(*indirect_size_range.start() > 0);
    debug_assert!(*indirect_size_range.end() < direct_size);

    match calculate_bpe(palette.len()) {
        0 => to_paletted_data_singular(palette[0]),
        bpe if bpe <= *indirect_size_range.end() => {
            let mut writer = Vec::new();

            writer.write_all(
                &bpe.clamp(*indirect_size_range.start(), *indirect_size_range.end())
                    .to_be_bytes(),
            )?;

            writer.encode(palette.len() as i32)?;
            palette.iter().try_for_each(|v| writer.encode(*v))?;

            packed_indices
                .iter()
                .try_for_each(|v| writer.write_all(&v.to_be_bytes()))?;

            Ok(writer.into_boxed_slice())
        }
        bpe if bpe <= direct_size => {
            let mut writer = Vec::new();

            writer.write_all(&direct_size.to_be_bytes())?;

            packed_indices
                .iter()
                .try_for_each(|v| writer.write_all(&v.to_be_bytes()))?;

            Ok(writer.into_boxed_slice())
        }
        _ => panic!("Paletted container palette size is bigger than direct size."),
    }
}

#[cfg(test)]
mod test {
    use crate::connection::paletted_container::to_paletted_data;

    #[test]
    fn test() -> std::io::Result<()> {
        assert_eq!(to_paletted_data(&[69], 4..=8, 15)?.as_ref(), [0, 69, 0]);
        assert_eq!(
            to_paletted_data(&[4, 7], 4..=8, 15)?.as_ref(),
            [4, 2, 4, 7, 1, 0, 0, 0, 0, 0, 0, 0, 0b0001_0000]
        );
        Ok(())
    }
}
