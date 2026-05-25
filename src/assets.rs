use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "asset/"]
pub struct Asset;
