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

    pub fn hue(hue: f32) -> Self {
        let mut rgb = [0f32; 3];
        rgb.iter_mut().enumerate().for_each(|(i, c)| {
            let h = hue + (i as f32) / 3.0;
            *c = f32::clamp(6.0 * f32::abs(h - f32::floor(h) - 0.5) - 1.0, 0.0, 1.0);
        });
        Self::new(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        )
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

    pub fn to_argb8888(&self, alpha: u8) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) | ((alpha as u32) << 24)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Self {
            r: value.0,
            g: value.1,
            b: value.2,
        }
    }
}

impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Self {
            r: value[0],
            g: value[1],
            b: value[2],
        }
    }
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
