use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum DamageTypeScaling {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "when_caused_by_living_non_player")]
    WhenCausedByLivingNonPlayer,
    #[serde(rename = "always")]
    Always,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum DamageTypeEffects {
    #[serde(rename = "hurt")]
    #[default]
    Hurt,
    #[serde(rename = "thorns")]
    Thorns,
    #[serde(rename = "drowning")]
    Drowning,
    #[serde(rename = "burning")]
    Burning,
    #[serde(rename = "poking")]
    Poking,
    #[serde(rename = "freezing")]
    Freezing,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum DamageTypeDeathMessageType {
    #[serde(rename = "default")]
    #[default]
    Default,
    #[serde(rename = "fall_variants")]
    FallVariants,
    #[serde(rename = "intentional_game_design")]
    IntentionalGameDesign,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DamageType {
    message_id: String,
    scaling: DamageTypeScaling,
    exhaustion: f32,
    #[serde(default)]
    effects: DamageTypeEffects,
    #[serde(default)]
    death_message_type: DamageTypeDeathMessageType,
}
