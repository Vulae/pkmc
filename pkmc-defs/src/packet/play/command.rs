use std::io::{Read, Write};

use pkmc_generated::registry::CommandArgumentType;
use pkmc_util::connection::{
    ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _, ServerboundPacket,
};

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

#[derive(Debug, PartialEq)]
pub enum CommandNodeParser {
    Literal(String),
    Bool,
    Float { min: Option<f32>, max: Option<f32> },
    Double { min: Option<f32>, max: Option<f32> },
    Int { min: Option<i32>, max: Option<i32> },
    Long { min: Option<i64>, max: Option<i64> },
    BlockPosition,
    ResourceKey(String),
    Vec3,
}

// Is node executable not implemented because it doesn't seem to have an actual effect to the client?
#[derive(Debug, PartialEq)]
pub struct CommandNode {
    parser: CommandNodeParser,
    children: Vec<CommandNode>,
}

impl CommandNode {
    pub fn new(parser: CommandNodeParser) -> Self {
        Self {
            parser,
            children: Vec::new(),
        }
    }

    pub fn with_child(mut self, child: CommandNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn new_literal<S: ToString>(literal: S) -> Self {
        Self::new(CommandNodeParser::Literal(literal.to_string()))
    }
}

struct RawNode {
    flags: u8,
    children: Vec<i32>,
    name: Option<String>,
    parser: Option<(CommandArgumentType, Box<[u8]>)>,
}

impl CommandNode {
    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &CommandNode> + '_> {
        Box::new(
            self.children
                .iter()
                .flat_map(|child| std::iter::once(child).chain(child.iter_nodes())),
        )
    }

    fn to_raw_node(&self, ids: &[&CommandNode]) -> Result<RawNode, std::io::Error> {
        let children_ids: Vec<i32> = self
            .children
            .iter()
            .map(|child| {
                ids.iter()
                    .enumerate()
                    .find_map(|(i, v)| (child == *v).then_some(i as i32))
                    .unwrap()
            })
            .collect();

        if let CommandNodeParser::Literal(literal) = &self.parser {
            return Ok(RawNode {
                flags: 1,
                children: children_ids,
                name: Some(literal.clone()),
                parser: None,
            });
        }

        let mut parser_data = Vec::new();
        let parser_type = match &self.parser {
            CommandNodeParser::Literal(..) => unreachable!(),
            CommandNodeParser::Bool => CommandArgumentType::BrigadierBool,
            CommandNodeParser::Float { min, max } => {
                parser_data.write_all(
                    &(if min.is_some() { 0x01u8 } else { 0 }
                        | if max.is_some() { 0x02 } else { 0 })
                    .to_be_bytes(),
                )?;
                if let Some(min) = min {
                    parser_data.write_all(&min.to_be_bytes())?;
                }
                if let Some(max) = max {
                    parser_data.write_all(&max.to_be_bytes())?;
                }
                CommandArgumentType::BrigadierFloat
            }
            CommandNodeParser::Double { min, max } => {
                parser_data.write_all(
                    &(if min.is_some() { 0x01u8 } else { 0 }
                        | if max.is_some() { 0x02 } else { 0 })
                    .to_be_bytes(),
                )?;
                if let Some(min) = min {
                    parser_data.write_all(&min.to_be_bytes())?;
                }
                if let Some(max) = max {
                    parser_data.write_all(&max.to_be_bytes())?;
                }
                CommandArgumentType::BrigadierDouble
            }
            CommandNodeParser::Int { min, max } => {
                parser_data.write_all(
                    &(if min.is_some() { 0x01u8 } else { 0 }
                        | if max.is_some() { 0x02 } else { 0 })
                    .to_be_bytes(),
                )?;
                if let Some(min) = min {
                    parser_data.write_all(&min.to_be_bytes())?;
                }
                if let Some(max) = max {
                    parser_data.write_all(&max.to_be_bytes())?;
                }
                CommandArgumentType::BrigadierInteger
            }
            CommandNodeParser::Long { min, max } => {
                parser_data.write_all(
                    &(if min.is_some() { 0x01u8 } else { 0 }
                        | if max.is_some() { 0x02 } else { 0 })
                    .to_be_bytes(),
                )?;
                if let Some(min) = min {
                    parser_data.write_all(&min.to_be_bytes())?;
                }
                if let Some(max) = max {
                    parser_data.write_all(&max.to_be_bytes())?;
                }
                CommandArgumentType::BrigadierLong
            }
            CommandNodeParser::BlockPosition => CommandArgumentType::BlockPos,
            CommandNodeParser::ResourceKey(resource_key) => {
                parser_data.encode(resource_key)?;
                CommandArgumentType::ResourceKey
            }
            CommandNodeParser::Vec3 => CommandArgumentType::Vec3,
        };

        Ok(RawNode {
            flags: 2,
            children: children_ids,
            name: Some("".to_owned()),
            parser: Some((parser_type, parser_data.into_boxed_slice())),
        })
    }
}

#[derive(Debug)]
pub struct CommandDefinitions<'a> {
    pub defs: &'a [CommandNode],
}

impl CommandDefinitions<'_> {
    fn iter_nodes(&self) -> impl Iterator<Item = &CommandNode> {
        self.defs
            .iter()
            .flat_map(|child| std::iter::once(child).chain(child.iter_nodes()))
    }
}

impl ClientboundPacket for CommandDefinitions<'_> {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_COMMANDS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // Parse nodes
        let flattened: Vec<&CommandNode> = self.iter_nodes().collect();
        let mut raw: Vec<RawNode> = flattened
            .iter()
            .map(|node| node.to_raw_node(&flattened))
            .collect::<Result<Vec<_>, _>>()?;

        // Include root node
        raw.push(RawNode {
            flags: 0,
            children: self
                .defs
                .iter()
                .map(|node| {
                    flattened
                        .iter()
                        .enumerate()
                        .find_map(|(i, v)| (node == *v).then_some(i as i32))
                        .unwrap()
                })
                .collect(),
            name: None,
            parser: None,
        });

        // Write nodes
        writer.encode(raw.len() as i32)?;
        for node in raw.iter() {
            writer.write_all(&node.flags.to_be_bytes())?;
            writer.encode(node.children.len() as i32)?;
            node.children
                .iter()
                .try_for_each(|child| writer.encode(*child))?;
            if let Some(name) = &node.name {
                writer.encode(name)?;
            }
            if let Some((parser_type, parser_data)) = &node.parser {
                writer.encode(parser_type.to_id())?;
                writer.write_all(parser_data)?;
            }
        }
        // Root node index
        writer.encode((raw.len() - 1) as i32)?;

        Ok(())
    }
}
