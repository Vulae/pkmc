use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
};

use pkmc_generated::{block::Block, registry::EntityType};
use pkmc_util::{
    connection::{ClientboundPacket, ConnectionError, PacketEncoder as _},
    nbt::NBT,
    Position, Vec3, UUID,
};

use crate::text_component::TextComponent;

#[derive(Debug)]
pub struct AddEntity {
    pub id: i32,
    pub uuid: UUID,
    pub r#type: EntityType,
    pub position: Vec3<f64>,
    pub pitch: u8,
    pub yaw: u8,
    pub head_yaw: u8,
    pub data: i32,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}

impl ClientboundPacket for AddEntity {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ADD_ENTITY;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.id)?;
        writer.encode(&self.uuid)?;
        writer.encode(self.r#type.to_id())?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.head_yaw.to_be_bytes())?;
        writer.encode(self.data)?;
        writer.write_all(&self.velocity_x.to_be_bytes())?;
        writer.write_all(&self.velocity_y.to_be_bytes())?;
        writer.write_all(&self.velocity_z.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityMetadata {
    Byte(u8),
    VarInt(i32),
    VarLong(i64),
    Float(f32),
    String(String),
    TextComponent(TextComponent),
    OptionalTextComponent(Option<TextComponent>),
    /// UNIMPLEMENTED
    Slot,
    Boolean(bool),
    Rotations(f32, f32, f32),
    Position(Position),
    OptionalPosition(Option<Position>),
    // TODO: Implement enum
    Direction(i32),
    OptionalUUID(Option<UUID>),
    BlockState(Block),
    OptionalBlockState(Option<Block>),
    NBT(NBT),
    Particle(i32),
    Particles(Vec<i32>),
    VillagerData(i32, i32, i32),
    OptionalVarInt(Option<i32>),
    // TODO: Implement enum
    Pose(i32),
    CatVariant(i32),
    /// UNIMPLEMENTED
    WolfVariant(i32),
    FrogVariant(i32),
    OptionalGlobalPosition(Option<(String, Position)>),
    /// UNIMPLEMENTED
    PaintingVariant,
    /// UNIMPLEMENTED
    SnifferState(i32),
    /// UNIMPLEMENTED
    ArmadilloState(i32),
    Vector3(Vec3<f32>),
    // TEMP: Implement quaternions
    Quaternion(f32, f32, f32, f32),
}

impl EntityMetadata {
    pub fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        match self {
            EntityMetadata::Byte(byte) => {
                writer.encode(0)?;
                writer.write_all(&byte.to_be_bytes())?;
            }
            EntityMetadata::VarInt(varint) => {
                writer.encode(1)?;
                writer.encode(*varint)?;
            }
            EntityMetadata::VarLong(varlong) => {
                writer.encode(2)?;
                writer.encode(*varlong)?;
            }
            EntityMetadata::Float(float) => {
                writer.encode(3)?;
                writer.write_all(&float.to_be_bytes())?;
            }
            EntityMetadata::String(string) => {
                writer.encode(4)?;
                writer.encode(string)?;
            }
            EntityMetadata::TextComponent(text_component) => {
                writer.encode(5)?;
                writer.encode(&text_component.to_nbt())?;
            }
            EntityMetadata::OptionalTextComponent(text_component) => {
                writer.encode(6)?;
                writer.encode(text_component.as_ref().map(|v| v.to_nbt()).as_ref())?;
            }
            EntityMetadata::Slot => todo!(),
            EntityMetadata::Boolean(bool) => {
                writer.encode(8)?;
                writer.encode(*bool)?;
            }
            EntityMetadata::Rotations(rx, ry, rz) => {
                writer.encode(9)?;
                writer.write_all(&rx.to_be_bytes())?;
                writer.write_all(&ry.to_be_bytes())?;
                writer.write_all(&rz.to_be_bytes())?;
            }
            EntityMetadata::Position(position) => {
                writer.encode(10)?;
                writer.encode(position)?;
            }
            EntityMetadata::OptionalPosition(position) => {
                writer.encode(11)?;
                writer.encode(position.as_ref())?;
            }
            EntityMetadata::Direction(direction) => {
                writer.encode(12)?;
                writer.encode(*direction)?;
            }
            EntityMetadata::OptionalUUID(uuid) => {
                writer.encode(13)?;
                writer.encode(uuid.as_ref())?;
            }
            EntityMetadata::BlockState(block_state) => {
                writer.encode(14)?;
                writer.encode(block_state.into_id())?;
            }
            EntityMetadata::OptionalBlockState(block_state) => {
                writer.encode(15)?;
                writer.encode(block_state.unwrap_or(Block::Air).into_id())?;
            }
            EntityMetadata::NBT(nbt) => {
                writer.encode(16)?;
                writer.encode(nbt)?;
            }
            EntityMetadata::Particle(particle) => {
                writer.encode(17)?;
                writer.encode(*particle)?;
            }
            EntityMetadata::Particles(particles) => {
                writer.encode(18)?;
                writer.encode(particles.len() as i32)?;
                for particle in particles {
                    writer.encode(*particle)?;
                }
            }
            EntityMetadata::VillagerData(r#type, profession, level) => {
                writer.encode(19)?;
                writer.encode(*r#type)?;
                writer.encode(*profession)?;
                writer.encode(*level)?;
            }
            EntityMetadata::OptionalVarInt(var_int) => {
                writer.encode(20)?;
                if let Some(var_int) = var_int {
                    writer.encode(var_int.checked_add(1).ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Optional var int size too large",
                    ))?)?;
                } else {
                    writer.encode(0)?;
                }
            }
            EntityMetadata::Pose(pose) => {
                writer.encode(21)?;
                writer.encode(*pose)?;
            }
            EntityMetadata::CatVariant(cat_variant) => {
                writer.encode(22)?;
                writer.encode(*cat_variant)?;
            }
            EntityMetadata::WolfVariant(_wolf_variant) => todo!(),
            EntityMetadata::FrogVariant(frog_variant) => {
                writer.encode(24)?;
                writer.encode(*frog_variant)?;
            }
            EntityMetadata::OptionalGlobalPosition(global_position) => {
                writer.encode(25)?;
                if let Some((dimension, position)) = global_position {
                    writer.encode(true)?;
                    writer.encode(dimension)?;
                    writer.encode(position)?;
                } else {
                    writer.encode(false)?;
                }
            }
            EntityMetadata::PaintingVariant => todo!(),
            EntityMetadata::SnifferState(sniffer_state) => {
                writer.encode(27)?;
                writer.encode(*sniffer_state)?;
            }
            EntityMetadata::ArmadilloState(armadillo_state) => {
                writer.encode(28)?;
                writer.encode(*armadillo_state)?;
            }
            EntityMetadata::Vector3(vec3) => {
                writer.encode(29)?;
                writer.write_all(&vec3.x.to_be_bytes())?;
                writer.write_all(&vec3.y.to_be_bytes())?;
                writer.write_all(&vec3.z.to_be_bytes())?;
            }
            EntityMetadata::Quaternion(x, y, z, w) => {
                writer.encode(30)?;
                writer.write_all(&x.to_be_bytes())?;
                writer.write_all(&y.to_be_bytes())?;
                writer.write_all(&z.to_be_bytes())?;
                writer.write_all(&w.to_be_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityMetadataBundle(pub BTreeMap<u8, EntityMetadata>);

impl EntityMetadataBundle {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }
}

impl EntityMetadataBundle {
    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_flags(&mut self, flags: u8) {
        self.0.insert(0, EntityMetadata::Byte(flags));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_air_ticks(&mut self, air_ticks: i32) {
        self.0.insert(1, EntityMetadata::VarInt(air_ticks));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_custom_name(&mut self, custom_name: Option<TextComponent>) {
        self.0
            .insert(2, EntityMetadata::OptionalTextComponent(custom_name));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_is_custom_name_visible(&mut self, is_custom_name_visible: bool) {
        self.0
            .insert(3, EntityMetadata::Boolean(is_custom_name_visible));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_is_silent(&mut self, is_silent: bool) {
        self.0.insert(4, EntityMetadata::Boolean(is_silent));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_has_no_gravity(&mut self, has_no_gravity: bool) {
        self.0.insert(5, EntityMetadata::Boolean(has_no_gravity));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_pose(&mut self, pose: i32) {
        self.0.insert(6, EntityMetadata::Pose(pose));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_ticks_frozen_in_powdered_snow(&mut self, ticks_frozen_in_powdered_snow: i32) {
        self.0
            .insert(7, EntityMetadata::VarInt(ticks_frozen_in_powdered_snow));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_default() -> Self {
        let mut bundle = Self::empty();
        bundle.entity_flags(0);
        bundle.entity_air_ticks(300);
        bundle.entity_custom_name(None);
        bundle.entity_is_custom_name_visible(false);
        bundle.entity_is_silent(false);
        bundle.entity_has_no_gravity(false);
        bundle.entity_pose(0);
        bundle.entity_ticks_frozen_in_powdered_snow(0);
        bundle
    }
}

impl EntityMetadataBundle {
    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_flags(&mut self, flags: u8) {
        self.0.insert(8, EntityMetadata::Byte(flags));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_health(&mut self, health: f32) {
        self.0.insert(9, EntityMetadata::Float(health));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_potion_effect_color(&mut self, potion_effect_color: Vec<i32>) {
        self.0
            .insert(10, EntityMetadata::Particles(potion_effect_color));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_potion_effect_is_ambient(&mut self, potion_effect_is_ambient: bool) {
        self.0
            .insert(11, EntityMetadata::Boolean(potion_effect_is_ambient));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_num_arrows_in_entity(&mut self, num_arrows_in_entity: i32) {
        self.0
            .insert(12, EntityMetadata::VarInt(num_arrows_in_entity));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_num_bee_stingers_in_entity(&mut self, num_bee_stingers_in_entity: i32) {
        self.0
            .insert(13, EntityMetadata::VarInt(num_bee_stingers_in_entity));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_sleeping_bed_position(&mut self, sleeping_bed_position: Option<Position>) {
        self.0
            .insert(14, EntityMetadata::OptionalPosition(sleeping_bed_position));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_default() -> Self {
        let mut bundle = Self::entity_default();
        bundle.living_entity_flags(0);
        bundle.living_entity_health(1.0);
        bundle.living_entity_potion_effect_color(Vec::new());
        bundle.living_entity_potion_effect_is_ambient(false);
        bundle.living_entity_num_arrows_in_entity(0);
        bundle.living_entity_num_bee_stingers_in_entity(0);
        bundle.living_entity_sleeping_bed_position(None);
        bundle
    }
}

impl EntityMetadataBundle {
    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_additional_hearts(&mut self, additional_hearts: f32) {
        self.0.insert(15, EntityMetadata::Float(additional_hearts));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_score(&mut self, score: i32) {
        self.0.insert(16, EntityMetadata::VarInt(score));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_skin_parts(&mut self, skin_parts: u8) {
        self.0.insert(17, EntityMetadata::Byte(skin_parts));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_main_hand(&mut self, main_hand: u8) {
        self.0.insert(18, EntityMetadata::Byte(main_hand));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_left_parrot(&mut self, left_parrot: NBT) {
        self.0.insert(19, EntityMetadata::NBT(left_parrot));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_right_parrot(&mut self, right_parrot: NBT) {
        self.0.insert(20, EntityMetadata::NBT(right_parrot));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_default() -> Self {
        let mut bundle = Self::living_entity_default();
        bundle.player_additional_hearts(0.0);
        bundle.player_score(0);
        bundle.player_skin_parts(0);
        bundle.player_main_hand(0);
        bundle.player_left_parrot(NBT::Compound(HashMap::new()));
        bundle.player_right_parrot(NBT::Compound(HashMap::new()));
        bundle
    }
}

#[derive(Debug)]
pub struct SetEntityMetadata<'a> {
    pub entity_id: i32,
    pub metadata: &'a EntityMetadataBundle,
}

impl ClientboundPacket for SetEntityMetadata<'_> {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_ENTITY_DATA;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        for (index, data) in &self.metadata.0 {
            writer.write_all(&index.to_be_bytes())?;
            data.write(&mut writer)?;
        }
        writer.write_all(&0xFFu8.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EntityPositionSync {
    pub entity_id: i32,
    pub position: Vec3<f64>,
    pub velocity: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ClientboundPacket for EntityPositionSync {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ENTITY_POSITION_SYNC;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.velocity.x.to_be_bytes())?;
        writer.write_all(&self.velocity.y.to_be_bytes())?;
        writer.write_all(&self.velocity.z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityPos {
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityPos {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_POS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityPosRot {
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityPosRot {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityRot {
    pub entity_id: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityRot {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_ROT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetHeadRotation {
    pub entity_id: i32,
    pub yaw: u8,
}

impl ClientboundPacket for SetHeadRotation {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ROTATE_HEAD;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAnimationType {
    SwingMainArm,
    LeaveBed,
    SwingOffhand,
    CriticalEffect,
    MagicCriticalEffect,
}

impl EntityAnimationType {
    fn value(&self) -> u8 {
        match self {
            EntityAnimationType::SwingMainArm => 0,
            EntityAnimationType::LeaveBed => 1,
            EntityAnimationType::SwingOffhand => 2,
            EntityAnimationType::CriticalEffect => 3,
            EntityAnimationType::MagicCriticalEffect => 4,
        }
    }

    pub fn can_stack(&self) -> bool {
        matches!(
            self,
            EntityAnimationType::CriticalEffect | EntityAnimationType::MagicCriticalEffect
        )
    }
}

pub struct EntityAnimation {
    pub entity_id: i32,
    pub r#type: EntityAnimationType,
}

impl ClientboundPacket for EntityAnimation {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ANIMATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.r#type.value().to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RemoveEntities(pub HashSet<i32>);

impl ClientboundPacket for RemoveEntities {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_REMOVE_ENTITIES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0.len() as i32)?;
        for id in &self.0 {
            writer.encode(*id)?;
        }
        Ok(())
    }
}
