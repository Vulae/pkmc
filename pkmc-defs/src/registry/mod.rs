use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum FormattedText {
    Unformatted(String),
    Formatted { color: String, translate: String },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum IntProvider {
    Int(i32),
    // TODO: It's somewhere in there: https://minecraft.wiki/w/Dimension_type
    Provider(),
}

pub type Registry = BTreeMap<String, serde_json::Value>;
pub type Registries = HashMap<String, Registry>;

pub mod worldgen;

pub mod banner_pattern;
pub mod chat_type;
pub mod damage_type;
pub mod dimension_type;
pub mod painting_variant;
pub mod trim_material;
pub mod trim_pattern;
pub mod wolf_variant;
