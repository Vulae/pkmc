use std::io::Read;

use pkmc_util::{
    packet::{ConnectionError, PacketDecoder as _, ServerboundPacket},
    ReadExt as _,
};

#[derive(Debug)]
pub enum IntentionNextState {
    Status,
    Login,
    Transfer,
}

impl TryFrom<i32> for IntentionNextState {
    type Error = ConnectionError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(IntentionNextState::Status),
            2 => Ok(IntentionNextState::Login),
            3 => Ok(IntentionNextState::Transfer),
            _ => Err(ConnectionError::Other(
                format!("Packet Intention next_state invalid value {}", value).into(),
            )),
        }
    }
}

#[derive(Debug)]
pub struct Intention {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: IntentionNextState,
}

impl ServerboundPacket for Intention {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::handshake::SERVERBOUND_INTENTION;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            protocol_version: reader.decode()?,
            server_address: reader.decode()?,
            server_port: u16::from_be_bytes(reader.read_const()?),
            next_state: IntentionNextState::try_from(reader.decode::<i32>()?)?,
        })
    }
}
