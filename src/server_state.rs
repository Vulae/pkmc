#[derive(Debug)]
pub struct ServerState {
    pub server_list_text: Option<String>,
    /// 64x64 base64 encoded PNG image
    pub server_list_icon: Option<String>,
}
