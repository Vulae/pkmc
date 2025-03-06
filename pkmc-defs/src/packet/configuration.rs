use std::{
    collections::HashMap,
    io::{Read, Write},
};

use pkmc_util::{
    nbt::NBT,
    packet::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _,
        ServerboundPacket,
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
        match reader.decode::<String>()? {
            channel if &channel == "minecraft:brand" => Ok(CustomPayload::Brand(reader.decode()?)),
            channel => Ok(CustomPayload::Unknown {
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
                writer.encode(channel)?;
                writer.write_all(data)?;
            }
            CustomPayload::Brand(brand) => {
                writer.encode(&"minecraft:brand".to_owned())?;
                writer.encode(brand)?;
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
            locale: reader.decode()?,
            view_distance: i8::from_be_bytes(reader.read_const()?),
            chat_mode: reader.decode()?,
            chat_colors: reader.decode()?,
            displayed_skin_parts: u8::from_be_bytes(reader.read_const()?),
            // TODO: Is this correct?
            left_handed: reader.decode::<i32>()? == 0,
            enable_text_filtering: reader.decode()?,
            allow_server_listings: reader.decode()?,
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
        writer.encode(self.packs.len() as i32)?;
        for pack in self.packs.iter() {
            writer.encode(&pack.namespace)?;
            writer.encode(&pack.id)?;
            writer.encode(&pack.version)?;
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
            packs: (0..reader.decode::<i32>()?)
                .map(|_| {
                    Ok(KnownPack {
                        namespace: reader.decode()?,
                        id: reader.decode()?,
                        version: reader.decode()?,
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
        writer.encode(&self.registry_id)?;
        writer.encode(self.entries.len() as i32)?;
        for entry in self.entries.iter() {
            writer.encode(&entry.entry_id)?;
            writer.encode(entry.data.as_ref())?;
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
        writer.encode(self.registries.len() as i32)?;
        self.registries
            .iter()
            .try_for_each(|(registry_name, registry_data)| {
                writer.encode(registry_name)?;
                writer.encode(registry_data.len() as i32)?;
                registry_data.iter().try_for_each(|(tag_name, tag_ids)| {
                    writer.encode(tag_name)?;
                    writer.encode(tag_ids.len() as i32)?;
                    tag_ids.iter().try_for_each(|id| writer.encode(*id))?;
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
