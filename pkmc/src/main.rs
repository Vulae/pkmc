pub mod client;
pub mod config;
pub mod player;
pub mod server;
pub mod server_state;

use anyhow::Result;
use base64::Engine as _;
use config::Config;
use pkmc_world::world::World;
use server::Server;
use server_state::ServerState;

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
        world: World::load(config.world)?,
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
