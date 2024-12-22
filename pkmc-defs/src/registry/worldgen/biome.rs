use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum BiomeTemperatureModifier {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "frozen")]
    Frozen,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum BiomeEffectsGrassColorModifier {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "dark_forest")]
    DarkForest,
    #[serde(rename = "swamp")]
    Swamp,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BiomeEffects {
    pub fog_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub sky_color: i32,
    pub foliage_color: Option<i32>,
    pub grass_color: Option<i32>,
    #[serde(default)]
    pub grass_color_modifier: BiomeEffectsGrassColorModifier,
    // TODO:
    pub particle: Option<()>,
    // TODO:
    pub ambient_sound: Option<()>,
    // TODO:
    pub mood_sound: Option<()>,
    // TODO:
    pub additions_sound: Option<()>,
    // TODO:
    pub music: Option<()>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Biome {
    pub has_precipitation: bool,
    pub temperature: f32,
    #[serde(default)]
    pub temperature_modifier: BiomeTemperatureModifier,
    pub downfall: f32,
    pub effects: BiomeEffects,
}
