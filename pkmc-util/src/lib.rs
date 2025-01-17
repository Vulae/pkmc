use std::collections::HashMap;

mod iter_retain;
pub mod nbt;
mod packed_array;
pub mod packet;
mod read_ext;
mod transmutable;
mod uuid;

pub use iter_retain::*;
pub use packed_array::*;
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
