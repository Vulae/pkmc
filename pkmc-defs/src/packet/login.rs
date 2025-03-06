use std::io::{Read, Write};

use pkmc_util::{
    packet::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _,
        ServerboundPacket,
    },
    serverbound_packet_enum, UUID,
};

#[derive(Debug)]
pub struct Hello {
    pub name: String,
    pub uuid: UUID,
}

impl ServerboundPacket for Hello {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::login::SERVERBOUND_HELLO;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            name: reader.decode()?,
            uuid: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct Compression {
    pub threshold: i32,
}

impl ClientboundPacket for Compression {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::login::CLIENTBOUND_LOGIN_COMPRESSION;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.threshold)?;
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
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::login::CLIENTBOUND_LOGIN_FINISHED;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.uuid)?;
        writer.encode(&self.name)?;
        writer.encode(self.properties.len() as i32)?;
        for property in self.properties.iter() {
            writer.encode(&property.name)?;
            writer.encode(&property.value)?;
            writer.encode(property.signature.as_ref())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Acknowledged;

impl ServerboundPacket for Acknowledged {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::login::SERVERBOUND_LOGIN_ACKNOWLEDGED;

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
