pub mod client;
pub mod client_handshake;
pub mod client_login;
pub mod connection;
pub mod nbt;
pub mod packet;
pub mod server;
pub mod server_state;
pub mod uuid;

use std::path::{Path, PathBuf};

use anyhow::Result;
use base64::Engine as _;
use serde::Deserialize;
use server::Server;
use server_state::ServerState;

#[derive(Debug, Deserialize, Default)]
struct ConfigServerList {
    text: Option<String>,
    icon: Option<PathBuf>,
}

fn config_default_address() -> String {
    "127.0.0.1:25565".to_owned()
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "config_default_address")]
    address: String,
    #[serde(default, rename = "server-list")]
    server_list: ConfigServerList,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }
}

fn main() -> Result<()> {
    let config = Config::load("pkmc.toml")?;

    let state = ServerState {
        server_list_text: config.server_list.text,
        server_list_icon: if let Some(icon_path) = config.server_list.icon {
            let file = std::fs::read(icon_path)?;
            let base64 = base64::prelude::BASE64_STANDARD.encode(file);
            Some(base64)
        } else {
            None
        },
        world_main_name: "pkmc:test".to_owned(),
        world_min_y: 0,
        world_max_y: 16,
    };

    let mut server = Server::new(config.address, state)?;

    loop {
        server.step()?;
        // TODO: Probably use something like mio (https://docs.rs/mio/latest/mio/) for this.
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
