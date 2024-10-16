use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    io::{Read, Write},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NBTTag {
    End,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    List,
    Compound,
    IntArray,
    LongArray,
}

impl TryFrom<u8> for NBTTag {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(NBTTag::End),
            1 => Ok(NBTTag::Byte),
            2 => Ok(NBTTag::Short),
            3 => Ok(NBTTag::Int),
            4 => Ok(NBTTag::Long),
            5 => Ok(NBTTag::Float),
            6 => Ok(NBTTag::Double),
            7 => Ok(NBTTag::ByteArray),
            8 => Ok(NBTTag::String),
            9 => Ok(NBTTag::List),
            10 => Ok(NBTTag::Compound),
            11 => Ok(NBTTag::IntArray),
            12 => Ok(NBTTag::LongArray),
            _ => Err(anyhow!("Invalid NBTTag value {}", value)),
        }
    }
}

impl From<NBTTag> for u8 {
    fn from(val: NBTTag) -> Self {
        match val {
            NBTTag::End => 0,
            NBTTag::Byte => 1,
            NBTTag::Short => 2,
            NBTTag::Int => 3,
            NBTTag::Long => 4,
            NBTTag::Float => 5,
            NBTTag::Double => 6,
            NBTTag::ByteArray => 7,
            NBTTag::String => 8,
            NBTTag::List => 9,
            NBTTag::Compound => 10,
            NBTTag::IntArray => 11,
            NBTTag::LongArray => 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NBT {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    // TODO: Refactor, make own NBTList type to fix alot of the stupid errors that could occur.
    List(Vec<NBT>),
    Compound(HashMap<String, NBT>),
    ByteArray(Box<[i8]>),
    IntArray(Box<[i32]>),
    LongArray(Box<[i64]>),
}

// TODO: More macros for creating NBTs

#[macro_export]
macro_rules! nbt_compound {
    [$($name:expr => $value:expr,)*] => {
        NBT::Compound(
            vec![
                $(
                    ($name.to_string(), $value),
                )*
            ].into_iter().collect::<std::collections::HashMap<String, NBT>>()
        )
    };
}

fn read<const N: usize>(reader: &mut impl Read) -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

impl NBT {
    fn tag(&self) -> NBTTag {
        match self {
            NBT::Byte(..) => NBTTag::Byte,
            NBT::Short(..) => NBTTag::Short,
            NBT::Int(..) => NBTTag::Int,
            NBT::Long(..) => NBTTag::Long,
            NBT::Float(..) => NBTTag::Float,
            NBT::Double(..) => NBTTag::Double,
            NBT::String(..) => NBTTag::String,
            NBT::List(..) => NBTTag::List,
            NBT::Compound(..) => NBTTag::Compound,
            NBT::ByteArray(..) => NBTTag::ByteArray,
            NBT::IntArray(..) => NBTTag::IntArray,
            NBT::LongArray(..) => NBTTag::LongArray,
        }
    }

    fn read_tag(data: &mut impl Read, tag: NBTTag) -> Result<NBT> {
        match tag {
            NBTTag::End => Err(anyhow!("NBT read unexpected NBTTag::End")),
            NBTTag::Byte => Ok(NBT::Byte(i8::from_be_bytes(read(data)?))),
            NBTTag::Short => Ok(NBT::Short(i16::from_be_bytes(read(data)?))),
            NBTTag::Int => Ok(NBT::Int(i32::from_be_bytes(read(data)?))),
            NBTTag::Long => Ok(NBT::Long(i64::from_be_bytes(read(data)?))),
            NBTTag::Float => Ok(NBT::Float(f32::from_be_bytes(read(data)?))),
            NBTTag::Double => Ok(NBT::Double(f64::from_be_bytes(read(data)?))),
            NBTTag::ByteArray => Ok(NBT::ByteArray(
                (0..i32::from_be_bytes(read(data)?))
                    .map(|_| Ok(i8::from_be_bytes(read(data)?)))
                    .collect::<Result<Vec<_>, anyhow::Error>>()?
                    .into_boxed_slice(),
            )),
            NBTTag::String => Ok(NBT::String({
                let mut str_bytes = vec![0u8; u16::from_be_bytes(read(data)?) as usize];
                data.read_exact(&mut str_bytes)?;
                String::from_utf8(str_bytes)?
            })),
            NBTTag::List => {
                let tag = NBTTag::try_from(u8::from_be_bytes(read(data)?))?;
                Ok(NBT::List(
                    (0..i32::from_be_bytes(read(data)?))
                        .map(|_| NBT::read_tag(data, tag))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
            NBTTag::Compound => {
                let mut compound = HashMap::new();
                loop {
                    let tag = NBTTag::try_from(u8::from_be_bytes(read(data)?))?;
                    if tag == NBTTag::End {
                        break;
                    }
                    let mut str_bytes = vec![0u8; u16::from_be_bytes(read(data)?) as usize];
                    data.read_exact(&mut str_bytes)?;
                    let name = String::from_utf8(str_bytes)?;
                    compound.insert(name, NBT::read_tag(data, tag)?);
                }
                Ok(NBT::Compound(compound))
            }
            NBTTag::IntArray => Ok(NBT::IntArray(
                (0..i32::from_be_bytes(read(data)?))
                    .map(|_| Ok(i32::from_be_bytes(read(data)?)))
                    .collect::<Result<Vec<_>, anyhow::Error>>()?
                    .into_boxed_slice(),
            )),
            NBTTag::LongArray => Ok(NBT::LongArray(
                (0..i32::from_be_bytes(read(data)?))
                    .map(|_| Ok(i64::from_be_bytes(read(data)?)))
                    .collect::<Result<Vec<_>, anyhow::Error>>()?
                    .into_boxed_slice(),
            )),
        }
    }

    pub fn read(mut data: impl Read, compressed: bool) -> Result<(String, NBT)> {
        if compressed {
            unimplemented!("Compressed NBT not implemented.");
        }
        let tag = NBTTag::try_from(u8::from_be_bytes(read(&mut data)?))?;
        let mut str_bytes = vec![0u8; u16::from_be_bytes(read(&mut data)?) as usize];
        data.read_exact(&mut str_bytes)?;
        Ok((
            String::from_utf8(str_bytes)?,
            NBT::read_tag(&mut data, tag)?,
        ))
    }

    pub fn read_network(mut data: impl Read) -> Result<NBT> {
        let tag = NBTTag::try_from(u8::from_be_bytes(read(&mut data)?))?;
        NBT::read_tag(&mut data, tag)
    }

    pub fn from_bytes(bytes: &[u8], compressed: bool) -> Result<(String, NBT)> {
        NBT::read(std::io::Cursor::new(bytes), compressed)
    }

    pub fn from_bytes_network(bytes: &[u8]) -> Result<NBT> {
        NBT::read_network(std::io::Cursor::new(bytes))
    }

    fn write_tag(&self, name: Option<&str>, write_tag: bool, data: &mut impl Write) -> Result<()> {
        if write_tag {
            data.write_all(&u8::from(self.tag()).to_be_bytes())?;
        }
        if let Some(name) = name {
            data.write_all(&(name.len() as u16).to_be_bytes())?;
            data.write_all(name.as_bytes())?;
        }
        match self {
            NBT::Byte(byte) => data.write_all(&byte.to_be_bytes())?,
            NBT::Short(short) => data.write_all(&short.to_be_bytes())?,
            NBT::Int(int) => data.write_all(&int.to_be_bytes())?,
            NBT::Long(long) => data.write_all(&long.to_be_bytes())?,
            NBT::Float(float) => data.write_all(&float.to_be_bytes())?,
            NBT::Double(double) => data.write_all(&double.to_be_bytes())?,
            NBT::String(string) => {
                data.write_all(&(string.len() as u16).to_be_bytes())?;
                data.write_all(string.as_bytes())?;
            }
            NBT::List(list) => {
                let Some(first) = list.first() else {
                    return Err(anyhow!("NBT could infer list type"));
                };
                let tag = first.tag();
                if list.iter().any(|item| item.tag() != tag) {
                    return Err(anyhow!("NBT list items dont match"));
                }
                data.write_all(&u8::from(tag).to_be_bytes())?;
                data.write_all(&(list.len() as u32).to_be_bytes())?;
                for item in list.iter() {
                    item.write_tag(None, false, data)?;
                }
            }
            NBT::Compound(compound) => {
                for (key, value) in compound.iter() {
                    value.write_tag(Some(key), true, data)?;
                }
                data.write_all(&u8::from(NBTTag::End).to_be_bytes())?;
            }
            NBT::ByteArray(bytes) => {
                data.write_all(&(bytes.len() as u32).to_be_bytes())?;
                data.write_all(
                    &bytes
                        .iter()
                        .flat_map(|b| b.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
            NBT::IntArray(ints) => {
                data.write_all(&(ints.len() as u32).to_be_bytes())?;
                data.write_all(
                    &ints
                        .iter()
                        .flat_map(|i| i.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
            NBT::LongArray(longs) => {
                data.write_all(&(longs.len() as u32).to_be_bytes())?;
                data.write_all(
                    &longs
                        .iter()
                        .flat_map(|l| l.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
        }
        Ok(())
    }

    pub fn write(&self, name: &str, mut data: impl Write, compressed: bool) -> Result<()> {
        if compressed {
            unimplemented!("Compressed NBT not implemented.");
        }
        self.write_tag(Some(name), true, &mut data)
    }

    pub fn write_network(&self, mut data: impl Write) -> Result<()> {
        self.write_tag(None, true, &mut data)
    }

    pub fn to_bytes(&self, name: &str, compressed: bool) -> Result<Box<[u8]>> {
        let mut data = Vec::new();
        self.write(name, &mut data, compressed)?;
        Ok(data.into_boxed_slice())
    }

    pub fn to_bytes_network(&self) -> Result<Box<[u8]>> {
        let mut data = Vec::new();
        self.write_network(&mut data)?;
        Ok(data.into_boxed_slice())
    }
}

#[cfg(test)]
mod test {
    use super::NBT;

    #[test]
    fn bigtest() -> anyhow::Result<()> {
        let bigtest_file = include_bytes!("./bigtest.nbt");
        let bigtest_nbt = (
            "Level".to_string(),
            nbt_compound![
                "nested compound test" => nbt_compound![
                    "egg" => nbt_compound![
                        "name" => NBT::String("Eggbert".to_string()),
                        "value" => NBT::Float(0.5),
                    ],
                    "ham" => nbt_compound![
                        "name" => NBT::String("Hampus".to_string()),
                        "value" => NBT::Float(0.75),
                    ],
                ],
                "intTest" => NBT::Int(2147483647),
                "byteTest" => NBT::Byte(127),
                "stringTest" => NBT::String("HELLO WORLD THIS IS A TEST STRING ÅÄÖ!".to_string()),
                "listTest (long)" => NBT::List(vec![
                    NBT::Long(11),
                    NBT::Long(12),
                    NBT::Long(13),
                    NBT::Long(14),
                    NBT::Long(15),
                ]),
                "doubleTest" => NBT::Double(0.493_128_713_218_231_5),
                "floatTest" => NBT::Float(0.498_231_47),
                "longTest" => NBT::Long(9223372036854775807),
                "listTest (compound)" => NBT::List(vec![
                    nbt_compound![
                        "created-on" => NBT::Long(1264099775885),
                        "name" => NBT::String("Compound tag #0".to_string()),
                    ],
                    nbt_compound![
                        "created-on" => NBT::Long(1264099775885),
                        "name" => NBT::String("Compound tag #1".to_string()),
                    ],
                ]),
                "byteArrayTest (the first 1000 values of (n*n*255+n*7)%100, starting with n=0 (0, 62, 34, 16, 8, ...))" => NBT::ByteArray((0i32..1000i32).map(|i| {
                    ((i*i*255+i*7) % 100) as i8
                }).collect::<Vec<i8>>().into_boxed_slice()),
                "shortTest" => NBT::Short(32767),
            ],
        );

        let parsed = NBT::from_bytes(bigtest_file, false)?;

        assert_eq!(parsed, bigtest_nbt);

        let binary = parsed.1.to_bytes(&parsed.0, false)?;
        let parsed = NBT::from_bytes(&binary, false)?;

        assert_eq!(parsed, bigtest_nbt);

        Ok(())
    }
}
