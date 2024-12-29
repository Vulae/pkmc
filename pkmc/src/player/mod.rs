pub mod chunk_loader;
pub mod player;

pub use chunk_loader::*;
pub use player::Player;

use pkmc_packet::connection::ConnectionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(
        "Client bad keep alive response (No response, wrong id, or responded when not expected)"
    )]
    BadKeepAliveResponse,
}
