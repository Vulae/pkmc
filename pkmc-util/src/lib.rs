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

    // I have absolutely no idea on how to properly escape this to actually make sense.
    // Like, it would probably be best not to mess with other escapes in the string.
    // Currently it does something like: b"\\ \&r \\&r" -> b"\ &r \§r", which doesn't make much sense that
    // it's doing to the first escape sequence.
    // Probably should just only mess with the escapes that are near the formatting codes: b"\\ &r \§r"
    let mut escape = false;
    text.chars()
        .tuple_windows()
        .flat_map(|(c, n)| {
            if escape {
                escape = false;
                return Some(c);
            }
            match c {
                '\\' if matches!(n, '\\' | SECTION_REPLACEMENT_SYMBOL) => {
                    escape = true;
                    None
                }
                SECTION_REPLACEMENT_SYMBOL if VALID_CODES.contains(&n) => Some(SECTION_SYMBOL),
                c => Some(c),
            }
        })
        .chain(text.chars().last())
        .collect::<String>()
}

#[cfg(test)]
mod test {
    use crate::convert_ampersand_formatting_codes;

    #[test]
    fn minecraft_formatting_codes() {
        assert_eq!(&convert_ampersand_formatting_codes(r"&r"), r"§r");
        assert_eq!(&convert_ampersand_formatting_codes(r"&z"), r"&z");
        assert_eq!(&convert_ampersand_formatting_codes(r"\&r"), r"&r");
        assert_eq!(&convert_ampersand_formatting_codes(r"\\&r"), r"\§r");
        //assert_eq!(&convert_ampersand_formatting_codes(r"\\ \\&r"), r"\\ \§r");
    }
}
