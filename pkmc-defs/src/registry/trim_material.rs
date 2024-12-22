use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::FormattedText;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TrimMaterial {
    pub asset_name: String,
    pub ingredient: String,
    pub item_model_index: f32,
    #[serde(default)]
    pub override_armor_materials: HashMap<String, String>,
    pub description: FormattedText,
}
