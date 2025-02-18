mod config;
mod player;
mod server;

use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use config::Config;
use pkmc_server::entity_manager::{Entity, EntityManager};
use pkmc_util::{Vec3, UUID};
use server::Server;

fn test_entities(entity_manager: Arc<Mutex<EntityManager>>) {
    #[derive(Debug)]
    struct TestEntity;
    impl Entity for TestEntity {
        fn r#type(&self) -> i32 {
            110
        }
    }

    let entity1 = entity_manager
        .lock()
        .unwrap()
        .add_entity(TestEntity, UUID::new_v7());
    let entity2 = entity_manager
        .lock()
        .unwrap()
        .add_entity(TestEntity, UUID::new_v7());

    let start = std::time::Instant::now();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(10));

        let time = std::time::Instant::now()
            .duration_since(start)
            .as_secs_f64();

        let pos1 =
            Vec3::new(0.0, 100.0, 0.0) + Vec3::get_vector_for_rotation(0.0, time * 25.0) * 5.0;
        let pos2 = pos1 + Vec3::get_vector_for_rotation(0.0, time * 65.0) * 3.0;

        entity1.handler().lock().unwrap().position = pos1;
        entity2.handler().lock().unwrap().position = pos2;
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load(&["pkmc.toml", "pkmc/pkmc.toml"])?;
    let mut server = Server::new(config)?;
    test_entities(server.state().entities);
    server.run()?;
    Ok(())
}
