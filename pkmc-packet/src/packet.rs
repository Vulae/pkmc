use std::io::{Read, Write};

use crate::connection::ConnectionError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Box<[u8]>,
}

impl RawPacket {
    pub fn new(id: i32, data: Box<[u8]>) -> Self {
        Self { id, data }
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
