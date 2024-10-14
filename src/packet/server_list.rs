use anyhow::Result;
use serde::Serialize;

use super::{reader::PacketReader, writer::PacketWriter, Packet};

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

impl Packet for Handshake {
    const ID: i32 = 0;

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

pub struct StatusRequest {}

impl Packet for StatusRequest {
    const ID: i32 = 0;

    fn packet_read(_reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
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

impl Packet for StatusResponse {
    const ID: i32 = 0;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_string(&serde_json::to_string(self)?)?;
        Ok(())
    }
}

pub struct Ping {
    pub payload: i64,
}

impl Packet for Ping {
    const ID: i32 = 1;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            payload: reader.read_long()?,
        })
    }

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_long(self.payload)?;
        Ok(())
    }
}
