use std::collections::HashMap;

mod iter_retain;
pub mod nbt;
mod packed_array;
pub mod packet;
mod position;
mod read_ext;
mod transmutable;
mod uuid;

pub use iter_retain::*;
pub use packed_array::*;
pub use position::*;
pub use read_ext::*;
pub use transmutable::*;
pub use uuid::*;

pub type IdTable<T> = HashMap<T, i32>;

pub fn normalize_identifier(identifier: &str, default_namespace: &str) -> String {
    if identifier.contains(":") {
        return identifier.to_owned();
    }
    format!("{}:{}", default_namespace, identifier)
}

pub fn get_vector_for_rotation(pitch: f32, yaw: f32) -> (f32, f32, f32) {
    let f = f32::cos(-yaw * 0.017453292 - std::f32::consts::PI);
    let f1 = f32::sin(-yaw * 0.017453292 - std::f32::consts::PI);
    let f2 = -f32::cos(-pitch * 0.017453292);
    let f3 = f32::sin(-pitch * 0.017453292);
    (f1 * f2, f3, f * f2)
}
