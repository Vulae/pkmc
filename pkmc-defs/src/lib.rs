pub mod handshake;
pub mod login;
pub mod play;
pub mod registry;

pub mod packet {
    pub use crate::handshake;
    pub use crate::login;
    pub use crate::play;
}
