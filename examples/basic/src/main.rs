// TODO: Currently the server is setup so that everything except for dimensions is all the same.
// So that means someone in the overworld can see someone else that is in the same position in the nether.

mod config;
mod player;
mod server;

use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use config::Config;
use pkmc_defs::packet;
use pkmc_generated::registry::EntityType;
use pkmc_server::entity_manager::{Entity, EntityManager};
use pkmc_util::{Vec3, UUID};
use server::Server;

#[derive(Debug)]
struct TestOrbitEntity {
    is_star: bool,
    size: i32,
    initial_offset: Vec3<f64>,
    // (distance, speed, offset)
    orbit_states: Vec<(f64, f64, f64)>,
}

impl Entity for TestOrbitEntity {
    fn r#type(&self) -> EntityType {
        match self.is_star {
            false => EntityType::Slime,
            true => EntityType::MagmaCube,
        }
    }
}

fn test_entities(entity_manager: Arc<Mutex<EntityManager>>) {
    let mut entities = vec![];
    {
        let mut entity_manager = entity_manager.lock().unwrap();
        entities.push(entity_manager.add_entity(
            TestOrbitEntity {
                is_star: true,
                size: 10,
                initial_offset: Vec3::new(0.0, 200.0, 0.0),
                orbit_states: vec![],
            },
            UUID::new_v7(),
        ));
        entities.push(entity_manager.add_entity(
            TestOrbitEntity {
                is_star: false,
                size: 3,
                initial_offset: Vec3::new(0.0, 200.0, 0.0),
                orbit_states: vec![(20.0, 30.0, 0.0)],
            },
            UUID::new_v7(),
        ));
        entities.push(entity_manager.add_entity(
            TestOrbitEntity {
                is_star: false,
                size: 1,
                initial_offset: Vec3::new(0.0, 200.0, 0.0),
                orbit_states: vec![(20.0, 30.0, 0.0), (4.0, -200.0, 0.0)],
            },
            UUID::new_v7(),
        ));
        entities.push(entity_manager.add_entity(
            TestOrbitEntity {
                is_star: false,
                size: 2,
                initial_offset: Vec3::new(0.0, 200.0, 0.0),
                orbit_states: vec![(10.0, 100.0, 0.0)],
            },
            UUID::new_v7(),
        ));
    }
    entities.iter().for_each(|entity| {
        entity
            .handler()
            .lock()
            .unwrap()
            .metadata
            .0
            .insert(16, packet::play::EntityMetadata::VarInt(entity.inner.size));
    });

    let start = std::time::Instant::now();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(10));

        let time = std::time::Instant::now()
            .duration_since(start)
            .as_secs_f64();

        entities.iter().for_each(|entity| {
            let pos = entity.inner.orbit_states.iter().fold(
                entity.inner.initial_offset,
                |pos, (distance, speed, offset)| {
                    pos + Vec3::get_vector_for_rotation(0.0, time * speed + offset) * *distance
                },
            );
            entity.handler().lock().unwrap().position = pos;
        });
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load(&["config.toml", "examples/basic/config.toml"])?;
    let mut server = Server::new(config)?;
    test_entities(server.state().entities);
    server.run()?;
    Ok(())
}
