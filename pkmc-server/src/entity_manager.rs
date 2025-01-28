use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{atomic::AtomicI32, Arc, Mutex, Weak},
};

use pkmc_defs::packet::{self, play::EntityMetadataBundle};
use pkmc_util::{
    packet::{ConnectionError, ConnectionSender},
    Vec3, UUID,
};

pub trait Entity: Debug {
    fn r#type(&self) -> i32;
}

#[derive(Debug)]
pub struct EntityBase<T: Entity + ?Sized> {
    pub inner: Box<T>,
    handler: Arc<Mutex<EntityHandler>>,
    id: i32,
    uuid: UUID,
}

static ENTITY_ID_COUNTER: AtomicI32 = AtomicI32::new(0);

pub fn new_entity_id() -> i32 {
    ENTITY_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl<T: Entity> EntityBase<T> {
    fn new(inner: T, uuid: UUID) -> Self {
        let id = new_entity_id();
        Self {
            handler: Arc::new(Mutex::new(EntityHandler::new(id, uuid, inner.r#type()))),
            inner: Box::new(inner),
            id,
            uuid,
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn handler(&self) -> &Arc<Mutex<EntityHandler>> {
        &self.handler
    }
}

#[derive(Debug)]
pub struct EntityHandler {
    id: i32,
    uuid: UUID,
    r#type: i32,
    pub position: Vec3<f64>,
    last_position: Vec3<f64>,
    pub velocity: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    last_yaw_pitch: (f32, f32),
    pub on_ground: bool,
    pub metadata: EntityMetadataBundle,
}

impl EntityHandler {
    fn new(id: i32, uuid: UUID, r#type: i32) -> Self {
        Self {
            id,
            uuid,
            r#type,
            position: Vec3::zero(),
            last_position: Vec3::zero(),
            velocity: Vec3::zero(),
            yaw: 0.0,
            pitch: 0.0,
            last_yaw_pitch: (0.0, 0.0),
            on_ground: false,
            metadata: EntityMetadataBundle::empty(),
        }
    }
}

#[derive(Debug)]
pub struct EntityViewer {
    connection: ConnectionSender,
    uuid: UUID,
    viewing: HashSet<i32>,
}

impl EntityViewer {
    fn new(connection: ConnectionSender, uuid: UUID) -> Self {
        Self {
            connection,
            uuid,
            viewing: HashSet::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct EntityManager {
    entities: HashMap<UUID, Weak<Mutex<EntityHandler>>>,
    viewers: Vec<Weak<Mutex<EntityViewer>>>,
}

impl EntityManager {
    pub fn add_viewer(
        &mut self,
        connection: ConnectionSender,
        uuid: UUID,
    ) -> Arc<Mutex<EntityViewer>> {
        let viewer = Arc::new(Mutex::new(EntityViewer::new(connection, uuid)));
        self.viewers.push(Arc::downgrade(&viewer));
        viewer
    }

    pub fn update_viewers(&mut self, force_sync: bool) -> Result<(), ConnectionError> {
        self.viewers.retain(|v| v.strong_count() > 0);

        let viewers = self
            .viewers
            .iter()
            .flat_map(|v| v.upgrade())
            .collect::<Vec<_>>();

        self.entities.retain(|_, e| e.strong_count() > 0);

        let entities = self
            .entities
            .values()
            .flat_map(|e| e.upgrade())
            .collect::<Vec<_>>();

        viewers
            .iter()
            .map(|v| v.lock().unwrap())
            .try_for_each(|mut viewer| {
                entities
                    .iter()
                    .map(|e| e.lock().unwrap())
                    .try_for_each(|entity| {
                        if entity.uuid == viewer.uuid {
                            return Ok(());
                        }
                        if viewer.viewing.contains(&entity.id) {
                            return Ok(());
                        }
                        viewer.viewing.insert(entity.id);
                        viewer.connection.send(&packet::play::AddEntity {
                            id: entity.id,
                            uuid: entity.uuid,
                            r#type: entity.r#type,
                            position: entity.position,
                            pitch: 0,
                            yaw: 0,
                            head_yaw: 0,
                            data: 0,
                            velocity_x: 0,
                            velocity_y: 0,
                            velocity_z: 0,
                        })?;
                        viewer.connection.send(&packet::play::SetEntityMetadata {
                            entity_id: entity.id,
                            metadata: entity.metadata.clone(),
                        })?;
                        Ok::<_, ConnectionError>(())
                    })
            })?;

        viewers
            .iter()
            .map(|v| v.lock().unwrap())
            .try_for_each(|viewer| {
                entities
                    .iter()
                    .map(|e| e.lock().unwrap())
                    .filter(|entity| viewer.viewing.contains(&entity.id))
                    .try_for_each(|entity| {
                        if force_sync {
                            viewer.connection.send(&packet::play::EntityPositionSync {
                                entity_id: entity.id,
                                position: entity.position,
                                velocity: entity.velocity,
                                yaw: entity.yaw,
                                pitch: entity.pitch,
                                on_ground: entity.on_ground,
                            })?;
                            return Ok(());
                        }

                        fn calc_delta(v: f64) -> Option<i16> {
                            let v = v * 4096.0;
                            if v > i16::MAX.into() {
                                return None;
                            }
                            if v < i16::MIN.into() {
                                return None;
                            }
                            Some(v.round() as i16)
                        }

                        // FIXME: I for the life of me cannot get the quantized angle to work
                        // properly. I've even gone into the decompiled Minecraft code and stole it
                        // from there and it still doesn't work. I honestly have no clue at this point.
                        fn wrap_degrees(v: f32) -> f32 {
                            match v % 360.0 {
                                v if v >= 180.0 => v - 360.0,
                                v if v < -180.0 => v + 360.0,
                                v => v,
                            }
                        }

                        pub fn quantize_angle(angle: f32) -> u8 {
                            ((wrap_degrees(angle) * 360.0) / 256.0) as u8
                        }

                        let position_delta = entity.position - entity.last_position;
                        match (
                            position_delta.length() <= 1e-6,
                            if let (Some(delta_x), Some(delta_y), Some(delta_z)) = (
                                calc_delta(position_delta.x),
                                calc_delta(position_delta.y),
                                calc_delta(position_delta.z),
                            ) {
                                if delta_x == 0 && delta_y == 0 && delta_z == 0 {
                                    None
                                } else {
                                    Some((delta_x, delta_y, delta_z))
                                }
                            } else {
                                None
                            },
                            (entity.yaw, entity.pitch) == entity.last_yaw_pitch,
                        ) {
                            (true, _, true) => {}
                            (false, Some((delta_x, delta_y, delta_z)), true) => {
                                viewer.connection.send(&packet::play::MoveEntityPos {
                                    entity_id: entity.id,
                                    delta_x,
                                    delta_y,
                                    delta_z,
                                    on_ground: entity.on_ground,
                                })?;
                            }
                            (false, Some((delta_x, delta_y, delta_z)), false) => {
                                viewer.connection.send(&packet::play::MoveEntityPosRot {
                                    entity_id: entity.id,
                                    delta_x,
                                    delta_y,
                                    delta_z,
                                    yaw: quantize_angle(entity.yaw),
                                    pitch: quantize_angle(entity.pitch),
                                    on_ground: entity.on_ground,
                                })?;
                            }
                            (true, _, false) => {
                                viewer.connection.send(&packet::play::MoveEntityRot {
                                    entity_id: entity.id,
                                    yaw: quantize_angle(entity.yaw),
                                    pitch: quantize_angle(entity.pitch),
                                    on_ground: entity.on_ground,
                                })?;
                            }
                            _ => {
                                viewer.connection.send(&packet::play::EntityPositionSync {
                                    entity_id: entity.id,
                                    position: entity.position,
                                    velocity: entity.velocity,
                                    yaw: entity.yaw,
                                    pitch: entity.pitch,
                                    on_ground: entity.on_ground,
                                })?;
                            }
                        }

                        Ok::<_, ConnectionError>(())
                    })
            })?;

        entities
            .iter()
            .map(|e| e.lock().unwrap())
            .for_each(|mut entity| {
                entity.last_position = entity.position;
                entity.last_yaw_pitch = (entity.yaw, entity.pitch);
            });

        Ok(())
    }

    pub fn add_entity<T: Entity>(&mut self, entity: T, uuid: UUID) -> EntityBase<T> {
        let entity = EntityBase::new(entity, uuid);
        self.entities.insert(uuid, Arc::downgrade(&entity.handler));
        entity
    }
}
