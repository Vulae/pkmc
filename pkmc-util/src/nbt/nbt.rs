use crate::ReadExt as _;
use std::{
    collections::HashMap,
    io::{Read, Write},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NBTError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("NBT invalid tag value {0}")]
    InvalidTagValue(u8),
    #[error("NBT unexpected end tag")]
    UnexpectedEnd,
    #[error("NBT could not write invalid list")]
    InvalidList,
    #[error("NBT error while deserializing: {0:?}")]
    DeserializeError(String),
    #[error("NBT Json cannot convert empty array")]
    JsonConversionEmptyArray,
    #[error("NBT Json cannot convert non-matching array")]
    JsonConversionNonMatchingArray(Vec<NBT>),
    #[error("NBT Json couldnt convert number")]
    JsonCouldntConvertNumber,
    #[error("NBT Json could not convert")]
    JsonCouldntConvert,
    #[error("NBT Json cannot convert number array that contains both ints & floats")]
    JsonMixedIntFloatArray,
}

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
    type Error = NBTError;

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
            _ => Err(NBTError::InvalidTagValue(value)),
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

macro_rules! nbt_from {
    ($type:ty, $value_name:ident => $parse:expr) => {
        impl From<$type> for NBT {
            fn from($value_name: $type) -> Self {
                $parse
            }
        }
    };
}

nbt_from!(i8, v => NBT::Byte(v));
nbt_from!(i16, v => NBT::Short(v));
nbt_from!(i32, v => NBT::Int(v));
nbt_from!(i64, v => NBT::Long(v));
nbt_from!(f32, v => NBT::Float(v));
nbt_from!(f64, v => NBT::Double(v));
nbt_from!(String, v => NBT::String(v));
nbt_from!(&str, v => NBT::String(v.to_owned()));
//nbt_from!(Vec<NBT>, v => NBT::List(v))
nbt_from!(Box<[NBT]>, v => NBT::List(v.to_vec()));

impl<T: Into<NBT>> From<Vec<T>> for NBT {
    fn from(value: Vec<T>) -> Self {
        NBT::List(value.into_iter().map(|v| v.into()).collect::<Vec<_>>())
    }
}

// TODO: More macros for creating NBTs

#[macro_export]
macro_rules! nbt_compound {
    [$($name:expr => $value:expr,)*] => {
        $crate::nbt::NBT::Compound(
            vec![
                $(
                    ($name.to_string(), $value),
                )*
            ].into_iter().collect::<std::collections::HashMap<String, $crate::nbt::NBT>>()
        )
    };
}

impl NBT {
    /// Empty NBT compound.
    pub fn empty() -> Self {
        Self::Compound(HashMap::new())
    }

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

    fn read_tag(data: &mut impl Read, tag: NBTTag) -> Result<NBT, NBTError> {
        match tag {
            NBTTag::End => Err(NBTError::UnexpectedEnd),
            NBTTag::Byte => Ok(NBT::Byte(i8::from_be_bytes(data.read_const()?))),
            NBTTag::Short => Ok(NBT::Short(i16::from_be_bytes(data.read_const()?))),
            NBTTag::Int => Ok(NBT::Int(i32::from_be_bytes(data.read_const()?))),
            NBTTag::Long => Ok(NBT::Long(i64::from_be_bytes(data.read_const()?))),
            NBTTag::Float => Ok(NBT::Float(f32::from_be_bytes(data.read_const()?))),
            NBTTag::Double => Ok(NBT::Double(f64::from_be_bytes(data.read_const()?))),
            NBTTag::ByteArray => Ok(NBT::ByteArray(
                (0..i32::from_be_bytes(data.read_const()?))
                    .map(|_| Ok(i8::from_be_bytes(data.read_const()?)))
                    .collect::<Result<Vec<_>, std::io::Error>>()?
                    .into_boxed_slice(),
            )),
            NBTTag::String => Ok(NBT::String({
                let mut str_bytes = vec![0u8; u16::from_be_bytes(data.read_const()?) as usize];
                data.read_exact(&mut str_bytes)?;
                String::from_utf8(str_bytes)?
            })),
            NBTTag::List => {
                let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
                Ok(NBT::List(
                    (0..i32::from_be_bytes(data.read_const()?))
                        .map(|_| NBT::read_tag(data, tag))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
            NBTTag::Compound => {
                let mut compound = HashMap::new();
                loop {
                    let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
                    if tag == NBTTag::End {
                        break;
                    }
                    let mut str_bytes = vec![0u8; u16::from_be_bytes(data.read_const()?) as usize];
                    data.read_exact(&mut str_bytes)?;
                    let name = String::from_utf8(str_bytes)?;
                    compound.insert(name, NBT::read_tag(data, tag)?);
                }
                Ok(NBT::Compound(compound))
            }
            NBTTag::IntArray => Ok(NBT::IntArray(
                (0..i32::from_be_bytes(data.read_const()?))
                    .map(|_| Ok(i32::from_be_bytes(data.read_const()?)))
                    .collect::<Result<Vec<_>, std::io::Error>>()?
                    .into_boxed_slice(),
            )),
            NBTTag::LongArray => Ok(NBT::LongArray(
                (0..i32::from_be_bytes(data.read_const()?))
                    .map(|_| Ok(i64::from_be_bytes(data.read_const()?)))
                    .collect::<Result<Vec<_>, std::io::Error>>()?
                    .into_boxed_slice(),
            )),
        }
    }

    pub fn read(mut data: impl Read, compressed: bool) -> Result<(String, NBT), NBTError> {
        if compressed {
            unimplemented!("Compressed NBT not implemented.");
        }
        let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
        let mut str_bytes = vec![0u8; u16::from_be_bytes(data.read_const()?) as usize];
        data.read_exact(&mut str_bytes)?;
        Ok((
            String::from_utf8(str_bytes)?,
            NBT::read_tag(&mut data, tag)?,
        ))
    }

    pub fn read_network(mut data: impl Read) -> Result<NBT, NBTError> {
        let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
        NBT::read_tag(&mut data, tag)
    }

    pub fn from_bytes(bytes: &[u8], compressed: bool) -> Result<(String, NBT), NBTError> {
        NBT::read(std::io::Cursor::new(bytes), compressed)
    }

    pub fn from_bytes_network(bytes: &[u8]) -> Result<NBT, NBTError> {
        NBT::read_network(std::io::Cursor::new(bytes))
    }

    fn write_tag(
        &self,
        name: Option<&str>,
        write_tag: bool,
        data: &mut impl Write,
    ) -> Result<(), NBTError> {
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
                    return Err(NBTError::InvalidList);
                };
                let tag = first.tag();
                if list.iter().any(|item| item.tag() != tag) {
                    return Err(NBTError::InvalidList);
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

    pub fn write(
        &self,
        name: &str,
        mut data: impl Write,
        compressed: bool,
    ) -> Result<(), NBTError> {
        if compressed {
            unimplemented!("Compressed NBT not implemented.");
        }
        self.write_tag(Some(name), true, &mut data)
    }

    pub fn write_network(&self, mut data: impl Write) -> Result<(), NBTError> {
        self.write_tag(None, true, &mut data)
    }

    pub fn to_bytes(&self, name: &str, compressed: bool) -> Result<Box<[u8]>, NBTError> {
        let mut data = Vec::new();
        self.write(name, &mut data, compressed)?;
        Ok(data.into_boxed_slice())
    }

    pub fn to_bytes_network(&self) -> Result<Box<[u8]>, NBTError> {
        let mut data = Vec::new();
        self.write_network(&mut data)?;
        Ok(data.into_boxed_slice())
    }
}

/// https://minecraft.wiki/w/NBT_format#Conversion_from_JSON
impl TryFrom<serde_json::Value> for NBT {
    type Error = NBTError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        #[derive(Eq, PartialEq, Clone, Copy, Debug)]
        enum BestMatchingNumberType {
            Byte,
            Short,
            Int,
            Long,
            Float,
            Double,
        }

        impl From<&serde_json::Number> for BestMatchingNumberType {
            fn from(value: &serde_json::Number) -> Self {
                if let Some(number) = value.as_i64() {
                    if i8::try_from(number).is_ok() {
                        BestMatchingNumberType::Byte
                    } else if i16::try_from(number).is_ok() {
                        BestMatchingNumberType::Short
                    } else if i32::try_from(number).is_ok() {
                        BestMatchingNumberType::Int
                    } else {
                        BestMatchingNumberType::Long
                    }
                } else if let Some(number) = value.as_f64() {
                    // TODO: Does this actually check if precision is lost?
                    if ((number as f32) as f64) == number {
                        BestMatchingNumberType::Float
                    } else {
                        BestMatchingNumberType::Double
                    }
                } else {
                    unreachable!()
                }
            }
        }

        impl BestMatchingNumberType {
            fn convert_json_to_nbt(&self, value: &serde_json::Number) -> Result<NBT, NBTError> {
                fn do_conversion_int<T: TryFrom<i64>>(
                    value: &serde_json::Number,
                ) -> Result<T, NBTError> {
                    value
                        .as_i64()
                        .ok_or(NBTError::JsonCouldntConvertNumber)?
                        .try_into()
                        .map_err(|_| NBTError::JsonCouldntConvertNumber)
                }

                Ok(match self {
                    BestMatchingNumberType::Byte => NBT::Byte(do_conversion_int(value)?),
                    BestMatchingNumberType::Short => NBT::Short(do_conversion_int(value)?),
                    BestMatchingNumberType::Int => NBT::Int(do_conversion_int(value)?),
                    BestMatchingNumberType::Long => NBT::Long(do_conversion_int(value)?),
                    BestMatchingNumberType::Float => {
                        NBT::Float(value.as_f64().ok_or(NBTError::JsonCouldntConvertNumber)? as f32)
                    }
                    BestMatchingNumberType::Double => {
                        NBT::Double(value.as_f64().ok_or(NBTError::JsonCouldntConvertNumber)?)
                    }
                })
            }

            fn is_int(&self) -> bool {
                matches!(
                    self,
                    BestMatchingNumberType::Byte
                        | BestMatchingNumberType::Short
                        | BestMatchingNumberType::Int
                        | BestMatchingNumberType::Long
                )
            }

            fn is_float(&self) -> bool {
                matches!(
                    self,
                    BestMatchingNumberType::Float | BestMatchingNumberType::Double
                )
            }

            fn value(&self) -> u8 {
                match self {
                    BestMatchingNumberType::Byte => 1,
                    BestMatchingNumberType::Short => 2,
                    BestMatchingNumberType::Int => 3,
                    BestMatchingNumberType::Long => 4,
                    BestMatchingNumberType::Float => 5,
                    BestMatchingNumberType::Double => 6,
                }
            }

            fn rank(
                self,
                other: BestMatchingNumberType,
            ) -> Result<BestMatchingNumberType, NBTError> {
                if (self.is_int() ^ other.is_int()) || (self.is_float() ^ other.is_float()) {
                    return Err(NBTError::JsonCouldntConvertNumber);
                }
                if self.value() > other.value() {
                    Ok(self)
                } else {
                    Ok(other)
                }
            }
        }

        match value {
            serde_json::Value::Bool(bool) => Ok(NBT::Byte(if bool { 1 } else { 0 })),
            serde_json::Value::Number(number) => {
                Ok(BestMatchingNumberType::from(&number).convert_json_to_nbt(&number)?)
                //if let Some(number) = number.as_i64() {
                //    if let Ok(byte) = i8::try_from(number) {
                //        Ok(NBT::Byte(byte))
                //    } else if let Ok(short) = i16::try_from(number) {
                //        Ok(NBT::Short(short))
                //    } else if let Ok(int) = i32::try_from(number) {
                //        Ok(NBT::Int(int))
                //    } else {
                //        Ok(NBT::Long(number))
                //    }
                //} else if let Some(number) = number.as_f64() {
                //    // TODO: Does this actually check if precision is lost?
                //    if ((number as f32) as f64) == number {
                //        Ok(NBT::Float(number as f32))
                //    } else {
                //        Ok(NBT::Double(number))
                //    }
                //} else {
                //    Err(NBTError::JsonInvalidNumber(number))
                //}
            }
            serde_json::Value::String(string) => Ok(NBT::String(string)),
            serde_json::Value::Array(array)
                if !array.is_empty() && array.iter().all(serde_json::Value::is_number) =>
            {
                // TODO: Convert to NBT::ByteArray, NBT::IntArray, & NBT::LongArray if possible.

                let array = array
                    .into_iter()
                    .map(|value| value.as_number().cloned().unwrap())
                    .collect::<Vec<_>>();

                let arena = array.iter().try_fold(
                    BestMatchingNumberType::from(array.first().unwrap()),
                    |arena, number| {
                        BestMatchingNumberType::rank(arena, BestMatchingNumberType::from(number))
                    },
                )?;

                let converted = array
                    .iter()
                    .map(|number| arena.convert_json_to_nbt(number))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(NBT::List(converted))
            }
            serde_json::Value::Array(array) => {
                let nbts = array
                    .into_iter()
                    .map(NBT::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                let Some(first) = nbts.first() else {
                    return Err(NBTError::JsonConversionEmptyArray);
                };
                let first_tag = first.tag();
                if nbts.iter().any(|nbt| nbt.tag() != first_tag) {
                    return Err(NBTError::JsonConversionNonMatchingArray(nbts));
                }
                Ok(NBT::List(nbts))
            }
            serde_json::Value::Object(object) => Ok(NBT::Compound(
                object
                    .into_iter()
                    .flat_map(|(key, value)| {
                        if let serde_json::Value::Array(ref array) = value {
                            if array.is_empty() {
                                return None;
                            }
                        }
                        Some((key, value))
                    })
                    .map(|(key, value)| Ok::<_, NBTError>((key, NBT::try_from(value)?)))
                    .collect::<Result<HashMap<_, _>, _>>()?,
            )),
            _ => Err(NBTError::JsonCouldntConvert),
        }
    }
}

/// https://minecraft.wiki/w/NBT_format#Conversion_to_JSON
impl From<NBT> for serde_json::Value {
    fn from(value: NBT) -> Self {
        match value {
            NBT::Byte(byte) => serde_json::Value::from(byte),
            NBT::Short(short) => serde_json::Value::from(short),
            NBT::Int(int) => serde_json::Value::from(int),
            NBT::Long(long) => serde_json::Value::from(long),
            NBT::Float(float) => serde_json::Value::from(float),
            NBT::Double(double) => serde_json::Value::from(double),
            NBT::String(string) => serde_json::Value::from(string),
            NBT::List(list) => serde_json::Value::from(list),
            NBT::Compound(compound) => serde_json::Value::Object(
                compound
                    .into_iter()
                    .map(|(key, value)| (key, serde_json::Value::from(value)))
                    .collect::<serde_json::Map<_, _>>(),
            ),
            NBT::ByteArray(byte_array) => serde_json::Value::from(byte_array.to_vec()),
            NBT::IntArray(int_array) => serde_json::Value::from(int_array.to_vec()),
            NBT::LongArray(long_array) => serde_json::Value::from(long_array.to_vec()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{NBTError, NBT};

    #[test]
    fn bigtest() -> Result<(), NBTError> {
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
