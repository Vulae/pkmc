use std::collections::HashMap;

use serde::Deserialize;

pub mod block;
pub mod generated;
pub mod packet;
pub mod registry;
pub mod text_component;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Registry {
    entries: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl Registry {
    pub fn load() -> Self {
        serde_json::from_str(include_str!("./registry.json")).unwrap()
    }

    pub fn iter_entries(
        &self,
    ) -> impl Iterator<Item = (&String, &HashMap<String, serde_json::Value>)> {
        self.entries.iter()
    }

    pub fn get_entries(&self, name: &str) -> Option<&HashMap<String, serde_json::Value>> {
        self.entries.get(name)
    }
}
