mod bin;
mod de;
mod json;
mod tag;

use std::collections::HashMap;

pub use de::from_nbt;

use tag::NBTTag;
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
    #[error("NBT list tag mismatch {expected:?} {got:?}")]
    ListTagMismatch { expected: NBTTag, got: NBTTag },
    #[error("NBT could not write invalid list")]
    InvalidList,
    #[error("NBT error while deserializing: {0:?}")]
    DeserializeError(String),
    #[error("NBT Json couldnt convert number")]
    JsonCouldntConvertNumber,
    #[error("NBT Json could not convert")]
    JsonCouldntConvert,
    #[error("NBT Json cannot convert number array that contains both ints & floats")]
    JsonMixedIntFloatArray,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// NBTList contains NBT values that MUST be the same type.
/// The list initially doesn't have a type, pushing to an empty list will set its type and any
/// subsequent new items will be required to be the same type.
pub struct NBTList {
    tag: Option<NBTTag>,
    list: Vec<NBT>,
}

impl NBTList {
    fn tag(&self) -> Option<NBTTag> {
        self.tag
            .or_else(|| self.list.first().map(|item| item.tag()))
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn new_with_tag(tag: NBTTag) -> Self {
        Self {
            tag: Some(tag),
            list: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Returns error if new item has mismatching type from already containing items.
    pub fn push(&mut self, v: NBT) -> Result<(), NBTError> {
        if let Some(tag) = self.tag() {
            if tag != v.tag() {
                return Err(NBTError::ListTagMismatch {
                    expected: tag,
                    got: v.tag(),
                });
            }
        }
        self.list.push(v);
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&NBT> {
        self.list.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut NBT> {
        self.list.get_mut(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NBT> {
        self.list.iter()
    }

    /// NBTList::iter_mut cannot exist due to the way NBTList works.
    /// So each element will have to be mapped instead while also returning an error on mismatch.
    /// if there is a type mishmatch, it will be skipped & continue, while returning an error at
    /// the end of the map.
    pub fn try_map<F>(&mut self, mut mapper: F) -> Result<(), NBTError>
    where
        F: FnMut(NBT) -> NBT,
    {
        let mut new = NBTList::new();
        let result = self
            .list
            .drain(..)
            .map(|v| new.push(mapper(v)))
            .collect::<Vec<_>>();
        *self = new;
        result.into_iter().collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }
}

impl IntoIterator for NBTList {
    type Item = NBT;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}

impl TryFrom<Vec<NBT>> for NBTList {
    type Error = NBTError;

    fn try_from(value: Vec<NBT>) -> Result<Self, Self::Error> {
        let mut list = Self::new();
        value.into_iter().try_for_each(|v| list.push(v))?;
        Ok(list)
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
    List(NBTList),
    Compound(HashMap<String, NBT>),
    ByteArray(Box<[i8]>),
    IntArray(Box<[i32]>),
    LongArray(Box<[i64]>),
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
}
