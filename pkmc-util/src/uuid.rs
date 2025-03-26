use std::{
    fmt,
    io::{Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use crate::{
    connection::{PacketDecodable, PacketEncodable},
    ReadExt,
};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct UUID(pub [u8; 16]);

impl UUID {
    pub const NULL: UUID = UUID([0u8; 16]);

    pub fn new_v7() -> Self {
        let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut value: u128 = rand::random();
        value &= !0xFFFFFFFF_FFFF_0000_0000_000000000000;
        value |= time_since_epoch.as_millis() << 80;
        value &= !0x00000000_0000_F000_F000_000000000000;
        value |= 0x00000000_0000_7000_B000_000000000000;
        Self(value.to_le_bytes())
    }
}

impl fmt::Display for UUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format each part of the UUID
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl From<u128> for UUID {
    fn from(value: u128) -> Self {
        Self(value.to_be_bytes())
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("UUIDTryFromError")]
pub struct UUIDTryFromStrError;

impl TryFrom<&str> for UUID {
    type Error = UUIDTryFromStrError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let dashless = match (
            value.chars().nth(8).ok_or(UUIDTryFromStrError)? == '-',
            value.chars().nth(13).ok_or(UUIDTryFromStrError)? == '-',
            value.chars().nth(18).ok_or(UUIDTryFromStrError)? == '-',
            value.chars().nth(23).ok_or(UUIDTryFromStrError)? == '-',
        ) {
            (true, true, true, true) => value.replacen('-', "", 4),
            (false, false, false, false) => value.to_owned(),
            _ => {
                return Err(UUIDTryFromStrError);
            }
        };
        if dashless.len() != 32
            || dashless
                .chars()
                .any(|char| !("0123456789abcdefABCDEF".contains(char)))
        {
            return Err(UUIDTryFromStrError);
        }
        u128::from_str_radix(&dashless, 16)
            .map_err(|_| UUIDTryFromStrError)
            .map(|v| v.into())
    }
}

impl PacketEncodable for &UUID {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&self.0)
    }
}

impl PacketDecodable for UUID {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        Ok(UUID(reader.read_const()?))
    }
}

#[cfg(test)]
mod test {
    use crate::{UUIDTryFromStrError, UUID};

    #[test]
    fn test() {
        let uuid = UUID([
            248, 29, 79, 174, 125, 236, 17, 208, 167, 101, 0, 160, 201, 30, 107, 246,
        ]);
        assert_eq!(
            UUID::try_from("f81d4fae-7dec-11d0-a765-00a0c91e6bf6"),
            Ok(uuid),
        );
        assert_eq!(UUID::try_from("f81d4fae7dec11d0a76500a0c91e6bf6"), Ok(uuid));
        // Hello inserted in middle of UUID
        assert_eq!(
            UUID::try_from("f81d4fae7HELLOd0a76500a0c91e6bf6"),
            Err(UUIDTryFromStrError),
        );
        // Invalid dash position
        assert_eq!(
            UUID::try_from("f81d4fa-e7dec-11d0-a765-00a0c91e6bf6"),
            Err(UUIDTryFromStrError),
        );
        // UUID is too big
        assert_eq!(
            UUID::try_from("f81d4fae-7dec-11d0-a765-00a0c901e6bf6"),
            Err(UUIDTryFromStrError),
        );
    }
}
