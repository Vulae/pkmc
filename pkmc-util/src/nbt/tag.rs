use super::NBTError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NBTTag {
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
