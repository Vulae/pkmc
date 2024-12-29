pub mod iter_retain;
pub mod packed_array;
pub mod read_ext;
pub mod transmutable;
pub mod uuid;

use std::collections::HashMap;

pub use iter_retain::IterRetain;
pub use packed_array::PackedArray;
pub use read_ext::ReadExt;
pub use transmutable::Transmutable;
pub use uuid::UUID;

pub type IdTable<T> = HashMap<T, i32>;
