use std::{
    collections::HashMap,
    io::{Read, Write},
};

use pkmc_util::{
    nbt::NBT,
    packet::{
        ClientboundPacket, ConnectionError, ReadExtPacket as _, ServerboundPacket,
        WriteExtPacket as _,
    },
    serverbound_packet_enum, ReadExt as _,
};

#[derive(Debug)]
pub enum CustomPayload {
    Unknown { channel: String, data: Box<[u8]> },
    Brand(String),
}

impl ServerboundPacket for CustomPayload {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::configuration::SERVERBOUND_CUSTOM_PAYLOAD;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        let channel = reader.read_string()?;
        match channel.as_ref() {
            "minecraft:brand" => Ok(CustomPayload::Brand(reader.read_string()?)),
            _ => Ok(CustomPayload::Unknown {
                channel,
                data: reader.read_all()?,
            }),
        }
    }
}

impl ClientboundPacket for CustomPayload {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::configuration::CLIENTBOUND_CUSTOM_PAYLOAD;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        match self {
            CustomPayload::Unknown { channel, data } => {
                writer.write_string(channel)?;
                writer.write_all(data)?;
            }
            CustomPayload::Brand(brand) => {
                writer.write_string("minecraft:brand")?;
                writer.write_string(brand)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ClientInformation {
    pub locale: String,
    pub view_distance: i8,
    pub chat_mode: i32,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub left_handed: bool,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

impl ServerboundPacket for ClientInformation {
    const SERVERBOUND_ID: i32 =
        pkmc_generated::packet::configuration::SERVERBOUND_CLIENT_INFORMATION;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            locale: reader.read_string()?,
            view_distance: i8::from_be_bytes(reader.read_const()?),
            chat_mode: reader.read_varint()?,
            chat_colors: reader.read_bool()?,
            displayed_skin_parts: u8::from_be_bytes(reader.read_const()?),
            // TODO: Is this correct?
            left_handed: reader.read_varint()? == 0,
            enable_text_filtering: reader.read_bool()?,
            allow_server_listings: reader.read_bool()?,
        })
    }
}

#[derive(Debug)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug)]
pub struct SelectKnownPacks {
    pub packs: Vec<KnownPack>,
}

impl ClientboundPacket for SelectKnownPacks {
    const CLIENTBOUND_ID: i32 =
        pkmc_generated::packet::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_varint(self.packs.len() as i32)?;
        for pack in self.packs.iter() {
            writer.write_string(&pack.namespace)?;
            writer.write_string(&pack.id)?;
            writer.write_string(&pack.version)?;
        }
        Ok(())
    }
}

impl ServerboundPacket for SelectKnownPacks {
    const SERVERBOUND_ID: i32 =
        pkmc_generated::packet::configuration::SERVERBOUND_SELECT_KNOWN_PACKS;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            packs: (0..reader.read_varint()?)
                .map(|_| {
                    Ok(KnownPack {
                        namespace: reader.read_string()?,
                        id: reader.read_string()?,
                        version: reader.read_string()?,
                    })
                })
                .collect::<Result<Vec<_>, std::io::Error>>()?,
        })
    }
}

#[derive(Debug)]
pub struct RegistryDataEntry {
    pub entry_id: String,
    pub data: Option<NBT>,
}

#[derive(Debug)]
pub struct RegistryData {
    pub registry_id: String,
    pub entries: Vec<RegistryDataEntry>,
}

impl ClientboundPacket for RegistryData {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::configuration::CLIENTBOUND_REGISTRY_DATA;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_string(&self.registry_id)?;
        writer.write_varint(self.entries.len() as i32)?;
        for entry in self.entries.iter() {
            writer.write_string(&entry.entry_id)?;
            if let Some(data) = &entry.data {
                writer.write_bool(true)?;
                writer.write_nbt(data)?;
            } else {
                writer.write_bool(false)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdateTags {
    pub registries: HashMap<String, HashMap<String, Vec<i32>>>,
}

impl ClientboundPacket for UpdateTags {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::configuration::CLIENTBOUND_UPDATE_TAGS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_varint(self.registries.len() as i32)?;
        self.registries
            .iter()
            .try_for_each(|(registry_name, registry_data)| {
                writer.write_string(registry_name)?;
                writer.write_varint(registry_data.len() as i32)?;
                registry_data.iter().try_for_each(|(tag_name, tag_ids)| {
                    writer.write_string(tag_name)?;
                    writer.write_varint(tag_ids.len() as i32)?;
                    tag_ids.iter().try_for_each(|id| writer.write_varint(*id))?;
                    Ok::<(), std::io::Error>(())
                })?;
                Ok::<(), std::io::Error>(())
            })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct FinishConfiguration;

impl ClientboundPacket for FinishConfiguration {
    const CLIENTBOUND_ID: i32 =
        pkmc_generated::packet::configuration::CLIENTBOUND_FINISH_CONFIGURATION;

    fn packet_write(&self, _writer: impl Write) -> Result<(), ConnectionError> {
        Ok(())
    }
}

impl ServerboundPacket for FinishConfiguration {
    const SERVERBOUND_ID: i32 =
        pkmc_generated::packet::configuration::SERVERBOUND_FINISH_CONFIGURATION;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

serverbound_packet_enum!(pub ConfigurationPacket;
    CustomPayload, CustomPayload;
    ClientInformation, ClientInformation;
    SelectKnownPacks, SelectKnownPacks;
    FinishConfiguration, FinishConfiguration;
);
