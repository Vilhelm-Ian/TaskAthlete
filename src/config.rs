// src/config.rs
use anyhow::{Context, Result};
use comfy_table::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{stdin, Write}; // Import Write for flush
use std::path::{Path, PathBuf};
use thiserror::Error;
// Import strum for color parsing
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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
    #[error("Bodyweight input cancelled by user.")]
    BodyweightPromptCancelled,
    #[error("Invalid bodyweight input: {0}")]
    InvalidBodyweightInput(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Metric,   // e.g., kg
    Imperial, // e.g., lbs
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
    // Add more theme options later if needed (e.g., row_color, border_color)
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            header_color: "Green".to_string(), // Default header color
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Config {
    pub bodyweight: Option<f64>,
    pub units: Units,
    pub prompt_for_bodyweight: bool, // Default is true
    pub theme: ThemeConfig,
}

// Implement Default for Config manually to set prompt_for_bodyweight correctly
impl Config {
    fn new_default() -> Self {
        Config {
            bodyweight: None,
            units: Units::default(),
            prompt_for_bodyweight: true, // Explicitly true by default
            theme: ThemeConfig::default(),
        }
    }
}

/// Determines the path to the configuration file.
/// Checks `WORKOUT_CONFIG_DIR` env var first, then falls back to default config dir.
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let config_dir_override = std::env::var(CONFIG_ENV_VAR).ok();

    let config_dir_path = match config_dir_override {
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path.is_dir() {
                eprintln!(
                    "Warning: Environment variable {} points to '{}', which is not a directory. Trying to create it.",
                    CONFIG_ENV_VAR,
                    path.display()
                 );
                // Attempt to create it, let subsequent operations handle failure if it persists
                 fs::create_dir_all(&path)?;
            }
            path
        }
        None => {
            // Use default config directory
            let base_config_dir =
                dirs::config_dir().ok_or(ConfigError::CannotDetermineConfigDir)?;
            base_config_dir.join(APP_CONFIG_DIR)
        }
    };

    // Ensure the chosen directory exists
    if !config_dir_path.exists() {
        fs::create_dir_all(&config_dir_path)?;
    }

    Ok(config_dir_path.join(CONFIG_FILE_NAME))
}

/// Loads the configuration from the TOML file.
/// If the file doesn't exist, returns default config and creates the default file.
pub fn load_config() -> Result<Config, ConfigError> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        println!(
            "Configuration file not found at {:?}. Creating a default one.",
            config_path
        );
        let default_config = Config::new_default();
        save_config(&config_path, &default_config)?; // Save the default config
        Ok(default_config)
    } else {
        let config_content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }
}

/// Saves the configuration to the TOML file.
pub fn save_config(config_path: &Path, config: &Config) -> Result<(), ConfigError> {
    // Ensure the directory exists before trying to write
    if let Some(parent_dir) = config_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    }

    let config_content =
        toml::to_string_pretty(config).map_err(ConfigError::TomlSerialize)?;
    fs::write(config_path, config_content)?;
    Ok(())
}


/// Prompts the user for bodyweight if needed and updates the config.
/// Returns Ok(bodyweight) if successful, or an appropriate ConfigError.
pub fn prompt_and_set_bodyweight(config: &mut Config) -> Result<f64, ConfigError> {
    let config_path = get_config_path()?; // Get path for messages

    // Check if we need to prompt
    if config.bodyweight.is_some() || !config.prompt_for_bodyweight {
        // Bodyweight already set OR user opted out of prompts
        return config.bodyweight.ok_or(ConfigError::BodyweightNotSet(config_path));
    }

    // Prompt is needed
    println!("Bodyweight is required for this exercise type but is not set.");
    println!(
        "Please enter your current bodyweight (in {:?}).",
        config.units
    );
    print!("Enter weight, or 'N' to not be asked again (you'll need to use 'set-bodyweight' later): ");
    std::io::stdout().flush()?; // Ensure the prompt is displayed before reading input

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!("Please use the 'set-bodyweight <weight>' command to set it manually.");
        config.prompt_for_bodyweight = false;
        save_config(&config_path, config)?; // Save the updated prompt setting
        Err(ConfigError::BodyweightPromptCancelled) // Indicate cancellation
    } else {
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!("Setting bodyweight to {} {:?}", weight, config.units);
                config.bodyweight = Some(weight);
                config.prompt_for_bodyweight = true; // Keep prompting enabled unless N is entered
                save_config(&config_path, config)?; // Save the new weight and prompt setting
                Ok(weight)
            }
            Ok(_) => Err(ConfigError::InvalidBodyweightInput(
                "Weight must be a positive number.".to_string(),
            )),
            Err(e) => Err(ConfigError::InvalidBodyweightInput(format!(
                "Could not parse '{}': {}",
                trimmed_input, e
            ))),
        }
    }
}
