//src/config.rs
use anyhow::Result;
use comfy_table::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{stdin, Write};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use thiserror::Error;

const CONFIG_FILE_NAME: &str = "config.toml";
const APP_CONFIG_DIR: &str = "workout-tracker-cli";
const CONFIG_ENV_VAR: &str = "WORKOUT_CONFIG_DIR"; // Environment variable name

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not determine configuration directory.")]
    CannotDetermineConfigDir,
    #[error("I/O error accessing config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config file (TOML): {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Failed to serialize config data (TOML): {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Invalid color name: {0}")]
    InvalidColor(String),
    #[error("Bodyweight not set in config. Use 'set-bodyweight <weight>' or update {0:?}.")]
    BodyweightNotSet(PathBuf),
    #[error("Bodyweight input cancelled by user.")] // Keep for potential interactive use
    BodyweightPromptCancelled,
    #[error("Invalid bodyweight input: {0}")]
    InvalidBodyweightInput(String),
    #[error("Personal best notification setting not configured. Please enable/disable using 'set-pb-notification true|false'.")]
    // Feature 4
    PbNotificationNotSet,
    #[error("Personal best notification prompt cancelled by user.")] // Feature 4
    PbNotificationPromptCancelled,
    #[error("Invalid input for PB notification prompt: {0}")] // Feature 4
    InvalidPbNotificationInput(String),
}

// Note: PbMetricScope removed as specific booleans are used now.
//       Kept the enum definition commented out in case of future refactoring.
// #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
// #[serde(rename_all = "lowercase")]
// pub enum PbMetricScope {
//     All,     // Check weight and reps
//     Weight,  // Only check weight PBs
//     Reps,    // Only check reps PBs
//     // Note: Disabling notifications entirely is handled by notify_on_pb = false
// }
// impl Default for PbMetricScope { fn default() -> Self { PbMetricScope::All } }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Metric,   // e.g., kg, km
    Imperial, // e.g., lbs, miles
}

// Implement Default for Units
impl Default for Units {
    fn default() -> Self {
        Units::Metric // Default to Metric
    }
}

// Define standard colors using strum for easy iteration/parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum StandardColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    DarkGrey,
    DarkRed,
    DarkGreen,
    DarkYellow,
    DarkBlue,
    DarkMagenta,
    DarkCyan,
    Grey,
}

// Helper to convert our enum to comfy_table::Color
impl From<StandardColor> for Color {
    fn from(value: StandardColor) -> Self {
        match value {
            StandardColor::Black => Color::Black,
            StandardColor::Red => Color::Red,
            StandardColor::Green => Color::Green,
            StandardColor::Yellow => Color::Yellow,
            StandardColor::Blue => Color::Blue,
            StandardColor::Magenta => Color::Magenta,
            StandardColor::Cyan => Color::Cyan,
            StandardColor::White => Color::White,
            StandardColor::DarkGrey => Color::DarkGrey,
            StandardColor::DarkRed => Color::DarkRed,
            StandardColor::DarkGreen => Color::DarkGreen,
            StandardColor::DarkYellow => Color::DarkYellow,
            StandardColor::DarkBlue => Color::DarkBlue,
            StandardColor::DarkMagenta => Color::DarkMagenta,
            StandardColor::DarkCyan => Color::DarkCyan,
            StandardColor::Grey => Color::Grey,
        }
    }
}

// Helper to parse a string into our StandardColor enum
pub fn parse_color(color_str: &str) -> Result<StandardColor, ConfigError> {
    for color in StandardColor::iter() {
        if format!("{:?}", color).eq_ignore_ascii_case(color_str) {
            return Ok(color);
        }
    }
    Err(ConfigError::InvalidColor(color_str.to_string()))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct ThemeConfig {
    pub header_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            header_color: "Green".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Removed Default derive
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Config {
    pub bodyweight: Option<f64>,
    pub units: Units,
    pub prompt_for_bodyweight: bool, // Default is true
    pub streak_interval_days: u32,   // Default 1

    // PB Notification Settings
    pub notify_pb_enabled: Option<bool>, // None = prompt first time, Some(true/false) = user setting
    pub notify_pb_weight: bool,
    pub notify_pb_reps: bool,
    pub notify_pb_duration: bool,
    pub notify_pb_distance: bool,
    pub target_bodyweight: Option<f64>,

    // Theming
    pub theme: ThemeConfig,
}

// Implement Default for Config manually to set defaults correctly
impl Default for Config {
    fn default() -> Self {
        Config {
            bodyweight: None,
            units: Units::default(),
            prompt_for_bodyweight: true, // Explicitly true by default
            streak_interval_days: 1,     // Default to daily streaks
            notify_pb_enabled: None,     // Default to None, so user is prompted first time
            notify_pb_weight: true,      // Default to true
            notify_pb_reps: true,        // Default to true
            notify_pb_duration: true,    // Default to true
            notify_pb_distance: true,    // Default to true
            target_bodyweight: None,
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    // Helper to create a new instance with defaults
    fn new_default() -> Self {
        Self::default()
    }
}

/// Determines the path to the configuration file.
/// Exposed at crate root as get_config_path_util
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let config_dir_override = std::env::var(CONFIG_ENV_VAR).ok();

    let config_dir_path = match config_dir_override {
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path.is_dir() {
                eprintln!( // Keep warning, as it's about env var setup
                    "Warning: Environment variable {} points to '{}', which is not a directory. Trying to create it.",
                    CONFIG_ENV_VAR,
                    path.display()
                 );
                fs::create_dir_all(&path)?;
            }
            path
        }
        None => {
            let base_config_dir =
                dirs::config_dir().ok_or(ConfigError::CannotDetermineConfigDir)?;
            base_config_dir.join(APP_CONFIG_DIR)
        }
    };

    if !config_dir_path.exists() {
        fs::create_dir_all(&config_dir_path)?;
    }

    Ok(config_dir_path.join(CONFIG_FILE_NAME))
}

/// Loads the configuration from the TOML file at the given path.
/// Exposed at crate root as load_config_util
pub fn load_config(config_path: &Path) -> Result<Config, ConfigError> {
    if !config_path.exists() {
        // Don't print here, let caller decide how to inform user
        let default_config = Config::new_default();
        save_config(&config_path, &default_config)?;
        Ok(default_config)
    } else {
        let config_content = fs::read_to_string(&config_path)?;
        // Use serde(default) to handle missing fields when parsing
        let config: Config = toml::from_str(&config_content).map_err(ConfigError::TomlParse)?;
        // No need to manually fill defaults here if using #[serde(default)] on struct and fields
        Ok(config)
    }
}

/// Saves the configuration to the TOML file.
/// Exposed at crate root as save_config_util
pub fn save_config(config_path: &Path, config: &Config) -> Result<(), ConfigError> {
    if let Some(parent_dir) = config_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    }
    let config_content = toml::to_string_pretty(config).map_err(ConfigError::TomlSerialize)?;
    fs::write(config_path, config_content)?;
    Ok(())
}
