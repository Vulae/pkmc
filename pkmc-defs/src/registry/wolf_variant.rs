use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WolfVariant {
    wild_texture: String,
    tame_texture: String,
    angry_texture: String,
    // TODO: Is this correct? It may be either String or Vec<String>, Hopefully this converts it to Vec<String>
    #[serde(flatten)]
    biomes: Vec<String>,
}
