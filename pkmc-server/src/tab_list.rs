use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, Weak},
};

use pkmc_defs::{packet, text_component::TextComponent};
use pkmc_util::{
    packet::{ConnectionError, ConnectionSender},
    retain_returned_hashmap, UUID,
};

#[derive(Debug)]
pub struct TabListPlayer {
    uuid: UUID,
    name: String,
    pub game_mode: i32,
    pub listed: bool,
    pub latency: i32,
    pub display_name: Option<TextComponent>,
    pub priority: i32,
    pub hat: bool,
}

impl TabListPlayer {
    pub fn new(uuid: UUID, name: String) -> Self {
        Self {
            uuid,
            name,
            game_mode: 0,
            listed: true,
            latency: 0,
            display_name: None,
            priority: 0,
            hat: true,
        }
    }
}

#[derive(Debug)]
pub struct TabListViewer {
    connection: ConnectionSender,
    _uuid: UUID,
}

#[derive(Debug, Default)]
pub struct TabList {
    entries: HashMap<UUID, Weak<Mutex<TabListPlayer>>>,
    // TODO: Optimize what data gets sent, send minimum data needed by keeping track of the
    // difference between viewer updates.
    force_update: bool,
    viewers: Vec<Weak<Mutex<TabListViewer>>>,
}

impl TabList {
    pub fn add_viewer(
        &mut self,
        connection: ConnectionSender,
        uuid: UUID,
    ) -> Result<Arc<Mutex<TabListViewer>>, ConnectionError> {
        self.entries.retain(|_, v| v.strong_count() > 0);

        let entries = self
            .entries
            .values()
            .flat_map(|v| v.upgrade())
            .collect::<Vec<_>>();

        connection.send(&packet::play::PlayerInfoUpdate(
            entries
                .iter()
                .map(|v| v.lock().unwrap())
                .map(|entry| {
                    (
                        entry.uuid,
                        vec![
                            packet::play::PlayerInfoUpdateAction::AddPlayer {
                                name: entry.name.clone(),
                                properties: HashMap::new(),
                            },
                            packet::play::PlayerInfoUpdateAction::InitializeChat,
                            packet::play::PlayerInfoUpdateAction::UpdateGamemode(entry.game_mode),
                            packet::play::PlayerInfoUpdateAction::UpdateListed(entry.listed),
                            packet::play::PlayerInfoUpdateAction::UpdateLatency(entry.latency),
                            packet::play::PlayerInfoUpdateAction::UpdateDisplayName(
                                entry.display_name.clone(),
                            ),
                            packet::play::PlayerInfoUpdateAction::UpdateListPriority(
                                entry.priority,
                            ),
                            packet::play::PlayerInfoUpdateAction::UpdateHat(entry.hat),
                        ],
                    )
                })
                .collect(),
        ))?;

        let viewer = Arc::new(Mutex::new(TabListViewer {
            connection,
            _uuid: uuid,
        }));
        self.viewers.push(Arc::downgrade(&viewer));
        Ok(viewer)
    }

    pub fn update_viewers(&mut self) -> Result<(), ConnectionError> {
        let removed_entries =
            retain_returned_hashmap(&mut self.entries, |_, v| v.strong_count() > 0)
                .into_iter()
                .map(|(uuid, _)| uuid)
                .collect::<HashSet<_>>();
        if !removed_entries.is_empty() {
            self.force_update = true;
        }

        self.viewers.retain(|v| v.strong_count() > 0);

        if !self.force_update {
            return Ok(());
        }
        self.force_update = false;

        let entries = self
            .entries
            .values()
            .flat_map(|v| v.upgrade())
            .collect::<Vec<_>>();

        let viewers = self
            .viewers
            .iter()
            .flat_map(|v| v.upgrade())
            .collect::<Vec<_>>();

        viewers
            .iter()
            .map(|v| v.lock().unwrap())
            .try_for_each(|viewer| {
                if !removed_entries.is_empty() {
                    viewer
                        .connection
                        .send(&packet::play::PlayerInfoRemove(removed_entries.clone()))?;
                }

                viewer.connection.send(&packet::play::PlayerInfoUpdate(
                    entries
                        .iter()
                        .map(|v| v.lock().unwrap())
                        .map(|entry| {
                            (
                                entry.uuid,
                                vec![
                                    packet::play::PlayerInfoUpdateAction::AddPlayer {
                                        name: entry.name.clone(),
                                        properties: HashMap::new(),
                                    },
                                    packet::play::PlayerInfoUpdateAction::InitializeChat,
                                    packet::play::PlayerInfoUpdateAction::UpdateGamemode(
                                        entry.game_mode,
                                    ),
                                    packet::play::PlayerInfoUpdateAction::UpdateListed(
                                        entry.listed,
                                    ),
                                    packet::play::PlayerInfoUpdateAction::UpdateLatency(
                                        entry.latency,
                                    ),
                                    packet::play::PlayerInfoUpdateAction::UpdateDisplayName(
                                        entry.display_name.clone(),
                                    ),
                                    packet::play::PlayerInfoUpdateAction::UpdateListPriority(
                                        entry.priority,
                                    ),
                                    packet::play::PlayerInfoUpdateAction::UpdateHat(entry.hat),
                                ],
                            )
                        })
                        .collect(),
                ))
            })?;

        Ok(())
    }

    pub fn insert(&mut self, uuid: UUID, name: String) -> Arc<Mutex<TabListPlayer>> {
        let tab_list_player = Arc::new(Mutex::new(TabListPlayer::new(uuid, name)));
        self.entries.insert(uuid, Arc::downgrade(&tab_list_player));
        self.force_update = true;
        tab_list_player
    }
}
