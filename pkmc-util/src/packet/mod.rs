mod codec;
mod connection;
pub mod handler;
mod paletted_container;
pub mod varint;

use std::io::{Read, Write};

pub use codec::*;
pub use connection::*;
pub use paletted_container::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unsupported packet {0}: {1:#X}")]
    UnsupportedPacket(String, i32),
    #[error("Invalid raw packet ID for parser (expected: {0}, found: {1})")]
    InvalidRawPacketIDForParser(i32, i32),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Box<[u8]>,
}

impl RawPacket {
    pub fn new(id: i32, data: Box<[u8]>) -> Self {
        Self { id, data }
    }

    pub fn into_bytes(self) -> Box<[u8]> {
        let mut data = Vec::new();
        data.encode(self.id).unwrap();
        data.write_all(&self.data).unwrap();
        data.into_boxed_slice()
    }
}

pub trait ServerboundPacket {
    const SERVERBOUND_ID: i32;

    fn serverbound_id(&self) -> i32 {
        Self::SERVERBOUND_ID
    }

    fn packet_read(reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized;

    fn packet_raw_read(raw: &RawPacket) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        if raw.id != Self::SERVERBOUND_ID {
            return Err(ConnectionError::InvalidRawPacketIDForParser(
                Self::SERVERBOUND_ID,
                raw.id,
            ));
        }
        Self::packet_read(std::io::Cursor::new(&raw.data))
    }
}

pub trait ClientboundPacket {
    const CLIENTBOUND_ID: i32;

    fn clientbound_id(&self) -> i32 {
        Self::CLIENTBOUND_ID
    }

    fn packet_write(&self, writer: impl Write) -> Result<(), ConnectionError>;

    fn raw_packet(&self) -> Result<RawPacket, ConnectionError> {
        let mut raw_data = Vec::new();
        self.packet_write(&mut raw_data)?;
        Ok(RawPacket {
            id: self.clientbound_id(),
            data: raw_data.into_boxed_slice(),
        })
    }
}

#[macro_export]
macro_rules! serverbound_packet_enum {
    ($enum_vis:vis $enum_name:ident; $($type:ty, $name:ident;)*) => {
        #[derive(Debug)]
        #[allow(unused)]
        $enum_vis enum $enum_name {
            $(
                $name($type),
            )*
        }

        impl TryFrom<$crate::packet::RawPacket> for $enum_name {
            type Error = $crate::packet::ConnectionError;

            fn try_from(value: $crate::packet::RawPacket) -> std::result::Result<Self, Self::Error> {
                use $crate::packet::ServerboundPacket as _;
                let mut reader = std::io::Cursor::new(&value.data);
                match value.id {
                    $(
                        <$type>::SERVERBOUND_ID => Ok(Self::$name(<$type>::packet_read(&mut reader)?)),
                    )*
                    _ => Err(Self::Error::UnsupportedPacket(stringify!($enum_name).to_owned(), value.id)),
                }
            }
        }
    }
}
