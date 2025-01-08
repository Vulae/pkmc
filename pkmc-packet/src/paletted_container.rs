use itertools::Itertools;
use pkmc_util::PackedArray;

use crate::WriteExtPacket as _;
use std::{collections::HashMap, io::Write};

fn calculate_bpe(num_unique_values: usize) -> u32 {
    match num_unique_values {
        0 => panic!("calculate_bpe cannot calculate bpe with 0 unique values."),
        1 => 0,
        2 => 1,
        _ => u64::BITS - num_unique_values.leading_zeros(),
    }
}

#[derive(Debug)]
struct SinglePalettedContainer {
    value: i32,
}

impl SinglePalettedContainer {
    fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&0u8.to_be_bytes())?;
        writer.write_varint(self.value)?;
        writer.write_varint(0)?;
        Ok(())
    }
}

#[derive(Debug)]
struct IndirectPalettedContainer<'a> {
    bpe: u32,
    palette: HashMap<i32, usize>,
    values: &'a [i32],
}

impl IndirectPalettedContainer<'_> {
    fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&(self.bpe as u8).to_be_bytes())?;

        writer.write_varint(self.palette.len() as i32)?;
        self.palette
            .iter()
            .sorted_by(|(_, a), (_, b)| a.cmp(b))
            .try_for_each(|(v, _)| {
                writer.write_varint(*v)?;
                Ok::<_, std::io::Error>(())
            })?;

        let mut packed = PackedArray::new(self.bpe as u8, self.values.len());
        let remaining = packed.consume(
            self.values
                .iter()
                .map(|value| self.palette.get(value).cloned().unwrap() as u64),
        );
        assert!(remaining.count() == 0);
        let packed = packed.into_inner();

        writer.write_varint(packed.len() as i32)?;
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
    bpe: u32,
    palette: HashMap<i32, usize>,
    values: &'a [i32],
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
        indirect_size_range: std::ops::RangeInclusive<u32>,
        direct_size: u32,
    ) -> Self {
        assert!(*indirect_size_range.start() > 0);
        assert!(*indirect_size_range.end() < direct_size);

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
            // TODO:
            #[allow(unused)]
            PalettedContainerEnum::Direct(direct_paletted_container) => {
                todo!("Direct paletted container not yet implemented")
            }
        }
    }
}

pub fn to_paletted_data(
    values: &[i32],
    indirect_size_range: std::ops::RangeInclusive<u32>,
    direct_size: u32,
) -> std::io::Result<Box<[u8]>> {
    let paletted = PalettedContainerEnum::from_values(values, indirect_size_range, direct_size);
    let mut writer = Vec::new();
    paletted.write(&mut writer)?;
    Ok(writer.into_boxed_slice())
}

#[cfg(test)]
mod test {
    use crate::paletted_container::to_paletted_data;

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
