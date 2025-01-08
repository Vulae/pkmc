pub mod client;
pub mod config;
pub mod player;
pub mod server;
pub mod server_state;

use std::sync::{
    mpsc::{self, TryRecvError},
    Arc, Mutex,
};

use anyhow::Result;
use base64::Engine as _;
use config::Config;
use pkmc_world::world::World;
use server::Server;
use server_state::ServerState;

#[allow(unreachable_code)]
fn main() -> Result<()> {
    let config = Config::load("pkmc.toml")?;

    let state = ServerState {
        server_brand: config.brand,
        server_list_text: config.server_list.text,
        server_list_icon: if let Some(icon_path) = config.server_list.icon {
            let img = image::open(icon_path)?;
            let img_resized = img.resize_exact(
                64,
                64,
                config
                    .server_list
                    .icon_filtering_method
                    .to_image_rs_filtering_method(),
            );
            let mut png = std::io::Cursor::new(Vec::new());
            img_resized.write_to(&mut png, image::ImageFormat::Png)?;
            let png_base64 = base64::prelude::BASE64_STANDARD.encode(png.into_inner());
            Some(format!("data:image/png;base64,{}", png_base64))
        } else {
            None
        },
        compression_threshold: config.compression_threshold,
        compression_level: config.compression_level,
        world: Arc::new(Mutex::new(World::load(config.world)?)),
    };

    let mut server = Server::new(config.address, state)?;

    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });

    loop {
        // TODO: Probably use something like mio (https://docs.rs/mio/latest/mio/) for this.
        std::thread::sleep(std::time::Duration::from_millis(1));

        server.step()?;

        match rx.try_recv() {
            Ok(content) => match content.to_lowercase().trim() {
                "help" => println!("help, stop, list"),
                "stop" => break,
                "list" => todo!(),
                _ => println!("Unknown command"),
            },
            Err(TryRecvError::Empty) => {}
            Err(err) => Err(err)?,
        }
    }

    println!("Stopping server");

    Ok(())
}
