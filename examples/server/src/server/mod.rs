mod state;
mod tab_info;

use std::{error::Error, net::TcpListener, sync::LazyLock};

use base64::Engine as _;
use pkmc_defs::{dimension::Dimension, registry::Registries};
use pkmc_server::{level::anvil::AnvilWorld, ClientHandler};
use pkmc_util::{
    connection::Connection, convert_ampersand_formatting_codes, normalize_identifier,
    retain_returned_vec, UUID,
};
pub use state::*;

use crate::{config::Config, player::Player};

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 / 20);

const PLAYER_DIMENSION: &str = "minecraft:overworld";

pub static REGISTRIES: LazyLock<Registries> =
    LazyLock::new(|| serde_json::from_str(include_str!("../registry.json")).unwrap());

#[derive(Debug)]
pub struct Server {
    config: Config,
    config_favicon: Option<String>,
    state: ServerState,
    tcp_listener: TcpListener,
    connecting: Vec<ClientHandler>,
    players: Vec<Player>,
}

impl Server {
    pub fn new(config: Config) -> Result<Self, Box<dyn Error>> {
        let tcp_listener = TcpListener::bind(&config.address)?;
        tcp_listener.set_nonblocking(true)?;

        println!("Server started on {}", tcp_listener.local_addr()?);

        Ok(Self {
            config_favicon: if let Some(icon_path) = &config.motd_icon {
                let img = image::open(icon_path)?;
                let img_resized = img.resize_exact(
                    64,
                    64,
                    config
                        .motd_icon_filtering_method
                        .to_image_rs_filtering_method(),
                );
                let mut png = std::io::Cursor::new(Vec::new());
                img_resized.write_to(&mut png, image::ImageFormat::Png)?;
                let png_base64 = base64::prelude::BASE64_STANDARD.encode(png.into_inner());
                Some(png_base64)
            } else {
                None
            },
            state: ServerState::new(AnvilWorld::new(
                &config.world,
                REGISTRIES
                    .get("minecraft:worldgen/biome")
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, (k, _v))| (normalize_identifier(k, "minecraft").into(), i as i32))
                    .collect(),
                REGISTRIES.get("minecraft:dimension_type").unwrap(),
            )?),
            config,
            tcp_listener,
            connecting: Vec::new(),
            players: Vec::new(),
        })
    }

    pub fn state(&self) -> ServerState {
        self.state.clone()
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        std::thread::spawn({
            let server_tab_info = self.state.server_tab_info.clone();
            move || loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Err(err) = server_tab_info.lock().unwrap().update() {
                    println!("{:?}", err);
                }
            }
        });

        std::thread::spawn({
            let world = self.state.world.clone();
            move || loop {
                std::thread::sleep(std::time::Duration::from_nanos(100));
                if let Err(err) = world.lock().unwrap().update_viewers() {
                    println!("{:?}", err);
                }
            }
        });

        let mut last_tick = std::time::Instant::now();
        let mut num_ticks: u64 = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1));

            self.update_connecting()?;
            self.update_players()?;

            if std::time::Instant::now().duration_since(last_tick) >= TICK_DURATION {
                last_tick = std::time::Instant::now();

                self.state.tab_list.lock().unwrap().update_viewers()?;
                self.state.iter_levels().try_for_each(|(_, state_level)| {
                    state_level
                        .entities
                        .lock()
                        .unwrap()
                        .update_viewers((num_ticks % 60) == 0)
                })?;

                num_ticks += 1;
            }
        }
    }

    fn update_connecting(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok((stream, _)) = self.tcp_listener.accept() {
            let connection = Connection::new(stream)?;
            let mut client = ClientHandler::new(connection)
                .with_brand(&self.config.brand)
                .with_online(self.config.online)
                .with_compression(
                    self.config.compression_threshold,
                    self.config.compression_level,
                )
                .with_registies(REGISTRIES.clone());
            if let Some(status_description) = &self.config.motd_text {
                client = client.with_status_description(convert_ampersand_formatting_codes(
                    status_description,
                ));
            }
            if let Some(status_favicon) = &self.config_favicon {
                client = client.with_status_favicon(status_favicon);
            }
            self.connecting.push(client);
        }

        self.connecting
            .iter_mut()
            .try_for_each(|client| client.update())?;

        retain_returned_vec(&mut self.connecting, |client| !client.is_finalized())
            .into_iter()
            .flat_map(|player| player.finalized_play_state())
            .try_for_each(|player| {
                let player = Player::new(
                    player.connection,
                    self.state.clone(),
                    player.player_name,
                    // Offline mode keeps players UUIDs the same, so we have to use a fake UUID for
                    // the players for everything to work properly.
                    if self.config.online {
                        player.player_id
                    } else {
                        UUID::new_v7()
                    },
                    player.player_properties,
                    player.player_info,
                    self.config.view_distance,
                    self.config.entity_distance,
                    Dimension::new(PLAYER_DIMENSION),
                )?;
                println!("{} Connected", player.name());
                self.players.push(player);
                Ok::<_, Box<dyn Error>>(())
            })?;

        Ok(())
    }

    fn update_players(&mut self) -> Result<(), Box<dyn Error>> {
        retain_returned_vec(&mut self.players, |player| !player.is_closed())
            .into_iter()
            .for_each(|player| {
                println!("{} Disconnected", player.name());
            });

        self.players.retain_mut(|player| {
            if let Err(err) = player.update() {
                let _ = player.kick(format!("{}", err));
                println!("{} Error: {}", player.name(), err);
                false
            } else {
                true
            }
        });

        Ok(())
    }
}
