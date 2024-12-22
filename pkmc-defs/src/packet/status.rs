use std::io::{Read, Write};

use crate::generated;
use pkmc_packet::{
    connection::{ClientboundPacket, ConnectionError, ServerboundPacket},
    serverbound_packet_enum, WriteExtPacket,
};
use pkmc_util::read_ext::ReadExt;
use serde::Serialize;

pub struct Request;

impl ServerboundPacket for Request {
    const SERVERBOUND_ID: i32 = generated::packet::status::SERVERBOUND_MINECRAFT_STATUS_REQUEST;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Serialize)]
pub struct ResponseVersion {
    pub name: String,
    pub protocol: u64,
}

#[derive(Serialize)]
pub struct ResponsePlayers {
    pub max: u64,
    pub online: u64,
    pub sample: Vec<ResponsePlayerSample>,
}

#[derive(Serialize)]
pub struct ResponsePlayerSample {
    pub name: String,
    pub id: String,
}

#[derive(Serialize)]
pub struct ResponseDescription {
    pub text: String,
}

#[derive(Serialize)]
pub struct Response {
    pub version: ResponseVersion,
    pub players: Option<ResponsePlayers>,
    pub description: Option<ResponseDescription>,
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: bool,
}

impl ClientboundPacket for Response {
    const CLIENTBOUND_ID: i32 = generated::packet::status::CLIENTBOUND_MINECRAFT_STATUS_RESPONSE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_string(
            &serde_json::to_string(self).map_err(|err| ConnectionError::Other(Box::new(err)))?,
        )?;
        Ok(())
    }
}

pub struct Ping {
    pub payload: i64,
}

impl ServerboundPacket for Ping {
    const SERVERBOUND_ID: i32 = generated::packet::status::SERVERBOUND_MINECRAFT_PING_REQUEST;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            payload: i64::from_be_bytes(reader.read_const()?),
        })
    }
}

impl ClientboundPacket for Ping {
    const CLIENTBOUND_ID: i32 = generated::packet::status::CLIENTBOUND_MINECRAFT_PONG_RESPONSE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.payload.to_be_bytes())?;
        Ok(())
    }
}

serverbound_packet_enum!(pub StatusPacket;
    Request, Request;
    Ping, Ping;
);
