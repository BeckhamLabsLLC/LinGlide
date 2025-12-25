//! Configuration types for LinGlide

use serde::{Deserialize, Serialize};

/// Position of the virtual display relative to the primary display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayPosition {
    #[default]
    RightOf,
    LeftOf,
    Above,
    Below,
}

impl DisplayPosition {
    pub fn as_xrandr_arg(&self) -> &'static str {
        match self {
            DisplayPosition::RightOf => "--right-of",
            DisplayPosition::LeftOf => "--left-of",
            DisplayPosition::Above => "--above",
            DisplayPosition::Below => "--below",
        }
    }
}

impl std::str::FromStr for DisplayPosition {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "right-of" | "right" => Ok(DisplayPosition::RightOf),
            "left-of" | "left" => Ok(DisplayPosition::LeftOf),
            "above" | "top" => Ok(DisplayPosition::Above),
            "below" | "bottom" => Ok(DisplayPosition::Below),
            _ => Err(format!("Invalid position: {}. Use: right-of, left-of, above, below", s)),
        }
    }
}

/// Main configuration for LinGlide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Virtual display width in pixels
    pub width: u32,
    /// Virtual display height in pixels
    pub height: u32,
    /// Target frame rate
    pub fps: u32,
    /// Server port
    pub port: u16,
    /// Position relative to primary display
    pub position: DisplayPosition,
    /// Video bitrate in kbps
    pub bitrate: u32,
    /// Primary display name (auto-detected if None)
    pub primary_display: Option<String>,
    /// Virtual display output name (auto-detected if None)
    pub virtual_output: Option<String>,
    /// Mirror mode: capture primary display instead of creating virtual display
    pub mirror_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            port: 8443,
            position: DisplayPosition::RightOf,
            bitrate: 8000,
            primary_display: None,
            virtual_output: None,
            mirror_mode: false,
        }
    }
}

impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern: set width
    pub fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Builder pattern: set height
    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Builder pattern: set frame rate
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    /// Builder pattern: set port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Builder pattern: set position
    pub fn with_position(mut self, position: DisplayPosition) -> Self {
        self.position = position;
        self
    }

    /// Builder pattern: set bitrate
    pub fn with_bitrate(mut self, bitrate: u32) -> Self {
        self.bitrate = bitrate;
        self
    }

    /// Builder pattern: set mirror mode
    pub fn with_mirror_mode(mut self, mirror: bool) -> Self {
        self.mirror_mode = mirror;
        self
    }

    /// Calculate bytes per frame for BGRA format
    pub fn frame_size_bytes(&self) -> usize {
        (self.width * self.height * 4) as usize
    }

    /// Calculate the mode name for xrandr
    pub fn mode_name(&self) -> String {
        format!("{}x{}_linglide", self.width, self.height)
    }
}
