use std::{
    collections::HashSet,
    fmt::Debug,
    sync::{atomic::AtomicI32, Arc, Mutex, Weak},
};

use pkmc_defs::packet;
use pkmc_util::{
    packet::{ConnectionError, ConnectionSender},
    UUID,
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
}

impl EntityHandler {
    fn new(id: i32, uuid: UUID, r#type: i32) -> Self {
        Self { id, uuid, r#type }
    }
}

#[derive(Debug)]
pub struct EntityViewer {
    connection: ConnectionSender,
    viewing: HashSet<i32>,
}

impl EntityViewer {
    fn new(connection: ConnectionSender) -> Self {
        Self {
            connection,
            viewing: HashSet::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct EntityManager {
    entities: Vec<Weak<Mutex<EntityHandler>>>,
    viewers: Vec<Weak<Mutex<EntityViewer>>>,
}

impl EntityManager {
    pub fn add_viewer(&mut self, connection: ConnectionSender) -> Arc<Mutex<EntityViewer>> {
        let viewer = Arc::new(Mutex::new(EntityViewer::new(connection)));
        self.viewers.push(Arc::downgrade(&viewer));
        viewer
    }

    pub fn update_viewers(&mut self) -> Result<(), ConnectionError> {
        self.viewers.retain(|v| v.strong_count() > 0);

        let viewers = self
            .viewers
            .iter()
            .flat_map(|v| v.upgrade())
            .collect::<Vec<_>>();

        self.entities.retain(|e| e.strong_count() > 0);

        let entities = self
            .entities
            .iter()
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
                        if viewer.viewing.contains(&entity.id) {
                            return Ok(());
                        }
                        viewer.viewing.insert(entity.id);
                        viewer.connection.send(&packet::play::AddEntity {
                            id: entity.id,
                            uuid: entity.uuid,
                            r#type: entity.r#type,
                            x: 0.0,
                            y: 100.0,
                            z: 0.0,
                            pitch: 0,
                            yaw: 0,
                            head_yaw: 0,
                            data: 0,
                            velocity_x: 0,
                            velocity_y: 0,
                            velocity_z: 0,
                        })?;
                        Ok::<_, ConnectionError>(())
                    })
            })?;

        Ok(())
    }

    pub fn add_entity<T: Entity>(&mut self, entity: T, uuid: UUID) -> EntityBase<T> {
        let entity = EntityBase::new(entity, uuid);
        self.entities.push(Arc::downgrade(&entity.handler));
        entity
    }
}
