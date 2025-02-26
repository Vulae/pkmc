use std::collections::HashMap;

use crate::nbt::NBTList;

use super::{NBTError, NBT};

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
            NBT::List(list) => serde_json::Value::from_iter(list.into_iter()),
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
