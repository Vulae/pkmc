use pkmc_world::world::World;

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
    pub world: World,
}
