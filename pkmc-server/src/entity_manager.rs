use std::{
    collections::HashSet,
    fmt::Debug,
    sync::{atomic::AtomicI32, Arc, Mutex},
};

use pkmc_defs::{
    generated::generated::registries::EntityType,
    packet::{
        self,
        play::{EntityAnimationType, EntityMetadataBundle},
    },
};
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
    const TYPE: EntityType;
    fn r#type(&self) -> EntityType {
        Self::TYPE
    }
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

/// Determines when entity is sent to viewers.
#[derive(Debug)]
pub enum EntityVisibility {
    /// Entity is visible to all but selected viewers.
    Exclude(HashSet<UUID>),
    /// Entity is visible to selected viewers.
    Include(HashSet<UUID>),
}

impl Default for EntityVisibility {
    fn default() -> Self {
        EntityVisibility::Exclude(HashSet::new())
    }
}

impl EntityVisibility {
    pub fn include(&mut self, uuid: UUID) {
        match self {
            EntityVisibility::Include(uuids) => {
                uuids.insert(uuid);
            }
            EntityVisibility::Exclude(uuids) => {
                uuids.remove(&uuid);
            }
        }
    }

    pub fn exclude(&mut self, uuid: UUID) {
        match self {
            EntityVisibility::Include(uuids) => {
                uuids.remove(&uuid);
            }
            EntityVisibility::Exclude(uuids) => {
                uuids.insert(uuid);
            }
        }
    }

    pub fn contains(&self, uuid: &UUID) -> bool {
        match self {
            EntityVisibility::Include(uuids) => uuids.contains(uuid),
            EntityVisibility::Exclude(uuids) => !uuids.contains(uuid),
        }
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
    animations: Vec<EntityAnimationType>,
    pub visibility: EntityVisibility,
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
            animations: Vec::new(),
            visibility: EntityVisibility::default(),
        }
    }

    pub fn animate(&mut self, animation: EntityAnimationType) {
        if !animation.can_stack() && self.animations.contains(&animation) {
            return;
        }
        self.animations.push(animation);
    }
}

#[derive(Debug)]
pub struct EntityViewer {
    connection: ConnectionSender,
    uuid: UUID,
    pub position: Vec3<f64>,
    pub radius: f64,
    viewing: HashSet<i32>,
}

impl EntityViewer {
    fn new(connection: ConnectionSender, uuid: UUID) -> Self {
        Self {
            connection,
            uuid,
            position: Vec3::zero(),
            radius: 256.0,
            viewing: HashSet::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct EntityManager {
    entities: WeakMap<i32, Mutex<EntityHandler>>,
    viewers: WeakList<Mutex<EntityViewer>>,
}

impl EntityManager {
    pub fn add_viewer(
        &mut self,
        connection: ConnectionSender,
        uuid: UUID,
    ) -> Arc<Mutex<EntityViewer>> {
        self.viewers
            .push(Mutex::new(EntityViewer::new(connection, uuid)))
    }

    pub fn update_viewers(&mut self, force_sync: bool) -> Result<(), ConnectionError> {
        self.viewers.cleanup();
        self.entities.cleanup();

        let mut entities = self.entities.lock();
        let mut viewers = self.viewers.lock();

        // Delete entities & create entities
        viewers.iter_mut().try_for_each(|viewer| {
            // Entities that the current viewer should have visible.
            let viewing_entities = entities
                .values()
                .flat_map(|entity| {
                    (entity.visibility.contains(&viewer.uuid)
                        //&& (entity.uuid != viewer.uuid)
                        && (entity.position.distance(&viewer.position) <= viewer.radius))
                        .then_some(entity.id)
                })
                .collect::<HashSet<i32>>();

            // Initialize new entities for viewer
            viewing_entities
                .difference(&viewer.viewing)
                .map(|id| entities.get(id).unwrap())
                .collect::<Vec<_>>()
                .into_iter()
                .try_for_each(|entity| {
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
                })?;

            // Remove not visible entities for viewers.
            let removed = viewer
                .viewing
                .difference(&viewing_entities)
                .cloned()
                .collect::<HashSet<i32>>();
            if !removed.is_empty() {
                removed.iter().for_each(|id| {
                    viewer.viewing.remove(id);
                });
                viewer
                    .connection
                    .send(&packet::play::RemoveEntities(removed))?;
            }

            Ok::<_, ConnectionError>(())
        })?;

        // Entity movement
        viewers.iter().try_for_each(|viewer| {
            entities
                .values()
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

        // Entity animation
        viewers.iter().try_for_each(|viewer| {
            entities.iter().try_for_each(|(entity_id, entity)| {
                if !viewer.viewing.contains(entity_id) {
                    return Ok(());
                }
                for animation in entity.animations.iter() {
                    viewer.connection.send(&packet::play::EntityAnimation {
                        entity_id: **entity_id,
                        r#type: *animation,
                    })?;
                }
                Ok::<_, ConnectionError>(())
            })
        })?;

        // End of processing cleanup.
        entities.values_mut().for_each(|entity| {
            entity.last_position = entity.position;
            entity.last_yaw_pitch = (entity.yaw, entity.pitch);
            entity.last_head_yaw = entity.head_yaw;
            entity.animations.drain(..);
        });

        Ok(())
    }

    pub fn add_entity<T: Entity>(&mut self, entity: T, uuid: UUID) -> EntityBase<T> {
        let id = new_entity_id();
        EntityBase {
            handler: self.entities.insert_ignored(
                id,
                Mutex::new(EntityHandler::new(id, uuid, entity.r#type().to_value())),
            ),
            inner: entity,
            uuid,
        }
    }
}
