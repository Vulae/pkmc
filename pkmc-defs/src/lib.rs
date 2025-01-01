use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

pub mod block;
pub mod generated;
pub mod packet;
pub mod registry;
pub mod text_component;

pub static REGISTRY: LazyLock<HashMap<String, BTreeMap<String, serde_json::Value>>> =
    LazyLock::new(|| serde_json::from_str(include_str!("./registry.json")).unwrap());
