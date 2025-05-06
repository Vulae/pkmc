use std::sync::{Arc, Mutex};

use pkmc_defs::{
    packet::{self, play::PlayerInfoPlayerProperties},
    text_component::TextComponent,
};
use pkmc_util::{
    connection::{ConnectionError, ConnectionSender},
    WeakList, WeakMap, UUID,
};

#[derive(Debug)]
pub struct TabListPlayer {
    uuid: UUID,
    name: String,
    pub game_mode: packet::play::Gamemode,
    pub listed: bool,
    pub latency: i32,
    pub display_name: Option<TextComponent>,
    pub priority: i32,
    pub hat: bool,
    pub properties: PlayerInfoPlayerProperties,
}

impl TabListPlayer {
    pub fn new(uuid: UUID, name: String, properties: PlayerInfoPlayerProperties) -> Self {
        Self {
            uuid,
            name,
            game_mode: packet::play::Gamemode::Survival,
            listed: true,
            latency: 0,
            display_name: None,
            priority: 0,
            hat: true,
            properties,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }
}

#[derive(Debug)]
pub struct TabListViewer {
    connection: ConnectionSender,
    _uuid: UUID,
}

#[derive(Debug, Default)]
pub struct TabList {
    entries: WeakMap<UUID, Mutex<TabListPlayer>>,
    // TODO: Optimize what data gets sent, send minimum data needed by keeping track of the
    // difference between viewer updates.
    force_update: bool,
    viewers: WeakList<Mutex<TabListViewer>>,
}

impl TabList {
    pub fn add_viewer(
        &mut self,
        connection: ConnectionSender,
        uuid: UUID,
    ) -> Result<Arc<Mutex<TabListViewer>>, ConnectionError> {
        connection.send(&packet::play::PlayerInfoUpdate(
            self.entries
                .lock()
                .values()
                .map(|entry| {
                    (
                        entry.uuid,
                        vec![
                            packet::play::PlayerInfoUpdateAction::AddPlayer {
                                name: entry.name.clone(),
                                properties: &entry.properties,
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

        Ok(self.viewers.push(Mutex::new(TabListViewer {
            connection,
            _uuid: uuid,
        })))
    }

    pub fn update_viewers(&mut self) -> Result<(), ConnectionError> {
        self.viewers.cleanup();

        let removed_entries = self.entries.cleanup();
        if !removed_entries.is_empty() {
            self.force_update = true;
        }

        if !self.force_update {
            return Ok(());
        }
        self.force_update = false;

        let mut viewers = self.viewers.lock();
        let entries = self.entries.lock();

        viewers.iter_mut().try_for_each(|viewer| {
            if !removed_entries.is_empty() {
                viewer
                    .connection
                    .send(&packet::play::PlayerInfoRemove(removed_entries.clone()))?;
            }

            viewer.connection.send(&packet::play::PlayerInfoUpdate(
                entries
                    .values()
                    .map(|entry| {
                        (
                            entry.uuid,
                            vec![
                                packet::play::PlayerInfoUpdateAction::AddPlayer {
                                    name: entry.name.clone(),
                                    properties: &entry.properties,
                                },
                                packet::play::PlayerInfoUpdateAction::InitializeChat,
                                packet::play::PlayerInfoUpdateAction::UpdateGamemode(
                                    entry.game_mode,
                                ),
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
            ))
        })?;

        Ok(())
    }

    pub fn insert(&mut self, tab_list_player: TabListPlayer) -> Arc<Mutex<TabListPlayer>> {
        self.force_update = true;
        self.entries
            .insert_ignored(tab_list_player.uuid, Mutex::new(tab_list_player))
    }
}
