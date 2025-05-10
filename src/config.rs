// src/config.rs
use anyhow::Result;
use comfy_table::Color;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
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
    PbNotificationNotSet,
    #[error("Personal best notification prompt cancelled by user.")]
    PbNotificationPromptCancelled,
    #[error("Invalid input for PB notification prompt: {0}")]
    InvalidPbNotificationInput(String),
    #[error("Invalid streak interval: {0}. Must be at least 1.")]
    InvalidStreakInterval(u32),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    #[default]
    Metric, // e.g., kg, km
    Imperial, // e.g., lbs, miles
}

impl Units {
    /// Returns the standard abbreviation for weight units.
    pub const fn weight_abbr(&self) -> &'static str {
        match self {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        }
    }

    /// Returns the standard abbreviation for distance units.
    pub const fn distance_abbr(&self) -> &'static str {
        match self {
            Units::Metric => "km",
            Units::Imperial => "miles",
        }
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

impl fmt::Display for StandardColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the debug format which matches the expected parsing format
        write!(f, "{self:?}")
    }
}

// Helper to convert our enum to comfy_table::Color
impl From<StandardColor> for Color {
    fn from(value: StandardColor) -> Self {
        match value {
            StandardColor::Black => Self::Black,
            StandardColor::Red => Self::Red,
            StandardColor::Green => Self::Green,
            StandardColor::Yellow => Self::Yellow,
            StandardColor::Blue => Self::Blue,
            StandardColor::Magenta => Self::Magenta,
            StandardColor::Cyan => Self::Cyan,
            StandardColor::White => Self::White,
            StandardColor::DarkGrey => Self::DarkGrey,
            StandardColor::DarkRed => Self::DarkRed,
            StandardColor::DarkGreen => Self::DarkGreen,
            StandardColor::DarkYellow => Self::DarkYellow,
            StandardColor::DarkBlue => Self::DarkBlue,
            StandardColor::DarkMagenta => Self::DarkMagenta,
            StandardColor::DarkCyan => Self::DarkCyan,
            StandardColor::Grey => Self::Grey,
        }
    }
}

/// Parses a string into a `StandardColor`.
///
/// The parsing is case-insensitive.
///
/// # Errors
///
/// Returns `ConfigError::InvalidColor` if the input string does not match any `StandardColor` variant name.
pub fn parse_color(color_str: &str) -> Result<StandardColor, ConfigError> {
    for color in StandardColor::iter() {
        if format!("{color:?}").eq_ignore_ascii_case(color_str) {
            return Ok(color);
        }
    }
    Err(ConfigError::InvalidColor(color_str.to_string()))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Theme {
    pub header_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            header_color: "Green".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct PbNotificationConfig {
    pub enabled: Option<bool>, // None = prompt first time, Some(true/false) = user setting
    pub notify_weight: bool,
    pub notify_reps: bool,
    pub notify_duration: bool,
    pub notify_distance: bool,
}

impl Default for PbNotificationConfig {
    fn default() -> Self {
        Self {
            enabled: None, // Default to None, so user is prompted first time
            notify_weight: true,
            notify_reps: true,
            notify_duration: true,
            notify_distance: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Config {
    pub bodyweight: Option<f64>,
    pub units: Units,
    pub prompt_for_bodyweight: bool, // Default is true
    pub streak_interval_days: u32,   // Default 1
    pub target_bodyweight: Option<f64>,
    pub theme: Theme,
    pub pb_notifications: PbNotificationConfig, // Grouped PB settings
}

// Implement Default for Config manually to set defaults correctly
impl Default for Config {
    fn default() -> Self {
        Self {
            bodyweight: None,
            units: Units::default(),
            prompt_for_bodyweight: true, // Explicitly true by default
            streak_interval_days: 1,     // Default to daily streaks
            target_bodyweight: None,
            theme: Theme::default(),
            pb_notifications: PbNotificationConfig::default(), // Use nested default
        }
    }
}

/// Determines the path to the configuration file.
///
/// It prioritizes the path specified by the `WORKOUT_CONFIG_DIR` environment variable.
/// If the variable is not set or invalid, it falls back to the standard configuration directory
/// (`~/.config/workout-tracker-cli/config.toml` on Linux).
/// If the directory doesn't exist, it attempts to create it.
///
/// Exposed at crate root as `get_config_path_util`.
///
/// # Errors
///
/// - `ConfigError::CannotDetermineConfigDir`: If the base configuration directory cannot be found (e.g., on unsupported platforms).
/// - `ConfigError::Io`: If there's an I/O error creating the configuration directory.
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let config_dir_override = std::env::var(CONFIG_ENV_VAR).ok();

    let config_dir_path = if let Some(path_str) = config_dir_override {
        let path = PathBuf::from(path_str);
        if !path.is_dir() {
            eprintln!(
                    "Warning: Environment variable {CONFIG_ENV_VAR} points to '{}', which is not a directory. Trying to create it.",
                    path.display()
                 );
            fs::create_dir_all(&path)?;
        }
        path
    } else {
        let base_config_dir = dirs::config_dir().ok_or(ConfigError::CannotDetermineConfigDir)?;
        base_config_dir.join(APP_CONFIG_DIR)
    };

    if !config_dir_path.exists() {
        fs::create_dir_all(&config_dir_path)?;
    }

    Ok(config_dir_path.join(CONFIG_FILE_NAME))
}

/// Loads the configuration from the TOML file at the given path.
///
/// If the file doesn't exist, it creates a default configuration file and returns the default config.
/// It uses `serde(default)` to handle missing fields gracefully when parsing an existing file.
///
/// Exposed at crate root as `load_config_util`.
///
/// # Errors
///
/// - `ConfigError::Io`: If there's an error reading the config file or writing the default config.
/// - `ConfigError::TomlParse`: If the existing config file content is invalid TOML.
/// - `ConfigError::TomlSerialize`: If the default config data cannot be serialized to TOML (should not happen).
pub fn load(config_path: &Path) -> Result<Config, ConfigError> {
    if config_path.exists() {
        let config_content = fs::read_to_string(config_path)?;
        // Use serde(default) to handle missing fields when parsing
        let config: Config = toml::from_str(&config_content).map_err(ConfigError::TomlParse)?;
        Ok(config)
    } else {
        // Don't print here, let caller decide how to inform user
        let default_config = Config::default();
        save(config_path, &default_config)?;
        Ok(default_config)
    }
}

/// Saves the configuration to the TOML file.
///
/// Creates the parent directory if it doesn't exist.
///
/// Exposed at crate root as `save_config_util`.
///
/// # Errors
///
/// - `ConfigError::Io`: If there's an error creating the parent directory or writing the file.
/// - `ConfigError::TomlSerialize`: If the config data cannot be serialized to TOML.
pub fn save(config_path: &Path, config: &Config) -> Result<(), ConfigError> {
    if let Some(parent_dir) = config_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    }
    let config_content = toml::to_string_pretty(config).map_err(ConfigError::TomlSerialize)?;
    fs::write(config_path, config_content)?;
    Ok(())
}
