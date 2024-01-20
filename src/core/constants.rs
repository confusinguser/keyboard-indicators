use openrgb::data::Color;
use rgb::RGBA8;

pub const CURRENT_WORKSPACE_OVERLAY: rgb::RGBA8 = RGBA8::new(0, 0, 0, 200);
pub const UNFOCUSED_WORKSPACE_OVERLAY: rgb::RGBA8 = RGBA8::new(0, 0, 0, 240);
pub const UNFOCUSED_DEFAULT_WORKSPACE_COLOR: Color = Color::new(128, 128, 128);
pub const EMPTY_WORKSPACE_COLOR: Color = Color::new(0, 0, 0);
pub const URGENT_WORKSPACE_COLOR: Color = Color::new(255, 0, 0);
pub const SPOTIFY_WORKSPACE_COLOR: Color = Color::new(30, 215, 96);
pub const DISCORD_WORKSPACE_COLOR: Color = Color::new(88, 101, 242);
pub const FIREFOX_WORKSPACE_COLOR: Color = Color::new(230, 0x60, 0);
