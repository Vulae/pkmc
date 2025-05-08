use std::{
    error::Error,
    net::TcpListener,
    sync::{Arc, LazyLock, Mutex},
};

use base64::Engine as _;
use pkmc_defs::{
    dimension::Dimension, packet, registry::Registries, text_component::TextComponent,
};
use pkmc_server::{
    entity_manager::EntityManager, level::anvil::AnvilWorld, tab_list::TabList, ClientHandler,
};
use pkmc_util::{
    connection::{Connection, ConnectionSender},
    convert_ampersand_formatting_codes, normalize_identifier, retain_returned_vec, Color, WeakList,
};

use crate::{
    config::Config,
    player::{Player, PlayerError},
};

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 / 20);

const PLAYER_DIMENSION: &str = "minecraft:overworld";

pub static REGISTRIES: LazyLock<Registries> =
    LazyLock::new(|| serde_json::from_str(include_str!("./registry.json")).unwrap());

#[derive(Debug)]
pub struct ServerTabInfo {
    sys: sysinfo::System,
    pid: sysinfo::Pid,
    viewers: WeakList<Mutex<ConnectionSender>>,
}

impl ServerTabInfo {
    fn new() -> Self {
        Self {
            sys: sysinfo::System::new_with_specifics(
                sysinfo::RefreshKind::nothing().with_processes(
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                ),
            ),
            pid: sysinfo::get_current_pid().unwrap(),
            viewers: WeakList::new(),
        }
    }

    pub fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<ConnectionSender>> {
        self.viewers.push(Mutex::new(connection))
    }

    fn update(&mut self) -> Result<(), PlayerError> {
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), true);
        let process = self.sys.process(self.pid).unwrap();
        self.viewers.iter().try_for_each(|viewer| {
            viewer.send(&packet::play::SetTabListHeaderAndFooter {
                header: None,
                footer: Some(
                    TextComponent::empty()
                        .with_child(|child| child.with_content("CPU: ").with_color(Color::GOLD))
                        .with_child(|child| {
                            child
                                .with_content(format!("{:.1}%", process.cpu_usage()))
                                .with_color(Color::YELLOW)
                        })
                        .with_child(|child| child.with_content(" - ").with_color(Color::DARK_GRAY))
                        .with_child(|child| {
                            child.with_content("MEM: ").with_color(Color::DARK_PURPLE)
                        })
                        .with_child(|child| {
                            child
                                .with_content(format!("{}MiB", process.memory() / 1048576))
                                .with_color(Color::LIGHT_PURPLE)
                        }),
                ),
            })
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub world: Arc<Mutex<AnvilWorld>>,
    pub entities: Arc<Mutex<EntityManager>>,
    pub tab_list: Arc<Mutex<TabList>>,
    pub server_tab_info: Arc<Mutex<ServerTabInfo>>,
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
                    REGISTRIES
                        .get("minecraft:worldgen/biome")
                        .unwrap()
                        .iter()
                        .enumerate()
                        .map(|(i, (k, _v))| (normalize_identifier(k, "minecraft").into(), i as i32))
                        .collect(),
                    REGISTRIES.get("minecraft:dimension_type").unwrap(),
                )?)),
                entities: Arc::new(Mutex::new(EntityManager::default())),
                tab_list: Arc::new(Mutex::new(TabList::default())),
                server_tab_info: Arc::new(Mutex::new(ServerTabInfo::new())),
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
                    player.player_id,
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

        self.players
            .iter_mut()
            .try_for_each(|player| player.update())?;

        Ok(())
    }
}
