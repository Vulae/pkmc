use anyhow::Result;

use crate::{
    connection::{ClientboundPacket, ServerboundPacket},
    nbt::NBT,
    uuid::UUID,
};

use super::reader::PacketReader;

#[derive(Debug)]
pub struct LoginStart {
    pub name: String,
    pub uuid: UUID,
}

impl ServerboundPacket for LoginStart {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
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
pub struct LoginSuccessProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: UUID,
    pub name: String,
    pub properties: Vec<LoginSuccessProperty>,
    // NOTE: Remove this for 1.21.2! https://wiki.vg/Protocol#Login_Success
    pub strict_error_handling: bool,
}

impl ClientboundPacket for LoginSuccess {
    const CLIENTBOUND_ID: i32 = 0x02;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_uuid(&self.uuid)?;
        writer.write_string(&self.name)?;
        writer.write_var_int(self.properties.len() as i32)?;
        for property in self.properties.iter() {
            writer.write_string(&property.name)?;
            writer.write_string(&property.value)?;
            if let Some(signature) = &property.signature {
                writer.write_boolean(true)?;
                writer.write_string(signature)?;
            } else {
                writer.write_boolean(false)?;
            }
        }
        writer.write_boolean(self.strict_error_handling)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct LoginAcknowledged;

impl ServerboundPacket for LoginAcknowledged {
    const SERVERBOUND_ID: i32 = 0x03;

    fn packet_read(_reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug)]
pub enum LoginConfigurationPluginMessage {
    Unknown { channel: String, data: Box<[u8]> },
    Brand(String),
}

impl ServerboundPacket for LoginConfigurationPluginMessage {
    const SERVERBOUND_ID: i32 = 0x02;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        let channel = reader.read_string()?;
        let data = reader.read_vec_to_end()?;
        match channel.as_ref() {
            "minecraft:brand" => Ok(LoginConfigurationPluginMessage::Brand(String::from_utf8(
                data.to_vec(),
            )?)),
            _ => Ok(LoginConfigurationPluginMessage::Unknown { channel, data }),
        }
    }
}

impl ClientboundPacket for LoginConfigurationPluginMessage {
    const CLIENTBOUND_ID: i32 = 0x01;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        match self {
            LoginConfigurationPluginMessage::Unknown { channel, data } => {
                writer.write_string(channel)?;
                writer.write_buf(data)?;
            }
            LoginConfigurationPluginMessage::Brand(brand) => {
                writer.write_string("minecraft:brand")?;
                writer.write_buf(brand.as_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LoginConfigurationClientInformation {
    pub locale: String,
    pub view_distance: i8,
    pub chat_mode: i32,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub left_handed: bool,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

impl ServerboundPacket for LoginConfigurationClientInformation {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            locale: reader.read_string()?,
            view_distance: reader.read_signed_byte()?,
            chat_mode: reader.read_var_int()?,
            chat_colors: reader.read_boolean()?,
            displayed_skin_parts: reader.read_unsigned_byte()?,
            // TODO: Is this correct?
            left_handed: reader.read_var_int()? == 0,
            enable_text_filtering: reader.read_boolean()?,
            allow_server_listings: reader.read_boolean()?,
        })
    }
}

#[derive(Debug)]
pub struct LoginConfigurationKnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug)]
pub struct LoginConfigurationKnownPacks {
    pub packs: Vec<LoginConfigurationKnownPack>,
}

impl ClientboundPacket for LoginConfigurationKnownPacks {
    const CLIENTBOUND_ID: i32 = 0x0E;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_var_int(self.packs.len() as i32)?;
        for pack in self.packs.iter() {
            writer.write_string(&pack.namespace)?;
            writer.write_string(&pack.id)?;
            writer.write_string(&pack.version)?;
        }
        Ok(())
    }
}

impl ServerboundPacket for LoginConfigurationKnownPacks {
    const SERVERBOUND_ID: i32 = 0x07;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            packs: (0..reader.read_var_int()?)
                .map(|_| {
                    Ok(LoginConfigurationKnownPack {
                        namespace: reader.read_string()?,
                        id: reader.read_string()?,
                        version: reader.read_string()?,
                    })
                })
                .collect::<Result<Vec<_>, anyhow::Error>>()?,
        })
    }
}

#[derive(Debug)]
pub struct LoginConfigurationRegistryDataEntry {
    pub entry_id: String,
    pub data: Option<NBT>,
}

#[derive(Debug)]
pub struct LoginConfigurationRegistryData {
    pub registry_id: String,
    pub entries: Vec<LoginConfigurationRegistryDataEntry>,
}

impl ClientboundPacket for LoginConfigurationRegistryData {
    const CLIENTBOUND_ID: i32 = 0x07;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_string(&self.registry_id)?;
        writer.write_var_int(self.entries.len() as i32)?;
        for entry in self.entries.iter() {
            writer.write_string(&entry.entry_id)?;
            if let Some(data) = &entry.data {
                writer.write_boolean(true)?;
                writer.write_buf(&data.to_bytes_network()?)?;
            } else {
                writer.write_boolean(false)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LoginConfigurationFinish;

impl ClientboundPacket for LoginConfigurationFinish {
    const CLIENTBOUND_ID: i32 = 0x03;

    fn packet_write(&self, _writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        Ok(())
    }
}

impl ServerboundPacket for LoginConfigurationFinish {
    const SERVERBOUND_ID: i32 = 0x03;

    fn packet_read(_reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}
