use anyhow::Result;
use serde::Serialize;

use crate::connection::{ClientboundPacket, ServerboundPacket};

use super::{reader::PacketReader, writer::PacketWriter};

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

impl ServerboundPacket for Handshake {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            protocol_version: reader.read_var_int()?,
            server_address: reader.read_string()?,
            server_port: reader.read_unsigned_short()?,
            next_state: reader.read_var_int()?,
        })
    }
}

pub struct StatusRequest;

impl ServerboundPacket for StatusRequest {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(_reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
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

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_string(&serde_json::to_string(self)?)?;
        Ok(())
    }
}

pub struct Ping {
    pub payload: i64,
}

impl ServerboundPacket for Ping {
    const SERVERBOUND_ID: i32 = 0x01;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            payload: reader.read_long()?,
        })
    }
}

impl ClientboundPacket for Ping {
    const CLIENTBOUND_ID: i32 = 0x01;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_long(self.payload)?;
        Ok(())
    }
}
