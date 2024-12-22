#![allow(unused)]
use pkmc_nbt::NBT;
/// https://minecraft.wiki/w/Raw_JSON_text_format
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text {
        text: String,
    },
    Translatable {
        translate: String,
        fallback: Option<String>,
        with: Vec<TextComponent>,
    },
    // TODO: Score,
    // TODO: Selector,
    Keybind {
        keybind: String,
    },
    // TODO: Nbt,
}

impl Default for Content {
    fn default() -> Self {
        Self::Text {
            text: String::new(),
        }
    }
}

impl Content {
    fn insert_map(&self, map: &mut serde_json::Map<String, serde_json::Value>) {
        match self {
            Content::Text { text } => {
                map.insert("type".to_owned(), "text".into());
                map.insert("text".to_owned(), text.to_owned().into());
            }
            Content::Translatable {
                translate,
                fallback,
                with,
            } => todo!(),
            Content::Keybind { keybind } => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const BLACK: Color = Color::new(0x00, 0x00, 0x00);
    pub const DARK_BLUE: Color = Color::new(0x00, 0x00, 0xAA);
    pub const DARK_GREEN: Color = Color::new(0x00, 0xAA, 0x00);
    pub const DARK_AQUA: Color = Color::new(0x00, 0xAA, 0xAA);
    pub const DARK_RED: Color = Color::new(0xAA, 0x00, 0x00);
    pub const DARK_PURPLE: Color = Color::new(0xAA, 0x00, 0xAA);
    pub const GOLD: Color = Color::new(0xFF, 0xAA, 0x00);
    pub const GRAY: Color = Color::new(0xAA, 0xAA, 0xAA);
    pub const DARK_GRAY: Color = Color::new(0x55, 0x55, 0x55);
    pub const BLUE: Color = Color::new(0x55, 0x55, 0xFF);
    pub const GREEN: Color = Color::new(0x55, 0xFF, 0x55);
    pub const AQUA: Color = Color::new(0x55, 0xFF, 0xFF);
    pub const RED: Color = Color::new(0xFF, 0x55, 0x55);
    pub const LIGHT_PURPLE: Color = Color::new(0xFF, 0x55, 0xFF);
    pub const YELLOW: Color = Color::new(0xFF, 0xFF, 0x55);
    pub const WHITE: Color = Color::new(0xFF, 0xFF, 0xFF);
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Color::BLACK => write!(f, "black"),
            &Color::DARK_BLUE => write!(f, "dark_blue"),
            &Color::DARK_GREEN => write!(f, "dark_green"),
            &Color::DARK_AQUA => write!(f, "dark_aqua"),
            &Color::DARK_RED => write!(f, "dark_red"),
            &Color::DARK_PURPLE => write!(f, "dark_purple"),
            &Color::GOLD => write!(f, "gold"),
            &Color::GRAY => write!(f, "gray"),
            &Color::DARK_GRAY => write!(f, "dark_gray"),
            &Color::BLUE => write!(f, "blue"),
            &Color::GREEN => write!(f, "green"),
            &Color::AQUA => write!(f, "aqua"),
            &Color::RED => write!(f, "red"),
            &Color::LIGHT_PURPLE => write!(f, "light_purple"),
            &Color::YELLOW => write!(f, "yellow"),
            &Color::WHITE => write!(f, "white"),
            Color { r, g, b } => write!(f, "#{:02X}{:02X}{:02X}", r, g, b),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Formatting {
    color: Option<Color>,
    font: String,
    bold: bool,
    // Items names may be italic by default.
    // So we cannot just assume non-italic as default.
    italic: Option<bool>,
    underline: bool,
    strikethrough: bool,
    obfuscated: bool,
}

impl Formatting {
    fn insert_map(&self, map: &mut serde_json::Map<String, serde_json::Value>) {
        if let Some(color) = self.color {
            map.insert("color".to_owned(), color.to_string().into());
        }
        if !self.font.is_empty() && self.font != "minecraft:default" {
            map.insert("font".to_owned(), self.font.clone().into());
        }
        if self.bold {
            map.insert("bold".to_owned(), self.bold.into());
        }
        if let Some(italic) = self.italic {
            map.insert("italic".to_owned(), italic.into());
        }
        if self.underline {
            map.insert("underline".to_owned(), self.underline.into());
        }
        if self.strikethrough {
            map.insert("strikethrough".to_owned(), self.strikethrough.into());
        }
        if self.obfuscated {
            map.insert("obfuscated".to_owned(), self.obfuscated.into());
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextComponent {
    content: Content,
    formatting: Formatting,
    // TODO: Children
    // TODO: Interactivity
}

impl TextComponent {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            content: Content::Text { text: text.into() },
            ..Default::default()
        }
    }

    pub fn with_color<C: Into<Option<Color>>>(mut self, color: C) -> Self {
        self.formatting.color = color.into();
        self
    }

    pub fn with_font<S: Into<String>>(mut self, font: S) -> Self {
        self.formatting.font = font.into();
        self
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.formatting.bold = bold;
        self
    }

    pub fn with_italic<B: Into<Option<bool>>>(mut self, italic: B) -> Self {
        self.formatting.italic = italic.into();
        self
    }

    pub fn with_underline(mut self, underline: bool) -> Self {
        self.formatting.underline = underline;
        self
    }

    pub fn with_strikethrough(mut self, strikethrough: bool) -> Self {
        self.formatting.strikethrough = strikethrough;
        self
    }

    pub fn with_obfuscated(mut self, obfuscated: bool) -> Self {
        self.formatting.obfuscated = obfuscated;
        self
    }
}

impl TextComponent {
    pub fn to_json(&self) -> serde_json::Value {
        if let Content::Text { text } = &self.content {
            if self.formatting == Formatting::default() {
                return serde_json::Value::String(text.to_owned());
            }
        }
        let mut map = serde_json::Map::new();
        self.content.insert_map(&mut map);
        self.formatting.insert_map(&mut map);
        serde_json::Value::Object(map)
    }

    pub fn to_nbt(&self) -> NBT {
        NBT::try_from(self.to_json()).unwrap()
    }
}

impl From<String> for TextComponent {
    fn from(val: String) -> Self {
        TextComponent::new(val)
    }
}

impl From<&str> for TextComponent {
    fn from(val: &str) -> Self {
        TextComponent::new(val)
    }
}

//#[cfg(test)]
//mod test {
//    use super::{Color, TextComponent};
//
//    #[test]
//    pub fn simple() {
//        let component = TextComponent::new("Hello, World!")
//            .with_color(Color::GOLD)
//            .with_bold(true)
//            .with_italic(true)
//            .with_underline(true);
//        println!("{:#?}", component);
//        println!("{:#?}", component.to_json());
//        println!("{:#?}", component.to_nbt());
//    }
//}
