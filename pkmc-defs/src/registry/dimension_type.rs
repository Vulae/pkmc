use serde::{Deserialize, Serialize};

use super::IntProvider;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DimensionType {
    fixed_time: Option<i64>,
    has_skylight: bool,
    has_ceiling: bool,
    ultrawarm: bool,
    natural: bool,
    coordinate_scale: f64,
    bed_works: bool,
    respawn_anchor_works: bool,
    min_y: i32,
    height: i32,
    logical_height: i32,
    infiniburn: String,
    effects: String,
    ambient_light: f32,
    piglin_safe: bool,
    has_raids: bool,
    monster_spawn_light_level: IntProvider,
    monster_spawn_block_light_limit: i32,
}
