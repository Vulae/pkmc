use pkmc_util::{nbt::NBT, Color};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Keybind {
    Jump,
    Sneak,
    Sprint,
    StrafeLeft,
    StrafeRight,
    WalkBackward,
    WalkForward,
    Attack_Destroy,
    PickBlock,
    UseItem_PlaceBlock,
    DropSelectedItem,
    HotbarSlot1,
    HotbarSlot2,
    HotbarSlot3,
    HotbarSlot4,
    HotbarSlot5,
    HotbarSlot6,
    HotbarSlot7,
    HotbarSlot8,
    HotbarSlot9,
    OpenInventory_CloseInventory,
    SwapItemsInHands,
    LoadToolbarActivator,
    SaveToolbarActivator,
    ListPlayers,
    OpenChat,
    OpenCommand,
    SocialInteractionsScreen,
    Advancements,
    HightlightPlayers_Spectator,
    TakeScreenshot,
    ToggleCinematicCamera,
    ToggleFullscreen,
    TogglePerspective,
}

impl Keybind {
    pub fn identifier(&self) -> &str {
        match self {
            Keybind::Jump => "key.jump",
            Keybind::Sneak => "key.sneak",
            Keybind::Sprint => "key.sprint",
            Keybind::StrafeLeft => "key.left",
            Keybind::StrafeRight => "key.right",
            Keybind::WalkBackward => "key.back",
            Keybind::WalkForward => "key.forward",
            Keybind::Attack_Destroy => "key.attack",
            Keybind::PickBlock => "key.pickItem",
            Keybind::UseItem_PlaceBlock => "key.use",
            Keybind::DropSelectedItem => "key.drop",
            Keybind::HotbarSlot1 => "key.hotbar.1",
            Keybind::HotbarSlot2 => "key.hotbar.2",
            Keybind::HotbarSlot3 => "key.hotbar.3",
            Keybind::HotbarSlot4 => "key.hotbar.4",
            Keybind::HotbarSlot5 => "key.hotbar.5",
            Keybind::HotbarSlot6 => "key.hotbar.6",
            Keybind::HotbarSlot7 => "key.hotbar.7",
            Keybind::HotbarSlot8 => "key.hotbar.8",
            Keybind::HotbarSlot9 => "key.hotbar.9",
            Keybind::OpenInventory_CloseInventory => "key.inventory",
            Keybind::SwapItemsInHands => "key.swapOffhand",
            Keybind::LoadToolbarActivator => "key.loadToolbarActivator",
            Keybind::SaveToolbarActivator => "key.saveToolbarActivator",
            Keybind::ListPlayers => "key.playerlist",
            Keybind::OpenChat => "key.chat",
            Keybind::OpenCommand => "key.command",
            Keybind::SocialInteractionsScreen => "key.socialInteractions",
            Keybind::Advancements => "key.advancements",
            Keybind::HightlightPlayers_Spectator => "key.spectatorOutlines",
            Keybind::TakeScreenshot => "key.screenshot",
            Keybind::ToggleCinematicCamera => "key.smoothCamera",
            Keybind::ToggleFullscreen => "key.fullscreen",
            Keybind::TogglePerspective => "key.togglePerspective",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text { text: String },
    // TODO: Translatable,
    // TODO: Score,
    // TODO: Selector,
    Keybind { keybind: Keybind },
    // TODO: Nbt,
}

impl Default for Content {
    fn default() -> Self {
        Self::Text {
            text: String::new(),
        }
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self::Text { text: value }
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::Text {
            text: value.to_owned(),
        }
    }
}

impl From<char> for Content {
    fn from(value: char) -> Self {
        Self::Text {
            text: value.to_string(),
        }
    }
}

impl From<Keybind> for Content {
    fn from(value: Keybind) -> Self {
        Self::Keybind { keybind: value }
    }
}

impl Content {
    fn insert_map(&self, map: &mut serde_json::Map<String, serde_json::Value>) {
        match self {
            Content::Text { text } => {
                //map.insert("type".to_owned(), "text".into());
                map.insert("text".to_owned(), text.to_owned().into());
            }
            Content::Keybind { keybind } => {
                //map.insert("type".to_owned(), "keybind".into());
                map.insert("keybind".to_owned(), keybind.identifier().into());
            }
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
    children: Vec<TextComponent>,
    inherited_formatting: Option<Formatting>,
    // TODO: Interactivity
}

impl TextComponent {
    pub fn new<C: Into<Content>>(content: C) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    pub fn empty() -> Self {
        // TODO: If no content type is specified, would it still work and render the children?
        Self {
            content: Content::Text {
                text: "".to_owned(),
            },
            ..Default::default()
        }
    }

    pub fn rainbow(text: &str, hue_offset: f32) -> Self {
        text.chars()
            .enumerate()
            .fold(TextComponent::empty(), |text_component, (index, char)| {
                let percent = (index as f32) / ((text.len() - 1) as f32);
                text_component.with_child(|child| {
                    child
                        .with_content(char)
                        .with_color(Color::hue(percent + hue_offset))
                })
            })
    }

    pub fn with_content<C: Into<Content>>(mut self, content: C) -> Self {
        self.content = content.into();
        self
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

    /// WARNING: Due to bad programming, only use this after formatting the text.
    /// TODO: Fix inheriting not being a reference to its parent.
    pub fn with_child<F>(mut self, cb: F) -> Self
    where
        F: FnOnce(TextComponent) -> TextComponent,
    {
        let mut child = self.clone().with_content("");
        child.children = Vec::new();
        child.inherited_formatting = Some(self.formatting.clone());
        let child = cb(child);
        self.children.push(child);
        self
    }
}

impl TextComponent {
    fn to_json_inner(&self, root: bool) -> serde_json::Value {
        // The root TextComponent can either be: String, TextComponent, TextComponent[]
        if root {
            if let Content::Text { text } = &self.content {
                match (
                    text.is_empty(),
                    self.formatting == Formatting::default(),
                    self.children.is_empty(),
                ) {
                    (true, true, false) => {
                        return serde_json::Value::Array(
                            self.children
                                .iter()
                                .map(|child| child.to_json_inner(false))
                                .collect::<Vec<_>>(),
                        )
                    }
                    (_, true, true) => return serde_json::Value::String(text.to_owned()),
                    _ => {}
                }
            }
        }
        let mut map = serde_json::Map::new();
        self.content.insert_map(&mut map);
        self.formatting.insert_map(&mut map);
        if !self.children.is_empty() {
            map.insert(
                "children".to_owned(),
                self.children
                    .iter()
                    .map(|child| child.to_json_inner(false))
                    .collect::<Vec<_>>()
                    .into(),
            );
        }
        serde_json::Value::Object(map)
    }

    pub fn to_json(&self) -> serde_json::Value {
        self.to_json_inner(true)
    }

    pub fn to_nbt(&self) -> NBT {
        NBT::try_from(self.to_json()).unwrap()
    }
}

impl<T: Into<Content>> From<T> for TextComponent {
    fn from(value: T) -> Self {
        TextComponent::new(value.into())
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
