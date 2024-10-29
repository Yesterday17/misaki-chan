use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = std::fs::read_to_string("config.toml").expect("Failed to load config file");
    let config: Config = toml::from_str(&config).expect("Failed to parse config file");
    config
});

#[derive(Deserialize)]
pub struct Config {
    /// Telegram Bot Token
    pub token: String,
    /// User auth secret
    pub secret: String,
    /// Record root path
    #[serde(rename = "record")]
    pub record_root: Option<String>,
    /// Paths
    pub path: PathConfig,
}

#[derive(Deserialize)]
pub struct PathConfig {
    streamlink: Option<String>,
    ffmpeg: Option<String>,
}

impl PathConfig {
    pub fn streamlink(&self) -> &str {
        self.streamlink.as_deref().unwrap_or("streamlink")
    }

    pub fn ffmpeg(&self) -> &str {
        self.ffmpeg.as_deref().unwrap_or("ffmpeg")
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoomInfo(i64);

impl RoomInfo {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn index(&self) -> i64 {
        self.0
    }
}
