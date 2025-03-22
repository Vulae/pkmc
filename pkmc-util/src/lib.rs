use std::collections::HashMap;

use itertools::Itertools as _;

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

/// https://minecraft.wiki/w/Formatting_codes
/// Converts text like: "&bHello!" to "§bHello!"
pub fn convert_ampersand_formatting_codes(text: &str) -> String {
    const SECTION_SYMBOL: char = '§';
    const SECTION_REPLACEMENT_SYMBOL: char = '&';
    #[rustfmt::skip]
    const VALID_CODES: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'k',
        'l', 'm', 'n', 'o', 'r',
    ];

    text.chars()
        .tuple_windows()
        .map(|(c, n)| match c {
            '\\' if n == SECTION_REPLACEMENT_SYMBOL => SECTION_REPLACEMENT_SYMBOL,
            SECTION_REPLACEMENT_SYMBOL if VALID_CODES.contains(&n) => SECTION_SYMBOL,
            c => c,
        })
        .chain(text.chars().last())
        .collect::<String>()
}
