use std::{
    collections::HashSet,
    fmt::Debug,
    sync::{atomic::AtomicI32, Arc, Mutex},
};

use pkmc_defs::packet::{self, play::EntityMetadataBundle};
use pkmc_util::{
    packet::{ConnectionError, ConnectionSender},
    Vec3, WeakList, WeakMap, UUID,
};

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

pub fn quantize_angle(angle: f32) -> u8 {
    (angle.rem_euclid(360.0) * (256.0 / 360.0)) as u8
}

pub trait Entity: Debug {
    fn r#type(&self) -> i32;
}

#[derive(Debug)]
pub struct EntityBase<T: Entity + Sized> {
    pub inner: T,
    handler: Arc<Mutex<EntityHandler>>,
    uuid: UUID,
}

static ENTITY_ID_COUNTER: AtomicI32 = AtomicI32::new(0);

pub fn new_entity_id() -> i32 {
    ENTITY_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl<T: Entity + Sized> EntityBase<T> {
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
    pub head_yaw: f32,
    last_head_yaw: f32,
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
            head_yaw: 0.0,
            last_head_yaw: 0.0,
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
    entities: WeakMap<i32, EntityHandler>,
    viewers: WeakList<EntityViewer>,
}

impl EntityManager {
    pub fn add_viewer(
        &mut self,
        connection: ConnectionSender,
        uuid: UUID,
    ) -> Arc<Mutex<EntityViewer>> {
        self.viewers.push(EntityViewer::new(connection, uuid))
    }

    pub fn update_viewers(&mut self, force_sync: bool) -> Result<(), ConnectionError> {
        self.viewers.cleanup();

        let removed_entities: HashSet<i32> = self.entities.cleanup();

        self.viewers.iter().try_for_each(|mut viewer| {
            if !removed_entities.is_empty() {
                viewer
                    .connection
                    .send(&packet::play::RemoveEntities(removed_entities.clone()))?;
            }

            self.entities.iter().try_for_each(|(_, entity)| {
                if entity.uuid == viewer.uuid {
                    return Ok(());
                }
                if viewer.viewing.contains(&entity.id) {
                    return Ok(());
                }
                viewer.viewing.insert(entity.id);
                viewer.connection.send(&packet::play::BundleDelimiter)?;
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
                viewer.connection.send(&packet::play::SetHeadRotation {
                    entity_id: entity.id,
                    yaw: quantize_angle(entity.head_yaw),
                })?;
                viewer.connection.send(&packet::play::SetEntityMetadata {
                    entity_id: entity.id,
                    metadata: entity.metadata.clone(),
                })?;
                viewer.connection.send(&packet::play::BundleDelimiter)?;
                Ok::<_, ConnectionError>(())
            })
        })?;

        self.viewers.iter().try_for_each(|viewer| {
            self.entities
                .iter()
                .filter(|(_, entity)| viewer.viewing.contains(&entity.id))
                .try_for_each(|(_, entity)| {
                    if force_sync {
                        viewer.connection.send(&packet::play::EntityPositionSync {
                            entity_id: entity.id,
                            position: entity.position,
                            velocity: entity.velocity,
                            yaw: entity.yaw,
                            pitch: entity.pitch,
                            on_ground: entity.on_ground,
                        })?;
                        viewer.connection.send(&packet::play::SetHeadRotation {
                            entity_id: entity.id,
                            yaw: quantize_angle(entity.head_yaw),
                        })?;
                        return Ok(());
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

                    if entity.head_yaw != entity.last_head_yaw {
                        viewer.connection.send(&packet::play::SetHeadRotation {
                            entity_id: entity.id,
                            yaw: quantize_angle(entity.head_yaw),
                        })?;
                    }

                    Ok::<_, ConnectionError>(())
                })
        })?;

        self.entities.iter().for_each(|(_, mut entity)| {
            entity.last_position = entity.position;
            entity.last_yaw_pitch = (entity.yaw, entity.pitch);
            entity.last_head_yaw = entity.head_yaw;
        });

        Ok(())
    }

    pub fn add_entity<T: Entity>(&mut self, entity: T, uuid: UUID) -> EntityBase<T> {
        let id = new_entity_id();
        EntityBase {
            handler: self
                .entities
                .insert_ignored(id, EntityHandler::new(id, uuid, entity.r#type())),
            inner: entity,
            uuid,
        }
    }
}
