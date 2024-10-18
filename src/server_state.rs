#[derive(Debug)]
pub struct ServerState {
    pub server_list_text: Option<String>,
    /// 64x64 base64 encoded PNG image
    pub server_list_icon: Option<String>,
    /// The main dimension name
    pub world_main_name: String,
    /// The main dimension min y (MUST me multiple of 16)
    pub world_min_y: i32,
    /// The main dimension max y (MUST me multiple of 16)
    pub world_max_y: i32,
}
