use std::collections::{BTreeMap, HashMap};

pub type Registry = BTreeMap<String, serde_json::Value>;
pub type Registries = HashMap<String, Registry>;
