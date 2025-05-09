use std::io::{Read, Write};

use pkmc_generated::registry::CommandArgumentType;
use pkmc_util::{
    connection::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder, ServerboundPacket,
    },
    FixedBitSet, ReadExt as _,
};

use crate::text_component::TextComponent;

#[derive(Debug)]
pub struct SystemChat {
    pub content: TextComponent,
    pub overlay: bool,
}

impl ClientboundPacket for SystemChat {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SYSTEM_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.content.to_nbt())?;
        writer.encode(self.overlay)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetActionBarText(pub TextComponent);

impl ClientboundPacket for SetActionBarText {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_ACTION_BAR_TEXT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub message: String,
    pub timestamp: i64,
    pub salt: i64,
    pub signature: Option<[u8; 256]>,
    pub message_count: i32,
    pub acknowledged: FixedBitSet<20>,
}

impl ServerboundPacket for ChatMessage {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CHAT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            message: reader.decode()?,
            timestamp: i64::from_be_bytes(reader.read_const()?),
            salt: i64::from_be_bytes(reader.read_const()?),
            signature: reader
                .decode::<bool>()?
                .then(|| reader.read_const())
                .transpose()?,
            message_count: reader.decode()?,
            acknowledged: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct DisguisedChatMessage {
    pub message: TextComponent,
    // TODO: minecraft:chat_type registry generated code enum
    pub chat_type: i32,
    pub sender_name: TextComponent,
    pub target_name: Option<TextComponent>,
}

impl ClientboundPacket for DisguisedChatMessage {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_DISGUISED_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.message.to_nbt())?;
        writer.encode(self.chat_type + 1)?;
        writer.encode(&self.sender_name.to_nbt())?;
        if let Some(target_name) = &self.target_name {
            writer.encode(true)?;
            writer.encode(&target_name.to_nbt())?;
        } else {
            writer.encode(false)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum CommandParserStringType {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

#[derive(Debug, PartialEq)]
pub enum CommandParser {
    Bool,
    Float {
        min: Option<f32>,
        max: Option<f32>,
    },
    Double {
        min: Option<f64>,
        max: Option<f64>,
    },
    Integer {
        min: Option<i32>,
        max: Option<i32>,
    },
    Long {
        min: Option<i64>,
        max: Option<i64>,
    },
    String {
        r#type: CommandParserStringType,
    },
    Entity {
        allow_multiple: bool,
        allow_players: bool,
    },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Style,
    Message,
    Nbt,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder {
        allow_multiple: bool,
    },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    Dimension,
    Gamemode,
    Time {
        min: i32,
    },
    ResourceOrTag(String),
    ResourceOrTagKey(String),
    Resource(String),
    ResourceKey(String),
    TemplateMirror,
    TemplateRotation,
    Heightmap,
    #[allow(clippy::upper_case_acronyms)]
    UUID,
}

impl CommandParser {
    fn id(&self) -> i32 {
        match self {
            Self::Bool => CommandArgumentType::BrigadierBool.to_id(),
            Self::Double { .. } => CommandArgumentType::BrigadierDouble.to_id(),
            Self::Float { .. } => CommandArgumentType::BrigadierFloat.to_id(),
            Self::Integer { .. } => CommandArgumentType::BrigadierInteger.to_id(),
            Self::Long { .. } => CommandArgumentType::BrigadierLong.to_id(),
            Self::String { .. } => CommandArgumentType::BrigadierString.to_id(),
            Self::Angle => CommandArgumentType::Angle.to_id(),
            Self::BlockPos => CommandArgumentType::BlockPos.to_id(),
            Self::BlockPredicate => CommandArgumentType::BlockPredicate.to_id(),
            Self::BlockState => CommandArgumentType::BlockState.to_id(),
            Self::Color => CommandArgumentType::Color.to_id(),
            Self::ColumnPos => CommandArgumentType::ColumnPos.to_id(),
            Self::Component => CommandArgumentType::Component.to_id(),
            Self::Dimension => CommandArgumentType::Dimension.to_id(),
            Self::Entity { .. } => CommandArgumentType::Entity.to_id(),
            Self::EntityAnchor => CommandArgumentType::EntityAnchor.to_id(),
            Self::FloatRange => CommandArgumentType::FloatRange.to_id(),
            Self::Function => CommandArgumentType::Function.to_id(),
            Self::GameProfile => CommandArgumentType::GameProfile.to_id(),
            Self::Gamemode => CommandArgumentType::Gamemode.to_id(),
            Self::Heightmap => CommandArgumentType::Heightmap.to_id(),
            Self::IntRange => CommandArgumentType::IntRange.to_id(),
            Self::ItemPredicate => CommandArgumentType::ItemPredicate.to_id(),
            Self::ItemSlot => CommandArgumentType::ItemSlot.to_id(),
            //Self::ItemSlots => CommandArgumentType::ItemSlots.to_id(),
            Self::ItemStack => CommandArgumentType::ItemStack.to_id(),
            //Self::LootModifier => CommandArgumentType::LootModifier.to_id(),
            //Self::LootPredicate => CommandArgumentType::LootPredicate.to_id(),
            //Self::LootTable => CommandArgumentType::LootTable.to_id(),
            Self::Message => CommandArgumentType::Message.to_id(),
            Self::Nbt => CommandArgumentType::NbtCompoundTag.to_id(),
            Self::NbtPath => CommandArgumentType::NbtPath.to_id(),
            Self::NbtTag => CommandArgumentType::NbtTag.to_id(),
            Self::Objective => CommandArgumentType::Objective.to_id(),
            Self::ObjectiveCriteria => CommandArgumentType::ObjectiveCriteria.to_id(),
            Self::Operation => CommandArgumentType::Operation.to_id(),
            Self::Particle => CommandArgumentType::Particle.to_id(),
            Self::Resource(..) => CommandArgumentType::Resource.to_id(),
            Self::ResourceKey(..) => CommandArgumentType::ResourceKey.to_id(),
            Self::ResourceLocation => CommandArgumentType::ResourceLocation.to_id(),
            Self::ResourceOrTag(..) => CommandArgumentType::ResourceOrTag.to_id(),
            Self::ResourceOrTagKey(..) => CommandArgumentType::ResourceOrTagKey.to_id(),
            //Self::ResourceSelector => CommandArgumentType::ResourceSelector.to_id(),
            Self::Rotation => CommandArgumentType::Rotation.to_id(),
            Self::ScoreHolder { .. } => CommandArgumentType::ScoreHolder.to_id(),
            Self::ScoreboardSlot => CommandArgumentType::ScoreboardSlot.to_id(),
            Self::Style => CommandArgumentType::Style.to_id(),
            Self::Swizzle => CommandArgumentType::Swizzle.to_id(),
            Self::Team => CommandArgumentType::Team.to_id(),
            Self::TemplateMirror => CommandArgumentType::TemplateMirror.to_id(),
            Self::TemplateRotation => CommandArgumentType::TemplateRotation.to_id(),
            Self::Time { .. } => CommandArgumentType::Time.to_id(),
            Self::UUID => CommandArgumentType::Uuid.to_id(),
            Self::Vec2 => CommandArgumentType::Vec2.to_id(),
            Self::Vec3 => CommandArgumentType::Vec3.to_id(),
        }
    }

    fn write_properties(&self) -> Result<Box<[u8]>, std::io::Error> {
        let mut writer = Vec::new();
        match self {
            Self::Float { .. } => unimplemented!(),
            Self::Double { .. } => unimplemented!(),
            Self::Integer { .. } => unimplemented!(),
            Self::Long { .. } => unimplemented!(),
            Self::String { .. } => unimplemented!(),
            Self::Entity { .. } => unimplemented!(),
            Self::ScoreHolder { .. } => unimplemented!(),
            Self::Time { .. } => unimplemented!(),
            Self::ResourceOrTag(identifier) => writer.encode(identifier)?,
            Self::ResourceOrTagKey(identifier) => writer.encode(identifier)?,
            Self::Resource(identifier) => writer.encode(identifier)?,
            Self::ResourceKey(identifier) => writer.encode(identifier)?,
            _ => {}
        }
        Ok(writer.into_boxed_slice())
    }
}

#[derive(Debug, PartialEq)]
pub enum CommandNode {
    Literal {
        name: String,
        children: Vec<CommandNode>,
    },
    Argument {
        name: String,
        children: Vec<CommandNode>,
        parser: CommandParser,
    },
}

impl CommandNode {
    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &CommandNode> + '_> {
        let children = match self {
            CommandNode::Literal { children, .. } => children,
            CommandNode::Argument { children, .. } => children,
        };
        Box::new(
            children
                .iter()
                .flat_map(move |child| std::iter::once(child).chain(child.iter_nodes())),
        )
    }
}

#[derive(Debug)]
pub struct CommandsTree {
    pub children: Vec<CommandNode>,
}

impl CommandsTree {
    fn iter_nodes(&self) -> impl Iterator<Item = &CommandNode> {
        self.children
            .iter()
            .flat_map(|child| std::iter::once(child).chain(child.iter_nodes()))
    }
}

impl ClientboundPacket for CommandsTree {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_COMMANDS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // https://minecraft.wiki/w/Java_Edition_protocol/Command_data
        //
        // Is executable flag doesn't seem to actually do anything from what I tested with.
        // Has redirects will not work with the way this is all structured.
        // And suggestions don't make much sense with how this is all structured.
        //
        // All command stuff just probably needs to be put into a command manager thingy so all
        // that extra stuff can be easily handled, unlike from multiple implementations of command
        // things.

        struct RawNode {
            flags: u8,
            children: Vec<i32>,
            name: Option<String>,
            parser: Option<(i32, Box<[u8]>)>,
        }

        let ids: Vec<&CommandNode> = self.iter_nodes().collect();

        let mut raw: Vec<RawNode> = Vec::new();

        raw.push(RawNode {
            flags: 0,
            children: self
                .children
                .iter()
                .map(|child| {
                    ids.iter()
                        .enumerate()
                        .find_map(|(i, v)| (child == *v).then_some(i as i32 + 1))
                        .unwrap()
                })
                .collect(),
            name: None,
            parser: None,
        });

        self.iter_nodes().for_each(|node| {
            raw.push(RawNode {
                flags: match node {
                    CommandNode::Literal { .. } => 1,
                    CommandNode::Argument { .. } => 2,
                },
                children: match node {
                    CommandNode::Literal { children, .. } => children,
                    CommandNode::Argument { children, .. } => children,
                }
                .iter()
                .map(|child| {
                    ids.iter()
                        .enumerate()
                        .find_map(|(i, v)| (child == *v).then_some(i as i32 + 1))
                        .unwrap()
                })
                .collect(),
                name: Some(match node {
                    CommandNode::Literal { name, .. } => name.clone(),
                    CommandNode::Argument { name, .. } => name.clone(),
                }),
                parser: if let CommandNode::Argument { parser, .. } = node {
                    Some((parser.id(), parser.write_properties().unwrap()))
                } else {
                    None
                },
            })
        });

        writer.encode(raw.len() as i32)?;
        for node in raw.into_iter() {
            writer.write_all(&node.flags.to_be_bytes())?;
            writer.encode(node.children.len() as i32)?;
            node.children
                .into_iter()
                .try_for_each(|child| writer.encode(child))?;
            if let Some(name) = node.name {
                writer.encode(&name)?;
            }
            if let Some((parser_id, parser_props)) = node.parser {
                writer.encode(parser_id)?;
                writer.write_all(&parser_props)?;
            }
        }
        writer.encode(0)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ChatCommand(pub String);

impl ServerboundPacket for ChatCommand {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CHAT_COMMAND;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(reader.decode()?))
    }
}
