#![allow(unused)]

mod config;
mod player;

use std::{
    error::Error,
    net::TcpListener,
    sync::{Arc, LazyLock, Mutex, RwLock},
};

use base64::Engine as _;
use config::Config;
use pkmc_defs::{biome::Biome, registry::Registries};
use pkmc_server::{world::anvil::AnvilWorld, ClientHandler};
use pkmc_util::{normalize_identifier, packet::Connection, IdTable, IterRetain};
use player::Player;

pub static REGISTRIES: LazyLock<Registries> =
    LazyLock::new(|| serde_json::from_str(include_str!("./registry.json")).unwrap());

#[derive(Debug, Clone)]
pub struct ServerState {
    pub world: Arc<Mutex<AnvilWorld>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load(&["pkmc.toml", "pkmc/pkmc.toml"])?;

    let config_favicon = if let Some(icon_path) = config.motd_icon {
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
    };

    let world = AnvilWorld::new(config.world, "minecraft:overworld", -4..=20);
    let state = ServerState {
        world: Arc::new(Mutex::new(world)),
    };

    let listener = TcpListener::bind(config.address)?;
    listener.set_nonblocking(true)?;

    println!("Server started on {}", listener.local_addr()?);

    let mut clients: Vec<ClientHandler> = Vec::new();
    let mut players: Vec<Player> = Vec::new();

    let biome_mapper: IdTable<Biome> = REGISTRIES
        .get("minecraft:worldgen/biome")
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, (k, _v))| (normalize_identifier(k, "minecraft").into(), i as i32))
        .collect();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1));

        while let Ok((stream, _)) = listener.accept() {
            let connection = Connection::new(stream)?;
            let mut client = ClientHandler::new(connection)
                .with_brand(&config.brand)
                .with_compression(config.compression_threshold, config.compression_level)
                .with_registies(REGISTRIES.clone());
            if let Some(status_description) = &config.motd_text {
                client = client.with_status_description(status_description);
            }
            if let Some(status_favicon) = &config_favicon {
                client = client.with_status_favicon(status_favicon);
            }
            clients.push(client);
        }

        clients.iter_mut().try_for_each(|client| client.update())?;

        clients
            .retain_returned(|client| !client.is_finalized())
            .into_iter()
            .flat_map(|player| player.finalized_play_state())
            .try_for_each(|player| {
                let mut player = Player::new(
                    player.connection,
                    state.clone(),
                    player.player_id,
                    player.player_name,
                    config.view_distance,
                    biome_mapper.clone(),
                )?;
                println!("{} Connected", player.name());
                players.push(player);
                Ok::<_, Box<dyn Error>>(())
            })?;

        players
            .retain_returned(|player| !player.is_closed())
            .into_iter()
            .for_each(|player| {
                println!("{} Disconnected", player.name());
            });

        players.iter_mut().try_for_each(|player| player.update())?;
    }
}
