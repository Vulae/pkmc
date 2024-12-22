pub mod client;
pub mod player;
pub mod server;
pub mod server_state;

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

fn config_default_brand() -> String {
    "Vulae/pkmc".to_owned()
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "config_default_address")]
    address: String,
    #[serde(default = "config_default_brand")]
    brand: String,
    #[serde(default, rename = "server-list")]
    server_list: ConfigServerList,
    #[serde(default, rename = "compression-threshold")]
    compression_threshold: usize,
    #[serde(default, rename = "compression-level")]
    compression_level: u32,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }
}

fn main() -> Result<()> {
    let config = Config::load("pkmc.toml")?;

    let state = ServerState {
        server_brand: config.brand,
        server_list_text: config.server_list.text,
        server_list_icon: if let Some(icon_path) = config.server_list.icon {
            let file = std::fs::read(icon_path)?;
            let base64 = base64::prelude::BASE64_STANDARD.encode(file);
            Some(base64)
        } else {
            None
        },
        compression_threshold: config.compression_threshold,
        compression_level: config.compression_level,
        world_main_name: "pkmc:test".to_owned(),
        world_min_y: 0,
        world_max_y: 16,
    };

    let mut server = Server::new(config.address, state)?;
    //let mut terminal = ratatui::init();
    //let mut last_render = None;
    //const RENDER_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

    loop {
        // TODO: Probably use something like mio (https://docs.rs/mio/latest/mio/) for this.
        std::thread::sleep(std::time::Duration::from_millis(1));

        server.step()?;

        //if ratatui::crossterm::event::poll(std::time::Duration::ZERO)? {
        //    match ratatui::crossterm::event::read()? {
        //        ratatui::crossterm::event::Event::Key(key)
        //            if key.code == ratatui::crossterm::event::KeyCode::Char('q') =>
        //        {
        //            break;
        //        }
        //        _ => {}
        //    }
        //}
        //
        //if let Some(last_render) = last_render {
        //    if std::time::Instant::now().duration_since(last_render) < RENDER_DELAY {
        //        continue;
        //    }
        //}
        //last_render = Some(std::time::Instant::now());
        //
        //terminal.draw(|frame| {
        //    let [info, time] =
        //        ratatui::prelude::Layout::vertical(ratatui::prelude::Constraint::from_lengths([
        //            1, 1,
        //        ]))
        //        .areas(frame.area());
        //    frame.render_widget(format!("Server running at \"{}\"", server.ip()), info);
        //    frame.render_widget(format!("{:?}", last_render), time);
        //})?;
    }

    //ratatui::restore();

    //Ok(())
}
