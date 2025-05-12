use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use pkmc_defs::dimension::Dimension;
use pkmc_server::{
    entity_manager::EntityManager,
    level::anvil::{AnvilLevel, AnvilWorld},
    tab_list::TabList,
};

use super::tab_info::ServerTabInfo;

#[derive(Debug, Clone)]
pub struct ServerStateLevel {
    pub level: Arc<Mutex<AnvilLevel>>,
    pub entities: Arc<Mutex<EntityManager>>,
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub world: Arc<Mutex<AnvilWorld>>,
    levels: HashMap<Dimension, ServerStateLevel>,
    pub tab_list: Arc<Mutex<TabList>>,
    pub server_tab_info: Arc<Mutex<ServerTabInfo>>,
}

impl ServerState {
    pub fn new(world: AnvilWorld) -> Self {
        Self {
            levels: world
                .iter_levels()
                .map(|(dimension, level)| {
                    (
                        dimension.clone(),
                        ServerStateLevel {
                            level: level.clone(),
                            entities: Arc::new(Mutex::new(EntityManager::default())),
                        },
                    )
                })
                .collect(),
            world: Arc::new(Mutex::new(world)),
            tab_list: Arc::new(Mutex::new(TabList::default())),
            server_tab_info: Arc::new(Mutex::new(ServerTabInfo::new())),
        }
    }

    pub fn get_level_state(&self, level: &Dimension) -> Option<ServerStateLevel> {
        self.levels.get(level).cloned()
    }

    pub fn iter_levels(&self) -> impl Iterator<Item = (&Dimension, &ServerStateLevel)> {
        self.levels.iter()
    }
}
