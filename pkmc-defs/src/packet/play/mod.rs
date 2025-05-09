mod chat;
mod entity;
mod level;
mod particle;
mod player;

pub use chat::*;
pub use entity::*;
pub use level::*;
pub use particle::*;
pub use player::*;

serverbound_packet_enum!(pub PlayPacket;
    KeepAlive, KeepAlive;
    Ping, Ping;
    PlayerLoaded, PlayerLoaded;
    AcceptTeleportation, AcceptTeleportation;
    MovePlayerPosRot, MovePlayerPosRot;
    MovePlayerPos, MovePlayerPos;
    MovePlayerRot, MovePlayerRot;
    MovePlayerStatusOnly, MovePlayerStatusOnly;
    ClientTickEnd, ClientTickEnd;
    PlayerInput, PlayerInput;
    PlayerAbilities_Serverbound, PlayerAbilities;
    PlayerCommand, PlayerCommand;
    SetCarriedItem, SetHeldItem;
    SwingArm, SwingArm;
    UseItemOn, UseItemOn;
    PlayerAction, PlayerAction;
    ChatMessage, ChatMessage;
    ChatCommand, ChatCommand;
);

use pkmc_util::{
    connection::{ClientboundPacket, ConnectionError, PacketEncoder as _, ServerboundPacket},
    serverbound_packet_enum, ReadExt as _,
};
use std::io::{Read, Write};

use crate::text_component::TextComponent;

#[derive(Debug)]
pub struct BundleDelimiter;

impl ClientboundPacket for BundleDelimiter {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_BUNDLE_DELIMITER;

    fn packet_write(&self, _writer: impl Write) -> Result<(), ConnectionError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct KeepAlive {
    pub id: i64,
}

impl ClientboundPacket for KeepAlive {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_KEEP_ALIVE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.id.to_be_bytes())?;
        Ok(())
    }
}

impl ServerboundPacket for KeepAlive {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_KEEP_ALIVE;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: i64::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
/// The ping packet should be sent back to the client as-is asap when recieved.
pub struct Ping(i64);

impl ServerboundPacket for Ping {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PING_REQUEST;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(i64::from_be_bytes(reader.read_const()?)))
    }
}

impl ClientboundPacket for Ping {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PONG_RESPONSE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.0.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ServerLink {
    BugReport,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
    Custom(TextComponent),
}

impl ServerLink {
    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(!matches!(self, ServerLink::Custom(..)))?;
        match self {
            ServerLink::BugReport => writer.encode(0)?,
            ServerLink::CommunityGuidelines => writer.encode(1)?,
            ServerLink::Support => writer.encode(2)?,
            ServerLink::Status => writer.encode(3)?,
            ServerLink::Feedback => writer.encode(4)?,
            ServerLink::Community => writer.encode(5)?,
            ServerLink::Website => writer.encode(6)?,
            ServerLink::Forums => writer.encode(7)?,
            ServerLink::News => writer.encode(8)?,
            ServerLink::Announcements => writer.encode(9)?,
            ServerLink::Custom(text_component) => writer.encode(&text_component.to_nbt())?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ServerLinks {
    pub links: Vec<(ServerLink, String)>,
}

impl ServerLinks {
    pub fn new<S, I>(links: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = (ServerLink, S)>,
    {
        Self {
            links: links
                .into_iter()
                .map(|(link, url)| (link, url.into()))
                .collect(),
        }
    }
}

impl ClientboundPacket for ServerLinks {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SERVER_LINKS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.links.len() as i32)?;
        for (link, url) in &self.links {
            link.write(&mut writer)?;
            writer.encode(url)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetTabListHeaderAndFooter {
    pub header: Option<TextComponent>,
    pub footer: Option<TextComponent>,
}

impl ClientboundPacket for SetTabListHeaderAndFooter {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_TAB_LIST;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(
            &self
                .header
                .as_ref()
                .unwrap_or(&TextComponent::empty())
                .to_nbt(),
        )?;
        writer.encode(
            &self
                .footer
                .as_ref()
                .unwrap_or(&TextComponent::empty())
                .to_nbt(),
        )?;
        Ok(())
    }
}
