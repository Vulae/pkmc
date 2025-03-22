use std::{
    error::Error,
    net::TcpListener,
    sync::{Arc, LazyLock, Mutex},
};

use base64::Engine as _;
use pkmc_defs::registry::Registries;
use pkmc_server::{
    entity_manager::EntityManager,
    tab_list::TabList,
    world::{anvil::AnvilWorld, World},
    ClientHandler,
};
use pkmc_util::{
    connection::Connection, convert_ampersand_formatting_codes, normalize_identifier,
    retain_returned_vec, UUID,
};

use crate::{config::Config, player::Player};

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 / 20);

pub static REGISTRIES: LazyLock<Registries> =
    LazyLock::new(|| serde_json::from_str(include_str!("./registry.json")).unwrap());

#[derive(Debug, Clone)]
pub struct ServerState {
    pub world: Arc<Mutex<AnvilWorld>>,
    pub entities: Arc<Mutex<EntityManager>>,
    pub tab_list: Arc<Mutex<TabList>>,
}

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
            state: ServerState {
                world: Arc::new(Mutex::new(AnvilWorld::new(
                    &config.world,
                    "minecraft:overworld",
                    -4..=19,
                    REGISTRIES
                        .get("minecraft:worldgen/biome")
                        .unwrap()
                        .iter()
                        .enumerate()
                        .map(|(i, (k, _v))| (normalize_identifier(k, "minecraft").into(), i as i32))
                        .collect(),
                ))),
                entities: Arc::new(Mutex::new(EntityManager::default())),
                tab_list: Arc::new(Mutex::new(TabList::default())),
            },
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
                self.state
                    .entities
                    .lock()
                    .unwrap()
                    .update_viewers((num_ticks % 60) == 0)?;

                num_ticks += 1;
            }
        }
    }

    fn update_connecting(&mut self) -> Result<(), Box<dyn Error>> {
        while let Ok((stream, _)) = self.tcp_listener.accept() {
            let connection = Connection::new(stream)?;
            let mut client = ClientHandler::new(connection)
                .with_brand(&self.config.brand)
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
                    player.player_id,
                    UUID::new_v7(),
                    self.config.view_distance,
                    self.config.entity_distance,
                )?;
                println!("{} Connected", player.player_name());
                self.players.push(player);
                Ok::<_, Box<dyn Error>>(())
            })?;

        Ok(())
    }

    fn update_players(&mut self) -> Result<(), Box<dyn Error>> {
        retain_returned_vec(&mut self.players, |player| !player.is_closed())
            .into_iter()
            .for_each(|player| {
                println!("{} Disconnected", player.player_name());
            });

        self.players
            .iter_mut()
            .try_for_each(|player| player.update())?;

        Ok(())
    }
}
