use std::io::{Read, Write};

use pkmc_packet::{
    connection::{ClientboundPacket, PacketError, ServerboundPacket},
    ReadExtPacket as _, WriteExtPacket as _,
};
use pkmc_util::ReadExt as _;
use serde::Serialize;

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

impl ServerboundPacket for Handshake {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized,
    {
        Ok(Self {
            protocol_version: reader.read_varint()?,
            server_address: reader.read_string()?,
            server_port: u16::from_be_bytes(reader.read_const()?),
            next_state: reader.read_varint()?,
        })
    }
}

pub struct StatusRequest;

impl ServerboundPacket for StatusRequest {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(_reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Serialize)]
pub struct StatusResponseVersion {
    pub name: String,
    pub protocol: u64,
}

#[derive(Serialize)]
pub struct StatusResponsePlayers {
    pub max: u64,
    pub online: u64,
    pub sample: Vec<StatusResponsePlayerSample>,
}

#[derive(Serialize)]
pub struct StatusResponsePlayerSample {
    pub name: String,
    pub id: String,
}

#[derive(Serialize)]
pub struct StatusResponseDescription {
    pub text: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: StatusResponseVersion,
    pub players: Option<StatusResponsePlayers>,
    pub description: Option<StatusResponseDescription>,
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: bool,
}

impl ClientboundPacket for StatusResponse {
    const CLIENTBOUND_ID: i32 = 0x00;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_string(
            &serde_json::to_string(self).map_err(|err| PacketError::Other(Box::new(err)))?,
        )?;
        Ok(())
    }
}

pub struct Ping {
    pub payload: i64,
}

impl ServerboundPacket for Ping {
    const SERVERBOUND_ID: i32 = 0x01;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized,
    {
        Ok(Self {
            payload: i64::from_be_bytes(reader.read_const()?),
        })
    }
}

impl ClientboundPacket for Ping {
    const CLIENTBOUND_ID: i32 = 0x01;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_all(&self.payload.to_be_bytes())?;
        Ok(())
    }
}
