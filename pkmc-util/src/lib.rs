use std::collections::HashMap;

mod bitset;
mod color;
pub mod connection;
pub mod crypto;
mod iter_retain;
pub mod nbt;
mod packed_array;
mod position;
mod read_ext;
mod transmutable;
mod uuid;
mod vec3;
mod weak_collections;

pub use bitset::*;
pub use color::*;
pub use iter_retain::*;
pub use packed_array::*;
pub use position::*;
pub use read_ext::*;
pub use transmutable::*;
pub use uuid::*;
pub use vec3::*;
pub use weak_collections::*;

pub type IdTable<T> = HashMap<T, i32>;

pub fn normalize_identifier(identifier: &str, default_namespace: &str) -> String {
    if identifier.contains(":") {
        return identifier.to_owned();
    }
    format!("{}:{}", default_namespace, identifier)
}
