use anyhow::{anyhow, Result};

use crate::{nbt::NBT, uuid::UUID};

use super::{reader::PacketReader, Packet, Position};

pub struct LoginStart {
    pub name: String,
    pub uuid: UUID,
}

impl Packet for LoginStart {
    const ID: i32 = 0;

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

pub struct LoginSuccessProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

pub struct LoginSuccess {
    pub uuid: UUID,
    pub name: String,
    pub properties: Vec<LoginSuccessProperty>,
    // NOTE: Remove this for 1.21.2! https://wiki.vg/Protocol#Login_Success
    pub strict_error_handling: bool,
}

impl Packet for LoginSuccess {
    const ID: i32 = 2;

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

pub struct LoginAcknowledged;

impl Packet for LoginAcknowledged {
    const ID: i32 = 3;

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

impl Packet for LoginConfigurationPluginMessage {
    const ID: i32 = 2;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LoginConfigurationClientInformationChatMode {
    Enabled,
    CommandsOnly,
    Hidden,
}

impl TryFrom<i32> for LoginConfigurationClientInformationChatMode {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(LoginConfigurationClientInformationChatMode::Enabled),
            1 => Ok(LoginConfigurationClientInformationChatMode::CommandsOnly),
            2 => Ok(LoginConfigurationClientInformationChatMode::Hidden),
            _ => Err(anyhow!(
                "Could not try from {} for LoginConfigurationClientInformationChatMode",
                value
            )),
        }
    }
}

#[derive(Debug)]
pub struct LoginConfigurationClientInformation {
    pub locale: String,
    pub view_distance: i8,
    pub chat_mode: LoginConfigurationClientInformationChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub left_handed: bool,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

impl Packet for LoginConfigurationClientInformation {
    const ID: i32 = 0;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            locale: reader.read_string()?,
            view_distance: reader.read_signed_byte()?,
            chat_mode: reader.read_var_int()?.try_into()?,
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
pub struct LoginConfigurationClientboundKnownPacks {
    pub packs: Vec<LoginConfigurationKnownPack>,
}

impl Packet for LoginConfigurationClientboundKnownPacks {
    const ID: i32 = 14;

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

#[derive(Debug)]
pub struct LoginConfigurationServerboundKnownPacks {
    pub packs: Vec<LoginConfigurationKnownPack>,
}

impl Packet for LoginConfigurationServerboundKnownPacks {
    const ID: i32 = 7;

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

impl Packet for LoginConfigurationRegistryData {
    const ID: i32 = 7;

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

pub struct LoginConfigurationFinish;

impl Packet for LoginConfigurationFinish {
    const ID: i32 = 3;

    fn packet_read(_reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn packet_write(&self, _writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        Ok(())
    }
}

pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimensions: Vec<String>,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub dimension_type: i32,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death: Option<(String, Position)>,
    pub portal_cooldown: i32,
    pub enforces_secure_chat: bool,
}

impl Packet for LoginPlay {
    const ID: i32 = 43;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_int(self.entity_id)?;
        writer.write_boolean(self.is_hardcore)?;
        writer.write_var_int(self.dimensions.len() as i32)?;
        for dimension in self.dimensions.iter() {
            writer.write_string(dimension)?;
        }
        writer.write_var_int(self.max_players)?;
        writer.write_var_int(self.view_distance)?;
        writer.write_var_int(self.simulation_distance)?;
        writer.write_boolean(self.reduced_debug_info)?;
        writer.write_boolean(self.enable_respawn_screen)?;
        writer.write_boolean(self.do_limited_crafting)?;
        writer.write_var_int(self.dimension_type)?;
        writer.write_string(&self.dimension_name)?;
        writer.write_long(self.hashed_seed)?;
        writer.write_unsigned_byte(self.game_mode)?;
        writer.write_signed_byte(self.previous_game_mode)?;
        writer.write_boolean(self.is_debug)?;
        writer.write_boolean(self.is_flat)?;
        if let Some(death) = &self.death {
            writer.write_boolean(true)?;
            writer.write_string(&death.0)?;
            writer.write_position(death.1)?;
        } else {
            writer.write_boolean(false)?;
        }
        writer.write_var_int(self.portal_cooldown)?;
        writer.write_boolean(self.enforces_secure_chat)?;

        Ok(())
    }
}
