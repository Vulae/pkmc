use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Serialize)]
pub struct BannerPattern {
    pub asset_id: String,
    pub translation_key: String,
}

#[derive(Deserialize, Serialize)]
pub struct ChatTypeDecoration {
    pub parameters: Vec<String>,
    pub translation_key: String,
    // TODO: please implement: https://wiki.vg/Registry_Data#Decoration
    //style: Option<_>,
}

#[derive(Deserialize, Serialize)]
pub struct ChatType {
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}

#[derive(Deserialize, Serialize)]
pub struct DamageType {
    pub message_id: String,
    pub scaling: String,
    pub exhaustion: f32,
    pub effects: Option<String>,
    pub death_message_type: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct DimensionType {
    pub fixed_time: Option<i64>,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub has_skylight: bool,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub has_ceiling: bool,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub ultrawarm: bool,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub natural: bool,
    pub coordinate_scale: f64,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub bed_works: bool,
    pub respawn_anchor_works: bool,
    pub min_y: i32,
    pub height: i32,
    pub logical_height: i32,
    pub infiniburn: String,
    pub effects: String,
    pub ambient_light: f32,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub piglin_safe: bool,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub has_raids: bool,
    // TODO: please implement: https://wiki.vg/Registry_Data#Dimension_Type
    //pub monster_spawn_light_level: _,
    pub monster_spawn_block_light_limit: i32,
}

#[derive(Deserialize, Serialize)]
pub struct PaintingVariant {
    pub asset_id: String,
    pub height: i32,
    pub width: i32,
}

#[derive(Deserialize, Serialize)]
pub struct TrimMaterialOveride {
    pub asset_name: String,
}

#[derive(Deserialize, Serialize)]
pub struct TrimMaterial {
    pub asset_name: String,
    pub ingredient: String,
    pub item_model_index: f32,
    pub override_armor_materials: Option<TrimMaterialOveride>,
    // TODO: implement: https://wiki.vg/Text_formatting
    //pub description: _,
}

#[derive(Deserialize, Serialize)]
pub struct TrimPattern {
    pub asset_id: String,
    pub template_item: String,
    // TODO: implement: https://wiki.vg/Text_formatting
    //pub description: _,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    pub decal: bool,
}

#[derive(Deserialize, Serialize)]
pub struct WolfVariant {
    pub wild_texture: String,
    pub tame_texture: String,
    pub angry_texture: String,
    // TODO: May also be Vec<String>, but is never used like that.
    pub biomes: String,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiomeEffectsAmbientSound {
    sound_id: String,
    range: Option<f32>,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiomeEffectsMoodSound {
    sound: String,
    tick_delay: i32,
    block_search_extent: i32,
    offset: f64,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiomeEffectsAdditionsSound {
    sound: String,
    tick_chance: f64,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiomeEffectsMusic {
    sound: String,
    min_delay: i32,
    max_delay: i32,
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    replace_current_music: bool,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiomeEffects {
    pub fog_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub sky_color: i32,
    pub foliage_color: Option<i32>,
    pub grass_color: Option<i32>,
    pub grass_color_modifier: Option<String>,
    // TODO: Implement: https://wiki.vg/Registry_Data#Ambient_sound
    //pub ambient_sounds: Option<_>,
    pub mood_sound: Option<WorldgenBiomeEffectsMoodSound>,
    pub additions_sound: Option<WorldgenBiomeEffectsAdditionsSound>,
    pub music: Option<WorldgenBiomeEffectsMusic>,
}

#[derive(Deserialize, Serialize)]
pub struct WorldgenBiome {
    #[serde(deserialize_with = "bool_from_num", serialize_with = "bool_to_num")]
    has_precipitation: bool,
    temperature: f32,
    temperature_modifier: Option<String>,
    downfall: f32,
    effects: WorldgenBiomeEffects,
}

#[derive(Deserialize, Serialize)]
pub struct Registry {
    #[serde(rename = "minecraft:banner_pattern")]
    minecraft_banner_pattern: HashMap<String, BannerPattern>,
    #[serde(rename = "minecraft:chat_type")]
    minecraft_chat_type: HashMap<String, ChatType>,
    #[serde(rename = "minecraft:damage_type")]
    minecraft_damage_type: HashMap<String, DamageType>,
    #[serde(rename = "minecraft:dimension_type")]
    minecraft_dimension_type: HashMap<String, DimensionType>,
    #[serde(rename = "minecraft:painting_variant")]
    minecraft_painting_variant: HashMap<String, PaintingVariant>,
    #[serde(rename = "minecraft:trim_material")]
    minecraft_trim_material: HashMap<String, TrimMaterial>,
    #[serde(rename = "minecraft:trim_pattern")]
    minecraft_trim_pattern: HashMap<String, TrimPattern>,
    #[serde(rename = "minecraft:wolf_variant")]
    minecraft_wolf_variant: HashMap<String, WolfVariant>,
    #[serde(rename = "minecraft:worldgen/biome")]
    minecraft_worldgen_biome: HashMap<String, WorldgenBiome>,
}

fn bool_from_num<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}

fn bool_to_num<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match *value {
        false => serializer.serialize_u8(0),
        true => serializer.serialize_u8(1),
    }
}
