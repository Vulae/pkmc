use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ChatTypeDecorationParameter {
    #[serde(rename = "sender")]
    Sender,
    #[serde(rename = "target")]
    Target,
    #[serde(rename = "content")]
    Content,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatTypeDecoration {
    pub translation_key: String,
    // TODO: No clue on how to do this.
    pub style: Option<()>,
    pub parameters: Vec<ChatTypeDecorationParameter>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatType {
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}
