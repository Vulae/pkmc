#[derive(Debug)]
pub struct ServerState {
    pub server_brand: String,
    pub server_list_text: Option<String>,
    /// 64x64 base64 encoded PNG image
    pub server_list_icon: Option<String>,
    /// Default packet compression 0..=9 (0 is uncompressed)
    pub compression_level: u32,
    /// Packet compression threshold
    pub compression_threshold: usize,
    /// The main dimension name
    pub world_main_name: String,
    /// The main dimension min y (MUST me multiple of 16)
    pub world_min_y: i32,
    /// The main dimension max y (MUST me multiple of 16)
    pub world_max_y: i32,
}
