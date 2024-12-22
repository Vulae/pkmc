use std::io::{Read, Write};

use crate::generated;
use pkmc_packet::{
    connection::{ClientboundPacket, ConnectionError, ServerboundPacket},
    serverbound_packet_enum, ReadExtPacket, WriteExtPacket,
};
use pkmc_util::UUID;

#[derive(Debug)]
pub struct Hello {
    pub name: String,
    pub uuid: UUID,
}

impl ServerboundPacket for Hello {
    const SERVERBOUND_ID: i32 = generated::packet::login::SERVERBOUND_MINECRAFT_HELLO;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            name: reader.read_string()?,
            uuid: reader.read_uuid()?,
        })
    }
}

#[derive(Debug)]
pub struct Compression {
    pub threshold: i32,
}

impl ClientboundPacket for Compression {
    const CLIENTBOUND_ID: i32 = generated::packet::login::CLIENTBOUND_MINECRAFT_LOGIN_COMPRESSION;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_varint(self.threshold)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct FinishedProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub struct Finished {
    pub uuid: UUID,
    pub name: String,
    pub properties: Vec<FinishedProperty>,
}

impl ClientboundPacket for Finished {
    const CLIENTBOUND_ID: i32 = generated::packet::login::CLIENTBOUND_MINECRAFT_LOGIN_FINISHED;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_uuid(&self.uuid)?;
        writer.write_string(&self.name)?;
        writer.write_varint(self.properties.len() as i32)?;
        for property in self.properties.iter() {
            writer.write_string(&property.name)?;
            writer.write_string(&property.value)?;
            if let Some(signature) = &property.signature {
                writer.write_bool(true)?;
                writer.write_string(signature)?;
            } else {
                writer.write_bool(false)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Acknowledged;

impl ServerboundPacket for Acknowledged {
    const SERVERBOUND_ID: i32 = generated::packet::login::SERVERBOUND_MINECRAFT_LOGIN_ACKNOWLEDGED;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

serverbound_packet_enum!(pub LoginPacket;
    Hello, Hello;
    Acknowledged, Acknowledged;
);
