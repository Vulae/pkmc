// I have no idea how serde works, I pretty much just stole all the code from
// https://github.com/PistonDevelopers/hematite_nbt/blob/master/src/de.rs

use std::fmt::Display;

use serde::{
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};

use crate::nbt::NBTError;

use super::NBT;

struct NBTDeserializer(NBT);

impl serde::de::Error for NBTError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::DeserializeError(msg.to_string())
    }
}

struct NBTListVisitor<L: Iterator<Item = NBT>>(L);

impl<'de, L: Iterator<Item = NBT>> SeqAccess<'de> for NBTListVisitor<L> {
    type Error = NBTError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.0
            .next()
            .map(|next| seed.deserialize(NBTDeserializer(next)))
            .transpose()
    }
}

struct NBTCompoundVisitor<M: Iterator<Item = (String, NBT)>> {
    map: M,
    stored_value: Option<NBT>,
}

impl<'de, M: Iterator<Item = (String, NBT)>> MapAccess<'de> for NBTCompoundVisitor<M> {
    type Error = NBTError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.stored_value.is_some() {
            panic!();
        }
        let Some((key, value)) = self.map.next() else {
            return Ok(None);
        };
        self.stored_value = Some(value);
        Ok(Some(seed.deserialize(NBTDeserializer(NBT::String(key)))?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let Some(value) = self.stored_value.take() else {
            panic!();
        };
        seed.deserialize(NBTDeserializer(value))
    }
}

impl<'de> Deserializer<'de> for NBTDeserializer {
    type Error = NBTError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            NBT::Byte(byte) => visitor.visit_i8(byte),
            NBT::Short(short) => visitor.visit_i16(short),
            NBT::Int(int) => visitor.visit_i32(int),
            NBT::Long(long) => visitor.visit_i64(long),
            NBT::Float(float) => visitor.visit_f32(float),
            NBT::Double(double) => visitor.visit_f64(double),
            NBT::String(string) => visitor.visit_string(string),
            NBT::List(list) => visitor.visit_seq(NBTListVisitor(list.into_iter())),
            NBT::Compound(compound) => visitor.visit_map(NBTCompoundVisitor {
                map: compound.into_iter(),
                stored_value: None,
            }),
            NBT::ByteArray(byte_array) => {
                visitor.visit_seq(NBTListVisitor(byte_array.iter().map(|v| NBT::Byte(*v))))
            }
            NBT::IntArray(int_array) => {
                visitor.visit_seq(NBTListVisitor(int_array.iter().map(|v| NBT::Int(*v))))
            }
            NBT::LongArray(long_array) => {
                visitor.visit_seq(NBTListVisitor(long_array.iter().map(|v| NBT::Long(*v))))
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

pub fn from_nbt<T>(nbt: NBT) -> Result<T, NBTError>
where
    T: DeserializeOwned,
{
    T::deserialize(NBTDeserializer(nbt))
}
