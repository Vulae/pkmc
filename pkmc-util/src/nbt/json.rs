use std::collections::HashMap;

use crate::nbt::NBTList;

use super::{number_arena::BestMatchingNumberType, NBTError, NBT};

/// https://minecraft.wiki/w/NBT_format#Conversion_from_JSON
impl TryFrom<serde_json::Value> for NBT {
    type Error = NBTError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Bool(bool) => Ok(NBT::Byte(if bool { 1 } else { 0 })),
            serde_json::Value::Number(number) => {
                Ok(BestMatchingNumberType::from(&number).convert_json_to_nbt(&number)?)
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

                let mut list = NBTList::new();

                array.iter().try_for_each(|number| {
                    list.push(arena.convert_json_to_nbt(number)?)?;
                    Ok::<(), NBTError>(())
                })?;

                Ok(NBT::List(list))
            }
            serde_json::Value::Array(array) => {
                let mut list = NBTList::new();
                array.into_iter().try_for_each(|value| {
                    let parsed = NBT::try_from(value)?;
                    list.push(parsed)?;
                    Ok::<(), NBTError>(())
                })?;
                Ok(NBT::List(list))
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
            NBT::List(list) => serde_json::Value::from_iter(list),
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
