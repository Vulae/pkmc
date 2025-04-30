use crate::nbt::NBT;

use super::NBTError;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum BestMatchingNumberType {
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

impl TryFrom<&NBT> for BestMatchingNumberType {
    type Error = NBTError;

    fn try_from(value: &NBT) -> Result<Self, Self::Error> {
        if let Some(number) = match value {
            NBT::Byte(number) => Some(*number as i64),
            NBT::Short(number) => Some(*number as i64),
            NBT::Int(number) => Some(*number as i64),
            NBT::Long(number) => Some(*number),
            _ => None,
        } {
            if i8::try_from(number).is_ok() {
                Ok(BestMatchingNumberType::Byte)
            } else if i16::try_from(number).is_ok() {
                Ok(BestMatchingNumberType::Short)
            } else if i32::try_from(number).is_ok() {
                Ok(BestMatchingNumberType::Int)
            } else {
                Ok(BestMatchingNumberType::Long)
            }
        } else if let Some(number) = match value {
            NBT::Float(number) => Some(*number as f64),
            NBT::Double(number) => Some(*number),
            _ => None,
        } {
            if ((number as f32) as f64) == number {
                Ok(BestMatchingNumberType::Float)
            } else {
                Ok(BestMatchingNumberType::Double)
            }
        } else {
            Err(NBTError::JsonCouldntConvert)
        }
    }
}

impl BestMatchingNumberType {
    pub fn convert_json_to_nbt(&self, value: &serde_json::Number) -> Result<NBT, NBTError> {
        fn do_conversion_int<T: TryFrom<i64>>(value: &serde_json::Number) -> Result<T, NBTError> {
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

    pub fn convert_nbt(&self, value: &NBT) -> Result<NBT, NBTError> {
        fn do_conversion_int<T: TryFrom<i64>>(value: &NBT) -> Result<T, NBTError> {
            match value {
                NBT::Byte(number) => Some(*number as i64),
                NBT::Short(number) => Some(*number as i64),
                NBT::Int(number) => Some(*number as i64),
                NBT::Long(number) => Some(*number),
                _ => None,
            }
            .ok_or(NBTError::JsonCouldntConvert)?
            .try_into()
            .map_err(|_| NBTError::JsonCouldntConvert)
        }

        Ok(match self {
            BestMatchingNumberType::Byte => NBT::Byte(do_conversion_int(value)?),
            BestMatchingNumberType::Short => NBT::Short(do_conversion_int(value)?),
            BestMatchingNumberType::Int => NBT::Int(do_conversion_int(value)?),
            BestMatchingNumberType::Long => NBT::Long(do_conversion_int(value)?),
            BestMatchingNumberType::Float => NBT::Float(
                match value {
                    NBT::Float(number) => Some(*number),
                    NBT::Double(number) => Some(*number as f32),
                    _ => None,
                }
                .ok_or(NBTError::JsonCouldntConvert)?,
            ),
            BestMatchingNumberType::Double => NBT::Double(
                match value {
                    NBT::Float(number) => Some(*number as f64),
                    NBT::Double(number) => Some(*number),
                    _ => None,
                }
                .ok_or(NBTError::JsonCouldntConvert)?,
            ),
        })
    }

    pub fn is_int(&self) -> bool {
        matches!(
            self,
            BestMatchingNumberType::Byte
                | BestMatchingNumberType::Short
                | BestMatchingNumberType::Int
                | BestMatchingNumberType::Long
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(
            self,
            BestMatchingNumberType::Float | BestMatchingNumberType::Double
        )
    }

    pub fn value(&self) -> u8 {
        match self {
            BestMatchingNumberType::Byte => 1,
            BestMatchingNumberType::Short => 2,
            BestMatchingNumberType::Int => 3,
            BestMatchingNumberType::Long => 4,
            BestMatchingNumberType::Float => 5,
            BestMatchingNumberType::Double => 6,
        }
    }

    pub fn rank(self, other: BestMatchingNumberType) -> Result<BestMatchingNumberType, NBTError> {
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
