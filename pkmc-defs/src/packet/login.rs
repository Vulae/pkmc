use std::io::{Read, Write};

use pkmc_util::{
    connection::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _,
        ServerboundPacket,
    },
    serverbound_packet_enum, ReadExt as _, UUID,
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
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Box<[u8]>,
    pub verify_token: Box<[u8]>,
    pub should_authenticate: bool,
}

impl ClientboundPacket for EncryptionRequest {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::login::CLIENTBOUND_HELLO;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        if self.server_id.len() > 20 {
            return Err(ConnectionError::Other(
                "EncryptionRequest packet server_id length > 20".into(),
            ));
        }
        writer.encode(&self.server_id)?;
        writer.encode(self.public_key.len() as i32)?;
        writer.write_all(&self.public_key)?;
        writer.encode(self.verify_token.len() as i32)?;
        writer.write_all(&self.verify_token)?;
        writer.encode(self.should_authenticate)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EncryptionResponse {
    pub shared_secret: [u8; 128],
    pub verify_token: [u8; 128],
}

impl ServerboundPacket for EncryptionResponse {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::login::SERVERBOUND_KEY;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            shared_secret: {
                if reader.decode::<i32>()? != 128 {
                    return Err(ConnectionError::Other(
                        "EncryptionResponse packet invalid shared_secret length".into(),
                    ));
                }
                reader.read_const()?
            },
            verify_token: {
                if reader.decode::<i32>()? != 128 {
                    return Err(ConnectionError::Other(
                        "EncryptionResponse packet invalid verify_token length".into(),
                    ));
                }
                reader.read_const()?
            },
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
    EncryptionResponse, EncryptionResponse;
    Acknowledged, Acknowledged;
);
