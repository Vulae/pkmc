use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
};

use pkmc_util::{
    connection::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _,
        ServerboundPacket,
    },
    Position, ReadExt as _, UUID,
};

use crate::text_component::TextComponent;

#[derive(Debug)]
pub struct Login {
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
    pub sea_level: i32,
    pub enforces_secure_chat: bool,
}

impl ClientboundPacket for Login {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LOGIN;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.entity_id.to_be_bytes())?;
        writer.encode(self.is_hardcore)?;
        writer.encode(self.dimensions.len() as i32)?;
        self.dimensions.iter().try_for_each(|s| writer.encode(s))?;
        writer.encode(self.max_players)?;
        writer.encode(self.view_distance)?;
        writer.encode(self.simulation_distance)?;
        writer.encode(self.reduced_debug_info)?;
        writer.encode(self.enable_respawn_screen)?;
        writer.encode(self.do_limited_crafting)?;
        writer.encode(self.dimension_type)?;
        writer.encode(&self.dimension_name)?;
        writer.write_all(&self.hashed_seed.to_be_bytes())?;
        writer.write_all(&self.game_mode.to_be_bytes())?;
        writer.write_all(&self.previous_game_mode.to_be_bytes())?;
        writer.encode(self.is_debug)?;
        writer.encode(self.is_flat)?;
        if let Some(death) = &self.death {
            writer.encode(true)?;
            writer.encode(&death.0)?;
            writer.encode(&death.1)?;
        } else {
            writer.encode(false)?;
        }
        writer.encode(self.portal_cooldown)?;
        writer.encode(self.sea_level)?;
        writer.encode(self.enforces_secure_chat)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Disconnect(pub TextComponent);

impl ClientboundPacket for Disconnect {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_DISCONNECT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameEvent {
    ChangeGamemode(u8),
    StartWaitingForLevelChunks,
}

impl ClientboundPacket for GameEvent {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_GAME_EVENT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        match self {
            GameEvent::ChangeGamemode(gamemode) => {
                writer.write_all(&3u8.to_be_bytes())?;
                writer.write_all(&(*gamemode as f32).to_be_bytes())?;
            }
            GameEvent::StartWaitingForLevelChunks => {
                writer.write_all(&13u8.to_be_bytes())?;
                writer.write_all(&0.0f32.to_be_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PlayerLoaded;

impl ServerboundPacket for PlayerLoaded {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_LOADED;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug, Default)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pos_relative: bool,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub delta_relative: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub angle_relative: bool,
    pub rotate_delta: bool,
    pub teleport_id: i32,
}

impl ClientboundPacket for PlayerPosition {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_POSITION;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.teleport_id)?;
        writer.write_all(&self.x.to_be_bytes())?;
        writer.write_all(&self.y.to_be_bytes())?;
        writer.write_all(&self.z.to_be_bytes())?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.write_all(
            &(if self.pos_relative { 0b111i32 } else { 0 }
                | if self.angle_relative { 0b11000 } else { 0 }
                | if self.delta_relative { 0b11100000 } else { 0 }
                | if self.rotate_delta { 0b100000000 } else { 0 })
            .to_be_bytes(),
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AcceptTeleportation {
    pub teleport_id: i32,
}

impl ServerboundPacket for AcceptTeleportation {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_ACCEPT_TELEPORTATION;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerPosRot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerPosRot {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_POS_ROT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f64::from_be_bytes(reader.read_const()?),
            y: f64::from_be_bytes(reader.read_const()?),
            z: f64::from_be_bytes(reader.read_const()?),
            yaw: f32::from_be_bytes(reader.read_const()?),
            pitch: f32::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerPos {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_POS;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f64::from_be_bytes(reader.read_const()?),
            y: f64::from_be_bytes(reader.read_const()?),
            z: f64::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerRot {
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerRot {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_ROT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            yaw: f32::from_be_bytes(reader.read_const()?),
            pitch: f32::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerStatusOnly {
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerStatusOnly {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_STATUS_ONLY;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct ClientTickEnd;

impl ServerboundPacket for ClientTickEnd {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CLIENT_TICK_END;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct PlayerInput {
    pub flags: u8,
}

impl ServerboundPacket for PlayerInput {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_INPUT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct PlayerAbilities_Clientbound {
    pub flags: u8,
    pub flying_speed: f32,
    pub field_of_view_modifier: f32,
}

impl ClientboundPacket for PlayerAbilities_Clientbound {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_ABILITIES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.flying_speed.to_be_bytes())?;
        writer.write_all(&self.field_of_view_modifier.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct PlayerAbilities_Serverbound {
    pub flags: u8,
}

impl ServerboundPacket for PlayerAbilities_Serverbound {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_ABILITIES;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCommandAction {
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse,
    StopJumpWithHorse,
    OpenVehicleInventory,
    StartFlyingWithElytra,
}

impl TryFrom<i32> for PlayerCommandAction {
    type Error = ConnectionError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PlayerCommandAction::StartSneaking),
            1 => Ok(PlayerCommandAction::StopSneaking),
            2 => Ok(PlayerCommandAction::LeaveBed),
            3 => Ok(PlayerCommandAction::StartSprinting),
            4 => Ok(PlayerCommandAction::StopSprinting),
            5 => Ok(PlayerCommandAction::StartJumpWithHorse),
            6 => Ok(PlayerCommandAction::StopJumpWithHorse),
            7 => Ok(PlayerCommandAction::OpenVehicleInventory),
            8 => Ok(PlayerCommandAction::StartFlyingWithElytra),
            _ => Err(ConnectionError::Other(
                "packet::play::PlayerActionCommand invalid varint value".into(),
            )),
        }
    }
}

#[derive(Debug)]
pub struct PlayerCommand {
    pub entity_id: i32,
    pub action: PlayerCommandAction,
    pub jump_boost: i32,
}

impl ServerboundPacket for PlayerCommand {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_COMMAND;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            entity_id: reader.decode()?,
            action: PlayerCommandAction::try_from(reader.decode::<i32>()?)?,
            jump_boost: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct SetCarriedItem(pub u16);

impl ServerboundPacket for SetCarriedItem {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_SET_CARRIED_ITEM;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(u16::from_be_bytes(reader.read_const()?)))
    }
}

#[derive(Debug)]
pub struct SwingArm(pub bool);

impl ServerboundPacket for SwingArm {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_SWING;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(match reader.decode::<i32>()? {
            0 => false,
            1 => true,
            _ => return Err(ConnectionError::Other("Invalid swing arm.".into())),
        }))
    }
}

#[derive(Debug)]
pub enum PlayerInfoUpdateAction {
    AddPlayer {
        name: String,
        properties: HashMap<String, (String, Option<String>)>,
    },
    // TODO:
    InitializeChat,
    UpdateGamemode(i32),
    UpdateListed(bool),
    UpdateLatency(i32),
    UpdateDisplayName(Option<TextComponent>),
    UpdateListPriority(i32),
    UpdateHat(bool),
}

impl PlayerInfoUpdateAction {
    fn flag(&self) -> u8 {
        match self {
            PlayerInfoUpdateAction::AddPlayer { .. } => 0x01,
            PlayerInfoUpdateAction::InitializeChat => 0x02,
            PlayerInfoUpdateAction::UpdateGamemode(_) => 0x04,
            PlayerInfoUpdateAction::UpdateListed(_) => 0x08,
            PlayerInfoUpdateAction::UpdateLatency(_) => 0x10,
            PlayerInfoUpdateAction::UpdateDisplayName(_) => 0x20,
            PlayerInfoUpdateAction::UpdateListPriority(_) => 0x40,
            PlayerInfoUpdateAction::UpdateHat(_) => 0x80,
        }
    }
}

#[derive(Debug)]
pub struct PlayerInfoUpdate(pub HashMap<UUID, Vec<PlayerInfoUpdateAction>>);

impl ClientboundPacket for PlayerInfoUpdate {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_INFO_UPDATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Protocol#Player_Info_Update
        let Some(first) = self.0.values().next() else {
            //return Err(ConnectionError::Other(
            //    "PlayerInfoUpdate cannot be empty.".into(),
            //));
            writer.write_all(&0u8.to_be_bytes())?;
            writer.encode(0)?;
            return Ok(());
        };
        let actions_flags = first.iter().fold(0, |f, a| f | a.flag());
        writer.write_all(&actions_flags.to_be_bytes())?;

        writer.encode(self.0.len() as i32)?;

        for (uuid, actions) in &self.0 {
            writer.encode(uuid)?;

            // Validate flags
            if Some(actions_flags)
                != actions
                    .iter()
                    .map(|a| a.flag())
                    .try_fold(0, |f, a| (f & a == 0).then_some(f | a))
            {
                return Err(ConnectionError::Other(
                    "PlayerInfoUpdate all player action types do not match.".into(),
                ));
            }

            let mut sorted_actions = actions.iter().collect::<Vec<_>>();
            sorted_actions.sort_by_key(|a| a.flag());

            for action in sorted_actions {
                match action {
                    PlayerInfoUpdateAction::AddPlayer { name, properties } => {
                        writer.encode(name)?;
                        writer.encode(properties.len() as i32)?;
                        for (key, (value, signature)) in properties {
                            writer.encode(key)?;
                            writer.encode(value)?;
                            writer.encode(signature.as_ref())?;
                        }
                    }
                    PlayerInfoUpdateAction::InitializeChat => {
                        writer.encode(false)?;
                    }
                    PlayerInfoUpdateAction::UpdateGamemode(gamemode) => {
                        writer.encode(*gamemode)?;
                    }
                    PlayerInfoUpdateAction::UpdateListed(listed) => {
                        writer.encode(*listed)?;
                    }
                    PlayerInfoUpdateAction::UpdateLatency(latency) => {
                        writer.encode(*latency)?;
                    }
                    PlayerInfoUpdateAction::UpdateDisplayName(display_name) => {
                        writer.encode(display_name.as_ref().map(|v| v.to_nbt()).as_ref())?;
                    }
                    PlayerInfoUpdateAction::UpdateListPriority(list_priority) => {
                        writer.encode(*list_priority)?;
                    }
                    PlayerInfoUpdateAction::UpdateHat(hat) => {
                        writer.encode(*hat)?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct PlayerInfoRemove(pub HashSet<UUID>);

impl ClientboundPacket for PlayerInfoRemove {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_INFO_REMOVE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0.len() as i32)?;
        for uuid in &self.0 {
            writer.encode(uuid)?;
        }
        Ok(())
    }
}
