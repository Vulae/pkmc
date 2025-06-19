use pkmc_defs::packet::{
    self,
    play::{CommandNode, CommandNodeParser},
};
use pkmc_generated::block::Block;
use pkmc_server::{
    command::{
        CommandCoordinateTransform, CommandListener, CommandParsableCoordinate, CommandParseError,
        CommandParser,
    },
    level::Level as _,
};
use pkmc_util::{Position, Vec3};

use super::{Player, PlayerError};

pub trait PlayerExecutableCommand: CommandListener {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError>;
}

impl Player {
    pub fn coordinate_transform(&self) -> CommandCoordinateTransform {
        CommandCoordinateTransform::new_partial(
            self.position,
            self.position + Vec3::new(0.0, 1.5, 0.0),
            Vec3::get_vector_for_rotation(self.pitch.into(), self.yaw.into()),
        )
    }

    pub fn execute_command<C: PlayerExecutableCommand>(
        &mut self,
        command: C,
    ) -> Result<(), PlayerError> {
        command.execute(self)
    }

    fn try_parse_then_execute_command<C: PlayerExecutableCommand>(
        &mut self,
        packet: &packet::play::ChatCommand,
        arg: &C::ParseArg,
    ) -> Result<(), PlayerError> {
        self.command_manager
            .try_parse::<C>(packet, arg, &self.connection.sender())?
            .map(|c| c.execute(self))
            .transpose()?;
        Ok(())
    }

    pub(super) fn define_commands(&mut self) -> Result<(), PlayerError> {
        self.command_manager.register::<CommandGoto>();
        self.command_manager.register::<CommandData>();
        self.command_manager.register::<CommandDestroy>();
        self.command_manager
            .update_client_command_list(&self.connection.sender())?;
        Ok(())
    }

    pub fn parse_then_execute_command(
        &mut self,
        packet: &packet::play::ChatCommand,
    ) -> Result<(), PlayerError> {
        let transform = self.coordinate_transform();
        self.try_parse_then_execute_command::<CommandGoto>(packet, &transform)?;
        self.try_parse_then_execute_command::<CommandData>(packet, &transform)?;
        self.try_parse_then_execute_command::<CommandDestroy>(packet, &transform)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandGoto {
    position: Vec3<f64>,
    dimension: Option<String>,
}

impl CommandListener for CommandGoto {
    fn node() -> CommandNode {
        CommandNode::new_literal("goto").with_child(
            CommandNode::new(CommandNodeParser::Vec3).with_child(
                CommandNode::new_literal("in").with_child(CommandNode::new(
                    CommandNodeParser::ResourceKey("minecraft:dimension_type".to_owned()),
                )),
            ),
        )
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("goto").is_err() {
            return Ok(None);
        }
        Ok(Some(Self {
            position: parser
                .consume::<CommandParsableCoordinate>()?
                .to_coordinates(arg),
            dimension: if parser.consume_until_space_or_end(false)? == "in" {
                Some(parser.consume_until_space_or_end(true)?.to_owned())
            } else {
                None
            },
        }))
    }
}

impl PlayerExecutableCommand for CommandGoto {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        if let Some(dimension) = self.dimension {
            match player.set_dimension(dimension.into(), self.position) {
                Err(PlayerError::CouldNotFindDimension(dimension)) => {
                    player.system_message(format!(
                        "Could not find dimension \"{:?}\"",
                        dimension.name()
                    ))?;
                }
                res => res?,
            }
        } else {
            player.teleport(self.position)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandData {
    position: Position,
}

impl CommandListener for CommandData {
    fn node() -> CommandNode {
        CommandNode::new_literal("data")
            .with_child(CommandNode::new(CommandNodeParser::BlockPosition))
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("data").is_err() {
            return Ok(None);
        }
        Ok(Some(Self {
            position: parser
                .consume::<CommandParsableCoordinate>()?
                .to_position(arg)?,
        }))
    }
}

impl PlayerExecutableCommand for CommandData {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        let level_mutex = player.server_state_level.level.clone();
        let mut level = level_mutex.lock().unwrap();
        if let Some(data) = level.query_block_data(self.position)? {
            player.system_message(format!(
                "Block data at {}: {}",
                self.position,
                data.data.to_string_pretty()
            ))?;
        } else {
            player.system_message(format!("No block data at {}", self.position))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum CommandDestroy {
    Sphere { radius: f32 },
    Cube { pos1: Position, pos2: Position },
}

impl CommandListener for CommandDestroy {
    fn node() -> CommandNode {
        CommandNode::new_literal("destroy")
            .with_child(
                CommandNode::new_literal("sphere").with_child(CommandNode::new(
                    CommandNodeParser::Float {
                        min: None,
                        max: Some(256.0),
                    },
                )),
            )
            .with_child(
                CommandNode::new_literal("cube").with_child(
                    CommandNode::new(CommandNodeParser::BlockPosition)
                        .with_child(CommandNode::new(CommandNodeParser::BlockPosition)),
                ),
            )
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("destroy").is_err() {
            return Ok(None);
        }
        match parser.consume_until_space_or_end(true)? {
            "sphere" => Ok(Some(Self::Sphere {
                radius: parser.consume_until_space_or_end(true)?.parse()?,
            })),
            "cube" => Ok(Some(Self::Cube {
                pos1: parser
                    .consume::<CommandParsableCoordinate>()?
                    .to_position(arg)?,
                pos2: parser
                    .consume::<CommandParsableCoordinate>()?
                    .to_position(arg)?,
            })),
            _ => Err(CommandParseError::Custom(
                "Expected valid literal argument".to_owned(),
            )),
        }
    }
}

impl PlayerExecutableCommand for CommandDestroy {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        let mut level = player.server_state_level.level.lock().unwrap();
        match self {
            CommandDestroy::Sphere { radius } => {
                if let Some(position) = Position::iter_ray(
                    player.position + Vec3::new(0.0, 1.5, 0.0),
                    Vec3::get_vector_for_rotation(player.pitch.into(), player.yaw.into()),
                    5000.0,
                )
                .find(|p| {
                    level
                        .get_block(*p)
                        .ok()
                        .flatten()
                        .map(|b| !b.is_air())
                        .unwrap_or(false)
                }) {
                    Position::iter_offset(Position::iter_sphere(radius), position)
                        .try_for_each(|p| level.set_block(p, Block::Air))?;
                }
            }
            CommandDestroy::Cube { pos1, pos2 } => {
                let (min, max) = Position::fix_boundaries(pos1, pos2);
                let size = max - min;
                Position::iter_offset(Position::iter_cube(size.x + 1, size.y + 1, size.z + 1), min)
                    .try_for_each(|p| level.set_block(p, Block::Air))?;
            }
        }
        Ok(())
    }
}
