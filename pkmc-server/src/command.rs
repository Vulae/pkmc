use std::fmt::Debug;

use pkmc_defs::packet::{self, play::CommandNode};
use pkmc_util::{
    connection::{ConnectionError, ConnectionSender},
    Position, Vec3,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandParseError {
    #[error("{0}")]
    Custom(String),
    #[error("Command expected to be fully consumed")]
    NotFullyConsumed,
    #[error("Tried to consume an empty string")]
    TriedToConsumeEmptyString,
    #[error("Literal mismatch (expected \"{0}\", got \"{1}\")")]
    LiteralMismatch(String, String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("Cannot mix world & local coordinates")]
    MixedWorldLocalCoordinates,
    #[error("Couldn't convert coordinates to block position")]
    CouldntConvertCoordsToPosition,
}

pub trait CommandParsable: Sized {
    fn parse(parser: &mut CommandParser<'_>) -> Result<Self, CommandParseError>;
}

#[derive(Debug)]
pub struct CommandCoordinateTransform {
    relative_coordinates: Vec3<f64>,
    local_coordinates: Vec3<f64>,
    local_forward: Vec3<f64>,
    local_up: Vec3<f64>,
}

impl CommandCoordinateTransform {
    pub fn new(
        relative_coordinates: Vec3<f64>,
        local_coordinates: Vec3<f64>,
        local_forward: Vec3<f64>,
        local_up: Vec3<f64>,
    ) -> Self {
        Self {
            relative_coordinates,
            local_coordinates,
            local_forward: local_forward.normalized(),
            local_up: local_up.normalized(),
        }
    }

    pub fn new_partial(
        relative_coordinates: Vec3<f64>,
        local_coordinates: Vec3<f64>,
        local_forward: Vec3<f64>,
    ) -> Self {
        let forward = local_forward.normalized();
        let fake_left = Vec3::new(0.0, 1.0, 0.0).cross(&forward);
        let fake_upward = forward.cross(&fake_left);
        Self::new(
            relative_coordinates,
            local_coordinates,
            local_forward,
            fake_upward,
        )
    }

    fn local_left(&self) -> Vec3<f64> {
        self.local_forward.cross(&self.local_up)
    }
}

#[derive(Debug)]
pub enum CommandParsableCoordinate {
    World {
        x: f64,
        x_relative: bool,
        y: f64,
        y_relative: bool,
        z: f64,
        z_relative: bool,
    },
    Local {
        x: f64,
        y: f64,
        z: f64,
    },
}

impl CommandParsable for CommandParsableCoordinate {
    fn parse(parser: &mut CommandParser<'_>) -> Result<Self, CommandParseError> {
        fn parse_auto_zeroize(str: &str) -> Result<f64, CommandParseError> {
            Ok(if str.is_empty() { 0.0 } else { str.parse()? })
        }

        fn parse_with_relative(str: &str) -> Result<(f64, bool), CommandParseError> {
            if let Some(stripped) = str.strip_prefix('~') {
                Ok((parse_auto_zeroize(stripped)?, true))
            } else {
                Ok((str.parse()?, false))
            }
        }

        let x_str = parser.consume_until_space_or_end(true)?;
        let y_str = parser.consume_until_space_or_end(true)?;
        let z_str = parser.consume_until_space_or_end(true)?;

        match (
            x_str.strip_prefix('^'),
            y_str.strip_prefix('^'),
            z_str.strip_prefix('^'),
        ) {
            (Some(stripped_x), Some(stripped_y), Some(stripped_z)) => Ok(Self::Local {
                x: parse_auto_zeroize(stripped_x)?,
                y: parse_auto_zeroize(stripped_y)?,
                z: parse_auto_zeroize(stripped_z)?,
            }),
            (None, None, None) => {
                let (x, x_relative) = parse_with_relative(x_str)?;
                let (y, y_relative) = parse_with_relative(y_str)?;
                let (z, z_relative) = parse_with_relative(z_str)?;
                Ok(Self::World {
                    x,
                    x_relative,
                    y,
                    y_relative,
                    z,
                    z_relative,
                })
            }
            _ => Err(CommandParseError::MixedWorldLocalCoordinates),
        }
    }
}

impl CommandParsableCoordinate {
    pub fn to_coordinates(&self, transform: &CommandCoordinateTransform) -> Vec3<f64> {
        match self {
            CommandParsableCoordinate::World {
                x,
                x_relative,
                y,
                y_relative,
                z,
                z_relative,
            } => {
                let mut coords = Vec3::new(*x, *y, *z);
                if *x_relative {
                    coords.x += transform.relative_coordinates.x;
                }
                if *y_relative {
                    coords.y += transform.relative_coordinates.y;
                }
                if *z_relative {
                    coords.z += transform.relative_coordinates.z;
                }
                coords
            }
            CommandParsableCoordinate::Local { x, y, z } => {
                transform.local_coordinates
                    + (transform.local_left() * *x)
                    + (transform.local_up * *y)
                    + (transform.local_forward * *z)
            }
        }
    }

    pub fn to_position(
        &self,
        transform: &CommandCoordinateTransform,
    ) -> Result<Position, CommandParseError> {
        if let CommandParsableCoordinate::World {
            x,
            x_relative,
            y,
            y_relative,
            z,
            z_relative,
        } = self
        {
            if (!x_relative && x.trunc() != *x)
                || (!y_relative && y.trunc() != *y)
                || (!z_relative && z.trunc() != *z)
            {
                return Err(CommandParseError::CouldntConvertCoordsToPosition);
            }
        }
        let coords = self.to_coordinates(transform);
        Position::from_vec3(coords).ok_or(CommandParseError::CouldntConvertCoordsToPosition)
    }
}

#[derive(Debug, Clone)]
pub struct CommandParser<'a> {
    command: &'a str,
    consumable: &'a str,
}

impl<'a> CommandParser<'a> {
    fn new(command: &'a str) -> Self {
        let command = command.trim();
        Self {
            command,
            consumable: command,
        }
    }

    pub fn consume_until_space_or_end(
        &mut self,
        err_on_empty: bool,
    ) -> Result<&'a str, CommandParseError> {
        if err_on_empty && self.consumable.is_empty() {
            return Err(CommandParseError::TriedToConsumeEmptyString);
        }
        let Some(index) = self.consumable.find(' ') else {
            let rest = self.consumable;
            self.consumable = "";
            return Ok(rest);
        };
        let (first, second) = self.consumable.split_at(index);
        self.consumable = &second[1..];
        Ok(first)
    }
}

impl CommandParser<'_> {
    pub fn command(&self) -> &str {
        self.command
    }

    pub fn position(&self) -> usize {
        self.command.len() - self.consumable.len()
    }

    pub fn fully_consumed(&self) -> bool {
        self.consumable.is_empty()
    }

    pub fn error_not_fully_consumed(&self) -> Result<(), CommandParseError> {
        if !self.fully_consumed() {
            return Err(CommandParseError::NotFullyConsumed);
        }
        Ok(())
    }

    pub fn consume_literal(&mut self, literal: &str) -> Result<(), CommandParseError> {
        match self.consume_until_space_or_end(true)? {
            consumed if consumed == literal => Ok(()),
            consumed => Err(CommandParseError::LiteralMismatch(
                literal.to_owned(),
                consumed.to_owned(),
            )),
        }
    }

    pub fn consume<T: CommandParsable>(&mut self) -> Result<T, CommandParseError> {
        T::parse(self)
    }
}

pub trait CommandListener: Sized {
    fn node() -> CommandNode;
    type ParseArg;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError>;
}

#[derive(Debug, Default)]
pub struct CommandManager {
    registered: Vec<CommandNode>,
}

impl CommandManager {
    pub fn register<P: CommandListener>(&mut self) {
        let node = P::node();
        if self.registered.iter().any(|v| v == &node) {
            return;
        }
        self.registered.push(node);
    }

    pub fn unregister<P: CommandListener>(&mut self) {
        let node = P::node();
        self.registered.retain(|v| v != &node);
    }

    pub fn update_client_command_list(
        &mut self,
        connection: &ConnectionSender,
    ) -> Result<(), ConnectionError> {
        connection.send(&packet::play::CommandDefinitions {
            defs: &self.registered,
        })?;
        Ok(())
    }

    pub fn parse<T: CommandListener>(
        &self,
        command: &str,
        arg: &T::ParseArg,
    ) -> Result<Option<T>, CommandParseError> {
        let mut parser = CommandParser::new(command);
        let res = T::try_parse(&mut parser, arg)?;
        if res.is_some() {
            parser.error_not_fully_consumed()?;
        }
        Ok(res)
    }

    /// Try parsing the command, sending any errors in the client chat
    pub fn try_parse<T: CommandListener>(
        &self,
        command: &packet::play::ChatCommand,
        arg: &T::ParseArg,
        connection: &ConnectionSender,
    ) -> Result<Option<T>, ConnectionError> {
        match self.parse::<T>(&command.0, arg) {
            Err(err) => {
                connection.send(&packet::play::SystemChat {
                    content: format!("Parse error: {}", err).into(),
                    overlay: false,
                })?;
                Ok(None)
            }
            Ok(parsed) => Ok(parsed),
        }
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use crate::command::{
        CommandListener, CommandManager, CommandNode, CommandParseError, CommandParser,
    };

    #[test]
    fn command_parser_test() -> Result<(), Box<dyn Error>> {
        #[derive(Debug, PartialEq, Eq)]
        struct TestCommand(bool);

        impl CommandListener for TestCommand {
            fn node() -> CommandNode {
                CommandNode::new_literal("uwu")
            }

            type ParseArg = ();
            fn try_parse(
                parser: &mut CommandParser<'_>,
                _arg: &Self::ParseArg,
            ) -> Result<Option<Self>, CommandParseError> {
                if parser.consume_literal("uwu").is_err() {
                    return Ok(None);
                }
                Ok(Some(Self(
                    parser.consume_until_space_or_end(false)? == "owo",
                )))
            }
        }

        let mut manager = CommandManager::default();
        manager.register::<TestCommand>();

        assert_eq!(Some(TestCommand(false)), manager.parse("uwu", &())?);
        assert_eq!(Some(TestCommand(false)), manager.parse("uwu 0", &())?);
        assert_eq!(Some(TestCommand(true)), manager.parse("uwu owo", &())?);
        assert_eq!(None, manager.parse::<TestCommand>("uwu0", &())?);
        assert_eq!(None, manager.parse::<TestCommand>("hello", &())?);

        Ok(())
    }
}
