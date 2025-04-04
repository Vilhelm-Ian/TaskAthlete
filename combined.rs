//src/cli.rs
// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};
use chrono::{NaiveDate, Utc, Duration, DateTime}; // Import DateTime

#[derive(Parser, Debug)]
#[command(author, version, about = "A CLI tool to track workouts", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum ExerciseTypeCli {
    Resistance,
    Cardio,
    BodyWeight,
}

// Custom parser for date strings and shorthands
fn parse_date_shorthand(s: &str) -> Result<NaiveDate, String> {
    match s.to_lowercase().as_str() {
        "today" => Ok(Utc::now().date_naive()),
        "yesterday" => Ok((Utc::now() - Duration::days(1)).date_naive()),
        _ => {
            // Try parsing YYYY-MM-DD first
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Ok(date)
            }
            // Try parsing DD.MM.YYYY next
            else if let Ok(date) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
                Ok(date)
            }
            // Try parsing YYYY/MM/DD
            else if let Ok(date) = NaiveDate::parse_from_str(s, "%Y/%m/%d") {
                 Ok(date)
            }
            else {
                 Err(format!(
                    "Invalid date format: '{}'. Use 'today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, or YYYY/MM/DD.", // Updated help message
                    s
                ))
            }
        }
    }
}


#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Define a new exercise type
    CreateExercise {
        /// Name of the exercise (e.g., "Bench Press", "Running") - Must be unique (case-insensitive)
        #[arg(short, long)]
        name: String,
        /// Type of exercise
        #[arg(short = 't', long, value_enum)] // Changed short arg
        type_: ExerciseTypeCli,
        /// Comma-separated list of target muscles (e.g., "chest,triceps,shoulders")
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Delete an exercise definition
    DeleteExercise {
        /// ID, Name, or Alias of the exercise to delete
        identifier: String,
    },
    /// Edit an exercise definition
    EditExercise {
        /// ID, Name, or Alias of the exercise to edit
        identifier: String,
        /// New name for the exercise (must be unique)
        #[arg(short, long)]
        name: Option<String>,
        /// New type for the exercise
        #[arg(short = 't', long, value_enum)] // Changed short arg
        type_: Option<ExerciseTypeCli>,
        /// New comma-separated list of target muscles
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Add a new workout entry
    Add {
        /// Name, ID, or Alias of the exercise (will prompt to create if not found and type/muscles given)
        #[arg(short = 'e', long)] // Added short alias
        exercise: String,

        /// Number of sets performed
        #[arg(short, long)]
        sets: Option<i64>,

        /// Number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,

        /// Weight used (e.g., kg, lbs). For Bodyweight exercises, this is *additional* weight.
        #[arg(short, long)]
        weight: Option<f64>,

        /// Duration in minutes (for cardio or timed exercises)
        #[arg(short = 'd', long)] // Added short alias
        duration: Option<i64>,

        /// Additional notes about the workout
        #[arg(short, long)]
        notes: Option<String>,

        /// Date of the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand, default_value = "today")] // Feature 3
        date: NaiveDate,

        // Optional fields for implicit exercise creation during 'add' if exercise not found
        #[arg(long = "type", value_enum, requires = "implicit-muscles", id = "implicit-exercise-type")]
        implicit_type: Option<ExerciseTypeCli>, // Renamed to avoid clash with filter

        #[arg(long, requires = "implicit-exercise-type", id = "implicit-muscles")]
        implicit_muscles: Option<String>, // Renamed to avoid clash with filter
    },
     /// Edit an existing workout entry
    EditWorkout {
        /// ID of the workout entry to edit
        id: i64, // Use ID for editing specific entries
        /// New exercise Name, ID or Alias for the workout
        #[arg(short = 'e', long)] // Added short alias
        exercise: Option<String>,
        /// New number of sets performed
        #[arg(short, long)]
        sets: Option<i64>,
        /// New number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,
        /// New weight used (absolute value, bodyweight logic NOT reapplied on edit)
        #[arg(short, long)]
        weight: Option<f64>,
        /// New duration in minutes
        #[arg(short = 'd', long)] // Added short alias
        duration: Option<i64>,
        /// New additional notes
        #[arg(short, long)]
        notes: Option<String>,
         /// New date for the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand)] // Feature 3 (for editing date)
        date: Option<NaiveDate>,
    },
    /// Delete a workout entry
    DeleteWorkout {
        /// ID of the workout to delete
        id: i64,
    },
    /// List workout entries with filters
    List {
         /// Filter by exercise Name, ID or Alias
        #[arg(short = 'e', long, conflicts_with = "nth_last_day_exercise")]
        exercise: Option<String>,

        /// Filter by a specific date ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY)
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["today_flag", "yesterday_flag", "nth_last_day_exercise"])]
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>, // Short 'm'

        /// Show only the last N entries (when no date/day filters used)
        #[arg(short = 'n', long, default_value_t = 20, conflicts_with_all = &["today_flag", "yesterday_flag", "date", "nth_last_day_exercise"])]
        limit: u32,

        // Keep flags for backward compatibility or preference, but date is more versatile
        #[arg(long, conflicts_with_all = &["yesterday_flag", "date", "nth_last_day_exercise", "limit"])]
        today_flag: bool,
        #[arg(long, conflicts_with_all = &["today_flag", "date", "nth_last_day_exercise", "limit"])]
        yesterday_flag: bool,


        /// Show workouts for the Nth most recent day a specific exercise (Name, ID, Alias) was performed
        #[arg(long, value_name = "EXERCISE_IDENTIFIER", requires = "nth_last_day_n", conflicts_with_all = &["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_exercise: Option<String>,
        #[arg(long, value_name = "N", requires = "nth_last_day_exercise", conflicts_with_all = &["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_n: Option<u32>,

    },
     /// List defined exercise types
    ListExercises {
        /// Filter by exercise type
        #[arg(short='t', long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Filter by a target muscle (matches if the muscle is in the list)
        #[arg(short='m', long)] // short 'm'
        muscle: Option<String>,
    },
    /// Create an alias for an existing exercise
    Alias { // Feature 1
        /// The alias name (e.g., "bp") - Must be unique
        alias_name: String,
        /// The ID, Name, or existing Alias of the exercise to alias
        exercise_identifier: String,
    },
    /// Delete an exercise alias
    Unalias { // Feature 1
        /// The alias name to delete
        alias_name: String,
    },
    /// List all defined exercise aliases
    ListAliases, // Feature 1
    /// Show the path to the database file
    DbPath,
    /// Show the path to the config file
    ConfigPath,
    /// Set your bodyweight in the config file
    SetBodyweight{
        /// Your current bodyweight
        weight: f64
    },
    /// Enable or disable Personal Best (PB) notifications
    SetPbNotification { // Feature 4
        /// Enable PB notifications (`true` or `false`)
        enabled: bool,
    },
    // Maybe add a command to set units later: SetUnits { units: UnitsCli }
}

// Function to parse CLI arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

//src/config.rs
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
    #[error("Bodyweight input cancelled by user.")] // Keep for potential interactive use
    BodyweightPromptCancelled,
    #[error("Invalid bodyweight input: {0}")]
    InvalidBodyweightInput(String),
    #[error("Personal best notification setting not configured. Please enable/disable using 'set-pb-notification true|false'.")] // Feature 4
    PbNotificationNotSet,
    #[error("Personal best notification prompt cancelled by user.")] // Feature 4
    PbNotificationPromptCancelled,
    #[error("Invalid input for PB notification prompt: {0}")] // Feature 4
    InvalidPbNotificationInput(String),
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
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, White,
    DarkGrey, DarkRed, DarkGreen, DarkYellow, DarkBlue, DarkMagenta, DarkCyan, Grey,
}

// Helper to convert our enum to comfy_table::Color
impl From<StandardColor> for Color {
    fn from(value: StandardColor) -> Self {
        match value {
            StandardColor::Black => Color::Black, StandardColor::Red => Color::Red,
            StandardColor::Green => Color::Green, StandardColor::Yellow => Color::Yellow,
            StandardColor::Blue => Color::Blue, StandardColor::Magenta => Color::Magenta,
            StandardColor::Cyan => Color::Cyan, StandardColor::White => Color::White,
            StandardColor::DarkGrey => Color::DarkGrey, StandardColor::DarkRed => Color::DarkRed,
            StandardColor::DarkGreen => Color::DarkGreen, StandardColor::DarkYellow => Color::DarkYellow,
            StandardColor::DarkBlue => Color::DarkBlue, StandardColor::DarkMagenta => Color::DarkMagenta,
            StandardColor::DarkCyan => Color::DarkCyan, StandardColor::Grey => Color::Grey,
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
        ThemeConfig { header_color: "Green".to_string() }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Config {
    pub bodyweight: Option<f64>,
    pub units: Units,
    pub prompt_for_bodyweight: bool, // Default is true
    pub notify_on_pb: Option<bool>, // Feature 4: None = ask, Some(true/false) = configured
    pub theme: ThemeConfig,
}

// Implement Default for Config manually to set prompt_for_bodyweight correctly
impl Config {
    fn new_default() -> Self {
        Config {
            bodyweight: None,
            units: Units::default(),
            prompt_for_bodyweight: true, // Explicitly true by default
            notify_on_pb: None, // Default to None, so user is prompted first time
            theme: ThemeConfig::default(),
        }
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
            let base_config_dir = dirs::config_dir().ok_or(ConfigError::CannotDetermineConfigDir)?;
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
         let mut config: Config = toml::from_str(&config_content)?;
         // Ensure default for new fields if loading old config
         if config.notify_on_pb.is_none() {
             // If the field didn't exist in the file, keep it None (will prompt user)
             // Don't need to save here, only on explicit change or prompt result.
         }
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

 /// Prompts the user for bodyweight if needed and updates the config.
 /// This function should ideally live in the UI layer (main.rs or tui).
 /// It's kept here temporarily but marked for potential removal from lib.
 /// *** Consider moving this interactive logic to the caller (e.g., main.rs) ***
 /// Note: This function is NOT directly used by AppService, it's intended for potential CLI use.
pub fn prompt_and_set_bodyweight_interactive(config: &mut Config, config_path: &PathBuf) -> Result<f64, ConfigError> {
    // Check if we need to prompt (Only if bodyweight is None AND prompting is enabled)
    if config.bodyweight.is_some() || !config.prompt_for_bodyweight {
        return config.bodyweight.ok_or_else(|| ConfigError::BodyweightNotSet(config_path.clone()));
    }

    // Prompt is needed
    println!("Bodyweight is required for this exercise type but is not set.");
    println!("Please enter your current bodyweight (in {:?}).", config.units);
    print!("Enter weight, or 'N' to not be asked again (use 'set-bodyweight' later): ");
    std::io::stdout().flush()?; // Ensure the prompt is displayed before reading input

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!("Please use the 'set-bodyweight <weight>' command to set it manually.");
        config.prompt_for_bodyweight = false;
        save_config(config_path, config)?; // Save the updated prompt setting
        Err(ConfigError::BodyweightPromptCancelled) // Indicate cancellation
    } else {
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!("Setting bodyweight to {} {:?}", weight, config.units);
                config.bodyweight = Some(weight);
                config.prompt_for_bodyweight = true; // Keep prompting enabled unless N is entered
                save_config(config_path, config)?; // Save the new weight and prompt setting
                Ok(weight)
            }
            Ok(_) => Err(ConfigError::InvalidBodyweightInput("Weight must be a positive number.".to_string())),
            Err(e) => Err(ConfigError::InvalidBodyweightInput(format!("Could not parse '{}': {}", trimmed_input, e))),
        }
    }
}

//src/db.rs
use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, ToSql, named_params}; // Import named_params
use std::collections::HashMap; // For listing aliases
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

// Use crate::config types if needed, or define locally if fully independent
// Assuming ExerciseType is defined here for now.
#[derive(Debug, PartialEq, Eq, Clone, Copy)] // Add Copy
pub enum ExerciseType {
    Resistance,
    Cardio,
    BodyWeight,
}

// Convert string from DB to ExerciseType
impl TryFrom<&str> for ExerciseType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "resistance" => Ok(ExerciseType::Resistance),
            "cardio" => Ok(ExerciseType::Cardio),
            "body-weight" | "bodyweight" => Ok(ExerciseType::BodyWeight), // Allow variation
            _ => anyhow::bail!("Invalid exercise type string from DB: {}", value),
        }
    }
}

// Convert ExerciseType to string for DB storage
impl fmt::Display for ExerciseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExerciseType::Resistance => write!(f, "resistance"),
            ExerciseType::Cardio => write!(f, "cardio"),
            ExerciseType::BodyWeight => write!(f, "body-weight"), // Consistent storage
        }
    }
}

#[derive(Debug)]
pub struct Workout {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String, // Always the canonical name
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub notes: Option<String>,
    pub exercise_type: Option<ExerciseType>, // Populated by JOIN
}

#[derive(Debug, Clone)] // Add Clone
pub struct ExerciseDefinition {
    pub id: i64,
    pub name: String,
    pub type_: ExerciseType,
    pub muscles: Option<String>,
}


// Custom Error type for DB operations
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection failed")]
    Connection(#[from] rusqlite::Error),
    #[error("Failed to get application data directory")]
    DataDir,
    #[error("I/O error accessing database file")]
    Io(#[from] std::io::Error),
    #[error("Exercise not found: {0}")]
    ExerciseNotFound(String),
    #[error("Workout entry not found: ID {0}")]
    WorkoutNotFound(i64),
    #[error("Database query failed: {0}")]
    QueryFailed(rusqlite::Error), // More specific query error
    #[error("Database update failed: {0}")]
    UpdateFailed(rusqlite::Error),
    #[error("Database insert failed: {0}")]
    InsertFailed(rusqlite::Error),
    #[error("Database delete failed: {0}")]
    DeleteFailed(rusqlite::Error),
    #[error("Alias not found: {0}")] // Feature 1
    AliasNotFound(String),
    #[error("Alias already exists: {0}")] // Feature 1
    AliasAlreadyExists(String),
    #[error("Exercise name must be unique (case-insensitive): '{0}' already exists.")] // Feature 2
    ExerciseNameNotUnique(String),
}

const DB_FILE_NAME: &str = "workouts.sqlite";

/// Gets the path to the SQLite database file within the app's data directory.
/// Exposed at crate root as get_db_path_util
pub fn get_db_path() -> Result<PathBuf, DbError> {
    let data_dir = dirs::data_dir().ok_or(DbError::DataDir)?;
    let app_dir = data_dir.join("workout-tracker-cli"); // Same dir name as config
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }
    Ok(app_dir.join(DB_FILE_NAME))
}

/// Opens a connection to the SQLite database.
pub fn open_db<P: AsRef<Path>>(path: P) -> Result<Connection, DbError> {
    let conn = Connection::open(path).map_err(DbError::Connection)?;
    // Enable foreign key support if needed later, though not strictly required for aliases
    // conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

/// Initializes the database tables if they don't exist.
pub fn init_db(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE, -- Feature 2: Ensure UNIQUE and case-insensitive
            type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio', 'body-weight')),
            muscles TEXT
        )",
        [],
    ).map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS workouts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL, -- Store as RFC3339 string
            exercise_name TEXT NOT NULL COLLATE NOCASE, -- Store canonical name, case-insensitive for joins
            sets INTEGER, reps INTEGER, weight REAL, duration_minutes INTEGER, notes TEXT
            -- Optionally add FOREIGN KEY(exercise_name) REFERENCES exercises(name) ON UPDATE CASCADE ON DELETE SET NULL ?
            -- Requires careful handling of deletion/renaming if implemented. Keeping it simple for now.
        )",
        [],
    ).map_err(DbError::Connection)?;

     conn.execute(
        "CREATE TABLE IF NOT EXISTS aliases ( -- Feature 1
            alias_name TEXT PRIMARY KEY NOT NULL COLLATE NOCASE, -- Alias is unique, case-insensitive
            exercise_name TEXT NOT NULL COLLATE NOCASE -- Canonical exercise name it refers to
            -- Optionally add FOREIGN KEY(exercise_name) REFERENCES exercises(name) ON UPDATE CASCADE ON DELETE CASCADE ?
            -- This would auto-update/delete aliases if the exercise name changes or is deleted.
            -- Requires robust transaction handling in exercise edit/delete. Let's manage manually for now.
        )",
        [],
    ).map_err(DbError::Connection)?;

    // Add indexes for common lookups
    conn.execute("CREATE INDEX IF NOT EXISTS idx_workouts_timestamp ON workouts(timestamp)", []).map_err(DbError::Connection)?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_workouts_exercise_name ON workouts(exercise_name)", []).map_err(DbError::Connection)?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_aliases_exercise_name ON aliases(exercise_name)", []).map_err(DbError::Connection)?;


    Ok(())
}

/// Adds a new workout entry to the database.
pub fn add_workout(
    conn: &Connection,
    exercise_name: &str, // Should be the canonical name
    timestamp: DateTime<Utc>, // Feature 3: Accept specific timestamp
    sets: Option<i64>, reps: Option<i64>, weight: Option<f64>,
    duration: Option<i64>, notes: Option<String>,
) -> Result<i64, DbError> {
    let timestamp_str = timestamp.to_rfc3339();
    // Use default value 1 for sets only if it's None and the exercise type needs it (e.g., resistance, bodyweight)
    // For simplicity, let's keep the original behavior where sets default to 1 if None.
    // A more robust approach might check exercise type.
    let sets_val = sets.unwrap_or(1);

    conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, notes)
         VALUES (:ts, :ex_name, :sets, :reps, :weight, :duration, :notes)",
        named_params! {
            ":ts": timestamp_str,
            ":ex_name": exercise_name,
            ":sets": sets_val,
            ":reps": reps,
            ":weight": weight,
            ":duration": duration,
            ":notes": notes,
        },
    ).map_err(DbError::InsertFailed)?;
    Ok(conn.last_insert_rowid())
}

/// Updates an existing workout entry in the database by its ID.
pub fn update_workout(
    conn: &Connection,
    id: i64, new_exercise_name: Option<&str>, new_sets: Option<i64>, new_reps: Option<i64>,
    new_weight: Option<f64>, new_duration: Option<i64>, new_notes: Option<&str>,
    new_timestamp: Option<DateTime<Utc>>, // Feature 3: Allow editing timestamp
) -> Result<u64, DbError> {
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(ex) = new_exercise_name { updates.push("exercise_name = :ex_name"); params_map.insert(":ex_name".into(), Box::new(ex.to_string())); }
    if let Some(s) = new_sets { updates.push("sets = :sets"); params_map.insert(":sets".into(), Box::new(s)); }
    if let Some(r) = new_reps { updates.push("reps = :reps"); params_map.insert(":reps".into(), Box::new(r)); }
    // Use is_some() to allow setting weight/duration to NULL explicitly if needed, though CLI usually wouldn't do this.
    // If the Option is None, we don't add it to the update. If it's Some(None) for the value, that needs different handling (not typical here).
    if new_weight.is_some() { updates.push("weight = :weight"); params_map.insert(":weight".into(), Box::new(new_weight)); }
    if new_duration.is_some() { updates.push("duration_minutes = :duration"); params_map.insert(":duration".into(), Box::new(new_duration)); }
    if new_notes.is_some() { updates.push("notes = :notes"); params_map.insert(":notes".into(), Box::new(new_notes)); }
    if let Some(ts) = new_timestamp { updates.push("timestamp = :ts"); params_map.insert(":ts".into(), Box::new(ts.to_rfc3339())); }


    let sql = format!("UPDATE workouts SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));

    // Convert HashMap<String, Box<dyn ToSql>> to Vec<(&str, &dyn ToSql)> for execute_named
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map.iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = conn.execute(&sql, params_for_exec.as_slice())
                        .map_err(DbError::UpdateFailed)?;

    if rows_affected == 0 { Err(DbError::WorkoutNotFound(id)) } else { Ok(rows_affected as u64) }
}

/// Deletes a workout entry from the database by its ID.
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64, DbError> { // Return DbError
    let rows_affected = conn.execute("DELETE FROM workouts WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 { Err(DbError::WorkoutNotFound(id)) } else { Ok(rows_affected as u64) }
}

// Helper function to map a database row to a Workout struct
fn map_row_to_workout(row: &Row) -> Result<Workout, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let timestamp_str: String = row.get(1)?;
    let exercise_name: String = row.get(2)?; // Canonical name from DB
    let sets: Option<i64> = row.get(3)?;
    let reps: Option<i64> = row.get(4)?;
    let weight: Option<f64> = row.get(5)?;
    let duration_minutes: Option<i64> = row.get(6)?;
    let notes: Option<String> = row.get(7)?;
    let type_str_opt: Option<String> = row.get(8)?; // From JOIN with exercises

    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;

    let exercise_type = match type_str_opt {
        Some(type_str) => match ExerciseType::try_from(type_str.as_str()) {
            Ok(et) => Some(et),
            Err(_) => None, // Silently ignore invalid type from DB in lib layer
        },
        None => None,
    };

    Ok(Workout { id, timestamp, exercise_name, sets, reps, weight, duration_minutes, notes, exercise_type })
}

#[derive(Default, Debug)]
pub struct WorkoutFilters<'a> {
    pub exercise_name: Option<&'a str>, // Canonical name expected
    pub date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit: Option<u32>,
}

/// Lists workout entries from the database based on various filters.
pub fn list_workouts_filtered(conn: &Connection, filters: WorkoutFilters) -> Result<Vec<Workout>, DbError> { // Return DbError
    let mut sql = "SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
                   FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(name) = filters.exercise_name { sql.push_str(" AND w.exercise_name = :ex_name"); params_map.insert(":ex_name".into(), Box::new(name.to_string())); }
    if let Some(date) = filters.date { sql.push_str(" AND date(w.timestamp) = date(:date)"); params_map.insert(":date".into(), Box::new(date.format("%Y-%m-%d").to_string())); }
    if let Some(ex_type) = filters.exercise_type { sql.push_str(" AND e.type = :ex_type"); params_map.insert(":ex_type".into(), Box::new(ex_type.to_string())); }
    if let Some(m) = filters.muscle { sql.push_str(" AND e.muscles LIKE :muscle"); params_map.insert(":muscle".into(), Box::new(format!("%{}%", m))); }

    // Order by timestamp: ASC if date filter is used (show earliest first for that day), DESC otherwise (show latest overall)
    if filters.date.is_some() { sql.push_str(" ORDER BY w.timestamp ASC"); }
    else { sql.push_str(" ORDER BY w.timestamp DESC"); }

    // Apply limit only if date is not specified (limit applies to overall latest, not within a date)
    if filters.date.is_none() {
        if let Some(limit) = filters.limit { sql.push_str(" LIMIT :limit"); params_map.insert(":limit".into(), Box::new(limit)); }
    }

    // Convert HashMap to slice for query_map_named
    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map.iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt.query_map(params_for_query.as_slice(), map_row_to_workout)
                       .map_err(DbError::QueryFailed)?;

    workout_iter.collect::<Result<Vec<_>, _>>().map_err(DbError::QueryFailed) // Collect results
}

 /// Lists workouts for a specific exercise (canonical name) performed on the Nth most recent day it was done.
 pub fn list_workouts_for_exercise_on_nth_last_day(
     conn: &Connection, exercise_name: &str, // Canonical name expected
     n: u32,
 ) -> Result<Vec<Workout>, DbError> { // Return DbError
     if n == 0 { return Err(DbError::QueryFailed(rusqlite::Error::InvalidParameterCount(n as usize, 2))); } // Indicate bad N via error
     let offset = n - 1;
     let sql = "WITH RankedDays AS (SELECT DISTINCT date(timestamp) as workout_date FROM workouts WHERE exercise_name = :ex_name COLLATE NOCASE ORDER BY workout_date DESC LIMIT 1 OFFSET :offset)
                SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
                FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
                WHERE w.exercise_name = :ex_name COLLATE NOCASE ORDER BY w.timestamp ASC;";

     let mut stmt = conn.prepare(sql).map_err(DbError::QueryFailed)?;
     let workout_iter = stmt.query_map(named_params! { ":ex_name": exercise_name, ":offset": offset }, map_row_to_workout)
                            .map_err(DbError::QueryFailed)?;

     workout_iter.collect::<Result<Vec<_>, _>>().map_err(DbError::QueryFailed)
 }

// ---- Exercise Definition Functions ----

/// Creates a new exercise definition. Returns ID. Handles UNIQUE constraint.
pub fn create_exercise(
    conn: &Connection, name: &str, type_: &ExerciseType, muscles: Option<&str>,
) -> Result<i64, DbError> { // Return DbError
    let type_str = type_.to_string();
    match conn.execute("INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)", params![name, type_str, muscles]) {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                 // Check for UNIQUE constraint violation specifically on 'exercises.name'
                 if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    // It's highly likely the name constraint. Return specific error.
                     return Err(DbError::ExerciseNameNotUnique(name.to_string()));
                 }
            }
            Err(DbError::InsertFailed(e)) // Wrap other errors
        }
    }
}

/// Updates an existing exercise definition (found by ID or name). Requires mutable conn for transaction.
/// Also handles updating associated aliases and workout entries if the name changes.
pub fn update_exercise(
    conn: &mut Connection, // Use mutable connection for transaction
    canonical_name_to_update: &str, // Use the resolved canonical name
    new_name: Option<&str>, new_type: Option<&ExerciseType>,
    new_muscles: Option<Option<&str>>,
) -> Result<u64, DbError> {
    // Find exercise by canonical name first to get ID and confirm existence
    let exercise = get_exercise_by_name(conn, canonical_name_to_update)?
        .ok_or_else(|| DbError::ExerciseNotFound(canonical_name_to_update.to_string()))?;
    let id = exercise.id;
    let original_name = exercise.name.clone(); // Clone needed for later comparison/updates

    let name_being_changed = new_name.is_some() && new_name != Some(original_name.as_str());
    let target_new_name = new_name.unwrap_or(&original_name);

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name { updates.push("name = :name"); params_map.insert(":name".into(), Box::new(name.to_string())); }
    if let Some(t) = new_type { updates.push("type = :type"); params_map.insert(":type".into(), Box::new(t.to_string())); }
    if let Some(m_opt) = new_muscles { updates.push("muscles = :muscles"); params_map.insert(":muscles".into(), Box::new(m_opt)); }

    if updates.is_empty() { return Ok(0); } // No fields to update

    // Use a transaction
    let tx = conn.transaction().map_err(DbError::Connection)?;

    // 1. Update exercises table
    let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map.iter().map(|(k, v)| (k.as_str(), v.as_ref())).collect();

    let rows_affected = match tx.execute(&sql_update_exercise, params_for_exec.as_slice()) {
        Ok(rows) => rows,
        Err(e) => {
             if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                 // Check for UNIQUE constraint violation specifically on 'exercises.name'
                 if err.code == rusqlite::ErrorCode::ConstraintViolation && name_being_changed {
                    // Name change failed due to unique constraint
                     return Err(DbError::ExerciseNameNotUnique(target_new_name.to_string()));
                 }
             }
             return Err(DbError::UpdateFailed(e)); // Other update error
        }
    };


    // 2. Update related tables if name changed
    if name_being_changed {
        // Update workouts table
        tx.execute("UPDATE workouts SET exercise_name = :new_name WHERE exercise_name = :old_name",
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(DbError::UpdateFailed)?;

        // Update aliases table (Feature 1)
        tx.execute("UPDATE aliases SET exercise_name = :new_name WHERE exercise_name = :old_name",
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(DbError::UpdateFailed)?;
    }

    tx.commit().map_err(DbError::Connection)?; // Commit transaction

    if rows_affected == 0 {
        // This case should ideally not happen if get_exercise_by_name succeeded, but handle defensively
        Err(DbError::ExerciseNotFound(original_name))
    } else {
        Ok(rows_affected as u64)
    }
}


 /// Deletes an exercise definition (found by canonical name).
 /// Also deletes associated aliases.
 /// Note: Warning about associated workouts is now handled in the AppService layer.
pub fn delete_exercise(conn: &mut Connection, canonical_name: &str) -> Result<u64, DbError> {
    // Find exercise by canonical name first to get ID and confirm existence
    let exercise = get_exercise_by_name(conn, canonical_name)?
        .ok_or_else(|| DbError::ExerciseNotFound(canonical_name.to_string()))?;
    let id = exercise.id;
    let name_to_delete = exercise.name.clone(); // Use the exact name from DB

    // Use a transaction (optional but safer if we add foreign keys later)
    let tx = conn.transaction().map_err(DbError::Connection)?;

    // 1. Delete associated aliases (Feature 1)
    tx.execute("DELETE FROM aliases WHERE exercise_name = ?", params![name_to_delete])
       .map_err(DbError::DeleteFailed)?;

    // 2. Delete the exercise definition
    let rows_affected = tx.execute("DELETE FROM exercises WHERE id = ?", params![id])
                          .map_err(DbError::DeleteFailed)?;

    tx.commit().map_err(DbError::Connection)?;

    if rows_affected == 0 {
         // Should not happen if get_exercise_by_name succeeded
        Err(DbError::ExerciseNotFound(name_to_delete))
    }
    else { Ok(rows_affected as u64) }
}


fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
     let id: i64 = row.get(0)?;
     let name: String = row.get(1)?;
     let type_str: String = row.get(2)?;
     let muscles: Option<String> = row.get(3)?;
     let ex_type = ExerciseType::try_from(type_str.as_str()).map_err(|e| {
         rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))
     })?;
     Ok(ExerciseDefinition { id, name, type_: ex_type, muscles })
}

/// Retrieves an exercise definition by its name (case-insensitive).
pub fn get_exercise_by_name(conn: &Connection, name: &str) -> Result<Option<ExerciseDefinition>, DbError> { // Return DbError
    let mut stmt = conn.prepare("SELECT id, name, type, muscles FROM exercises WHERE name = ?1 COLLATE NOCASE")
                      .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![name], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Retrieves an exercise definition by its ID.
pub fn get_exercise_by_id(conn: &Connection, id: i64) -> Result<Option<ExerciseDefinition>, DbError> { // Return DbError
    let mut stmt = conn.prepare("SELECT id, name, type, muscles FROM exercises WHERE id = ?1")
                      .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![id], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

// --- Alias Functions (Feature 1) ---

/// Creates a new alias for a given canonical exercise name.
pub fn create_alias(conn: &Connection, alias_name: &str, canonical_exercise_name: &str) -> Result<(), DbError> {
    match conn.execute(
        "INSERT INTO aliases (alias_name, exercise_name) VALUES (?1, ?2)",
        params![alias_name, canonical_exercise_name],
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
             if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                 // Check for UNIQUE constraint violation specifically on 'aliases.alias_name'
                 if err.code == rusqlite::ErrorCode::ConstraintViolation {
                     return Err(DbError::AliasAlreadyExists(alias_name.to_string()));
                 }
             }
             Err(DbError::InsertFailed(e)) // Wrap other errors
        }
    }
}

/// Deletes an alias by its name.
pub fn delete_alias(conn: &Connection, alias_name: &str) -> Result<u64, DbError> {
    let rows_affected = conn.execute("DELETE FROM aliases WHERE alias_name = ?1", params![alias_name])
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 {
        Err(DbError::AliasNotFound(alias_name.to_string()))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Retrieves the canonical exercise name associated with an alias (case-insensitive).
pub fn get_canonical_name_for_alias(conn: &Connection, alias_name: &str) -> Result<Option<String>, DbError> {
    let mut stmt = conn.prepare("SELECT exercise_name FROM aliases WHERE alias_name = ?1 COLLATE NOCASE")
                      .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![alias_name], |row| row.get(0))
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Lists all defined aliases and their corresponding canonical exercise names.
pub fn list_aliases(conn: &Connection) -> Result<HashMap<String, String>, DbError> {
    let mut stmt = conn.prepare("SELECT alias_name, exercise_name FROM aliases ORDER BY alias_name ASC")
                      .map_err(DbError::QueryFailed)?;
    let alias_iter = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).map_err(DbError::QueryFailed)?;

    alias_iter.collect::<Result<HashMap<_, _>, _>>().map_err(DbError::QueryFailed)
}


// --- Combined Identifier Resolution ---

/// Retrieves an exercise definition by trying ID first, then alias, then name.
/// Returns Option<(Definition, ResolvedByType)>.
#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedByType { Id, Alias, Name }

pub fn get_exercise_by_identifier(conn: &Connection, identifier: &str) -> Result<Option<(ExerciseDefinition, ResolvedByType)>, DbError> {
    // 1. Try parsing as ID
    if let Ok(id) = identifier.parse::<i64>() {
        if let Some(exercise) = get_exercise_by_id(conn, id)? {
            return Ok(Some((exercise, ResolvedByType::Id)));
        }
        // If it parsed as ID but wasn't found, don't proceed to alias/name check for IDs.
        // This prevents ambiguity if an alias/name happens to be numeric.
        return Ok(None);
    }

    // 2. Try resolving as Alias
    if let Some(canonical_name) = get_canonical_name_for_alias(conn, identifier)? {
        // Found alias, now get the definition using the canonical name
        match get_exercise_by_name(conn, &canonical_name)? {
            Some(exercise) => return Ok(Some((exercise, ResolvedByType::Alias))),
            None => {
                // Alias exists but points to a non-existent exercise (data inconsistency?)
                // Log warning or handle as appropriate. For now, return as not found.
                 eprintln!("Warning: Alias '{}' points to non-existent exercise '{}'.", identifier, canonical_name);
                 return Ok(None);
            }
        }
    }

    // 3. Try resolving as Name
    match get_exercise_by_name(conn, identifier)? {
        Some(exercise) => Ok(Some((exercise, ResolvedByType::Name))),
        None => Ok(None), // Not found by ID, Alias, or Name
    }
}

/// Lists defined exercises, optionally filtering by type and/or muscle.
pub fn list_exercises(
    conn: &Connection, type_filter: Option<ExerciseType>, muscle_filter: Option<&str>,
) -> Result<Vec<ExerciseDefinition>, DbError> { // Return DbError
    let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(t) = type_filter { sql.push_str(" AND type = :type"); params_map.insert(":type".into(), Box::new(t.to_string())); }
    if let Some(m) = muscle_filter { sql.push_str(" AND muscles LIKE :muscle"); params_map.insert(":muscle".into(), Box::new(format!("%{}%", m))); }
    sql.push_str(" ORDER BY name ASC");

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map.iter().map(|(k,v)| (k.as_str(), v.as_ref())).collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let exercise_iter = stmt.query_map(params_for_query.as_slice(), map_row_to_exercise_definition)
                            .map_err(DbError::QueryFailed)?;

    exercise_iter.collect::<Result<Vec<_>, _>>().map_err(DbError::QueryFailed) // Collect results
}


// --- Personal Best Query Functions (Feature 4) ---

/// Gets the maximum weight lifted for a specific exercise (canonical name).
pub fn get_max_weight_for_exercise(conn: &Connection, canonical_exercise_name: &str) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT MAX(weight) FROM workouts WHERE exercise_name = ?1 AND weight IS NOT NULL",
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<f64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum reps performed in a single set for a specific exercise (canonical name).
pub fn get_max_reps_for_exercise(conn: &Connection, canonical_exercise_name: &str) -> Result<Option<i64>, DbError> {
     conn.query_row(
        // Note: This assumes reps are per set. If reps column means total reps for the entry, the interpretation changes.
        // Assuming reps is 'reps per set'.
        "SELECT MAX(reps) FROM workouts WHERE exercise_name = ?1 AND reps IS NOT NULL",
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<i64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}


//src/lib.rs
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc}; // Add TimeZone
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::collections::HashMap; // For list_aliases return type

// --- Declare modules to load from separate files ---
mod config;
pub mod db;

// --- Expose public types ---
pub use config::{
    Config, ConfigError, ThemeConfig, Units, StandardColor, parse_color,
    get_config_path as get_config_path_util, // Rename utility function
    load_config as load_config_util,         // Rename utility function
    save_config as save_config_util,         // Rename utility function
};
pub use db::{
    DbError, ExerciseDefinition, ExerciseType, Workout, WorkoutFilters, ResolvedByType,
    get_db_path as get_db_path_util, // Rename utility function
};

// --- Personal Best Information (Feature 4) ---
#[derive(Debug, Clone, PartialEq)]
pub enum PBType {
    Weight,
    Reps,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PBInfo {
    pub pb_type: PBType,
    pub new_weight: Option<f64>,
    pub previous_weight: Option<f64>,
    pub new_reps: Option<i64>,
    pub previous_reps: Option<i64>,
}


// --- Service Layer ---

/// Main application service holding configuration and database connection.
pub struct AppService {
    pub config: Config, // Public for reading by UI layers (CLI, TUI)
    pub conn: Connection, // Make mutable ONLY if needed (e.g., transactions directly in service) - currently DB funcs handle transactions
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

impl AppService {
    /// Initializes the application service by loading config and connecting to the DB.
    pub fn initialize() -> Result<Self> {
        let config_path = config::get_config_path()
            .context("Failed to determine configuration file path")?;
        // Use the load_config function from the config module
        let config = config::load_config(&config_path)
            .context(format!("Failed to load config from {:?}", config_path))?;

        let db_path = db::get_db_path().context("Failed to determine database path")?;
        let conn = db::open_db(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        db::init_db(&conn).context("Failed to initialize database schema")?;

        Ok(Self {
            config,
            conn,
            db_path,
            config_path,
        })
    }

    // --- Configuration Methods ---

    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn save_config(&self) -> Result<(), ConfigError> {
        config::save_config(&self.config_path, &self.config)
    }

    /// Sets the bodyweight in the configuration and saves it.
    pub fn set_bodyweight(&mut self, weight: f64) -> Result<(), ConfigError> {
         if weight <= 0.0 {
             return Err(ConfigError::InvalidBodyweightInput("Weight must be a positive number.".to_string()));
         }
         self.config.bodyweight = Some(weight);
         self.save_config()?;
         Ok(())
    }

     /// Checks if bodyweight is needed and returns it, or returns error if needed but not set.
     /// Does NOT prompt.
     pub fn get_required_bodyweight(&self) -> Result<f64, ConfigError> {
         self.config.bodyweight.ok_or_else(|| ConfigError::BodyweightNotSet(self.config_path.clone()))
     }

     /// Disables the bodyweight prompt in the config and saves it.
     pub fn disable_bodyweight_prompt(&mut self) -> Result<(), ConfigError> {
         self.config.prompt_for_bodyweight = false;
         self.save_config()
     }

     /// Sets the PB notification preference in the config and saves it. (Feature 4)
     pub fn set_pb_notification(&mut self, enabled: bool) -> Result<(), ConfigError> {
         self.config.notify_on_pb = Some(enabled);
         self.save_config()
     }

     /// Checks the PB notification config setting. Returns error if not set. (Feature 4)
     pub fn check_pb_notification_config(&self) -> Result<bool, ConfigError> {
         self.config.notify_on_pb.ok_or(ConfigError::PbNotificationNotSet)
     }

    // --- Database Path ---
    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }

    // --- Exercise Identifier Resolution (Helper) ---

    /// Resolves an identifier (ID, Alias, Name) to an ExerciseDefinition.
    /// Returns Ok(None) if not found, Err if DB error occurs.
    pub  fn resolve_exercise_identifier(&self, identifier: &str) -> Result<Option<ExerciseDefinition>> {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        // Call function from db module
        db::get_exercise_by_identifier(&self.conn, trimmed)
            .map(|opt_result| opt_result.map(|(definition, _)| definition)) // Discard ResolvedByType here
            .context(format!("Failed to resolve exercise identifier '{}'", identifier))
    }

     /// Resolves an identifier (ID, Alias, Name) to its canonical name.
     /// Returns Ok(None) if not found, Err if DB error occurs.
    fn resolve_identifier_to_canonical_name(&self, identifier: &str) -> Result<Option<String>> {
        self.resolve_exercise_identifier(identifier)
            .map(|opt_def| opt_def.map(|def| def.name))
    }


    // --- Exercise Definition Methods ---

    /// Creates a new exercise definition. (Feature 2: Name uniqueness enforced by DB)
    pub fn create_exercise(
        &self,
        name: &str,
        type_: ExerciseType, // Use db::ExerciseType directly
        muscles: Option<&str>,
    ) -> Result<i64> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            bail!("Exercise name cannot be empty.");
        }
        // Call function from db module
        db::create_exercise(&self.conn, trimmed_name, &type_, muscles)
            .map_err(|db_err| match db_err {
                 DbError::ExerciseNameNotUnique(_) => anyhow::anyhow!(db_err), // Keep specific error message
                 _ => anyhow::Error::new(db_err).context(format!("Failed to create exercise '{}' in database", trimmed_name)),
            })
    }

    /// Edits an existing exercise definition (identified by ID, Alias, or Name).
    /// Handles updates to workouts and aliases if name changes.
    pub fn edit_exercise(
        &mut self, // Takes mut self because db::update_exercise requires it for transaction
        identifier: &str,
        new_name: Option<&str>,
        new_type: Option<ExerciseType>,
        new_muscles: Option<Option<&str>>, // None = don't change, Some(None) = clear, Some(Some("val")) = set
    ) -> Result<u64> {
        // 1. Resolve identifier to get the *current* canonical name
        let current_def = self.resolve_exercise_identifier(identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let canonical_name_to_update = &current_def.name;

        // Trim new name if provided
        let trimmed_new_name = new_name.map(|n| n.trim()).filter(|n| !n.is_empty());
        if new_name.is_some() && trimmed_new_name.is_none() {
             bail!("New exercise name cannot be empty if provided.");
        }


        // 2. Call DB function with the canonical name
        // Need mutable connection borrow for the transaction inside db::update_exercise
        let mut_conn = &mut self.conn; // Create a mutable reference
        db::update_exercise(
            mut_conn,
            canonical_name_to_update,
            trimmed_new_name,
            new_type.as_ref(), // Pass Option<&DbType>
            new_muscles,
        )
        .map_err(|db_err| match db_err {
            // Make unique constraint violation error more specific
             DbError::ExerciseNameNotUnique(failed_name) => {
                  anyhow::anyhow!("Failed to rename exercise: the name '{}' is already taken.", failed_name)
             }
             _ => anyhow::Error::new(db_err).context(format!("Failed to update exercise '{}' in database", identifier))
        })
    }


    /// Deletes an exercise definition (identified by ID, Alias, or Name). Returns number of definitions deleted (0 or 1).
    /// Includes warnings about associated workouts and deletes associated aliases.
    pub fn delete_exercise(&mut self, identifier: &str) -> Result<u64> {
        // 1. Resolve identifier to get canonical name and check existence
        let exercise_def = self.resolve_exercise_identifier(identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let canonical_name = exercise_def.name.clone(); // Clone needed for messages/DB call

        // 2. Check for associated workouts (using canonical name)
        let workout_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE",
            [&canonical_name], // Use canonical name
            |row| row.get(0),
        ).context(format!("Failed to check for associated workouts for '{}'", canonical_name))?;

        if workout_count > 0 {
             // Print warning here. Ideally UI layer formats this, but simpler here for now.
             eprintln!(
                "Warning: Deleting exercise '{}'. {} associated workout entries will remain but reference a deleted definition.",
                canonical_name, workout_count
             );
        }

        // 3. Call DB function to delete exercise and its aliases (using canonical name)
        db::delete_exercise(&mut self.conn, &canonical_name)
            .context(format!("Failed to delete exercise '{}' from database", canonical_name))
    }


    /// Retrieves an exercise definition by ID, Alias or name.
    pub fn get_exercise_by_identifier_service(&self, identifier: &str) -> Result<Option<ExerciseDefinition>> {
        self.resolve_exercise_identifier(identifier)
    }

    /// Lists exercise definitions based on filters.
    pub fn list_exercises(
        &self,
        type_filter: Option<ExerciseType>,
        muscle_filter: Option<&str>,
    ) -> Result<Vec<ExerciseDefinition>> {
        // Call function from db module
        db::list_exercises(&self.conn, type_filter, muscle_filter)
            .context("Failed to list exercise definitions from database")
    }

    // --- Alias Methods (Feature 1) ---

    /// Creates a new alias for an exercise.
    pub fn create_alias(&self, alias_name: &str, exercise_identifier: &str) -> Result<()> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
         // Ensure alias doesn't clash with existing exercise IDs or names
        if let Some((_, resolved_type)) = db::get_exercise_by_identifier(&self.conn, trimmed_alias)? {
             match resolved_type {
                 ResolvedByType::Id => bail!("Alias '{}' conflicts with an existing exercise ID.", trimmed_alias),
                 ResolvedByType::Name => bail!("Alias '{}' conflicts with an existing exercise name.", trimmed_alias),
                 ResolvedByType::Alias => { /* This is handled by the INSERT constraint */ }
             }
        }

        // Resolve the target exercise identifier to its canonical name
        let canonical_name = self.resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Call DB function
        db::create_alias(&self.conn, trimmed_alias, &canonical_name)
            .map_err(|db_err| match db_err {
                 DbError::AliasAlreadyExists(_) => anyhow::anyhow!(db_err), // Keep specific error
                 _=> anyhow::Error::new(db_err).context(format!("Failed to create alias '{}' for exercise '{}'", trimmed_alias, canonical_name)),
            })
    }

    /// Deletes an exercise alias.
    pub fn delete_alias(&self, alias_name: &str) -> Result<u64> {
        let trimmed_alias = alias_name.trim();
         if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        db::delete_alias(&self.conn, trimmed_alias)
             .map_err(|db_err| match db_err {
                  DbError::AliasNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
                  _=> anyhow::Error::new(db_err).context(format!("Failed to delete alias '{}'", trimmed_alias)),
             })
    }

    /// Lists all defined aliases.
    pub fn list_aliases(&self) -> Result<HashMap<String, String>> {
        db::list_aliases(&self.conn).context("Failed to list aliases from database")
    }

    // --- Workout Entry Methods ---

    /// Adds a workout entry. Handles implicit exercise creation, bodyweight logic, past dates, and PB checking.
    /// Returns Result<(workout_id, Option<PBInfo>)>
    pub fn add_workout(
        &mut self, // Needs mut because bodyweight prompt might update config via caller
        exercise_identifier: &str,
        date: NaiveDate, 
        sets: Option<i64>,
        reps: Option<i64>,
        weight_arg: Option<f64>, // Weight from CLI/TUI args
        duration: Option<i64>,
        notes: Option<String>,
        // For implicit creation:
        implicit_type: Option<ExerciseType>,
        implicit_muscles: Option<String>,
        // Bodyweight handling (determined by caller):
        bodyweight_to_use: Option<f64>, // If type is BodyWeight and caller prompted/knows BW
    ) -> Result<(i64, Option<PBInfo>)> { // Feature 4: Return PB info
        // 1. Resolve identifier / Implicitly create Exercise Definition
        let exercise_def = match self.resolve_exercise_identifier(exercise_identifier)? {
             Some(def) => def,
             None => {
                  // Try implicit creation
                  if let (Some(db_type), Some(muscle_list)) = (implicit_type, implicit_muscles) {
                      println!( // Keep CLI print for now
                          "Exercise '{}' not found, defining it implicitly...",
                          exercise_identifier
                      );
                      let muscles_opt = if muscle_list.trim().is_empty() { None } else { Some(muscle_list.as_str()) };
                      match self.create_exercise(exercise_identifier, db_type, muscles_opt) {
                          Ok(id) => {
                               println!("Implicitly defined exercise: '{}' (ID: {})", exercise_identifier, id);
                               // Refetch the newly created definition
                               self.resolve_exercise_identifier(exercise_identifier)?
                                   .ok_or_else(|| anyhow::anyhow!("Failed to re-fetch implicitly created exercise '{}'", exercise_identifier))?
                          }
                          Err(e) => {
                               return Err(e).context(format!("Failed to implicitly define exercise '{}'", exercise_identifier));
                          }
                      }
                  } else {
                      // Not found and no implicit creation info provided
                      bail!(
                          "Exercise '{}' not found. Define it first using 'create-exercise', use an alias, or provide details for implicit creation.",
                          exercise_identifier
                      );
                  }
             }
        };

        let canonical_exercise_name = exercise_def.name.clone(); // Use canonical name from here

        // 2. Determine final weight based on type and provided bodyweight
        let final_weight = if exercise_def.type_ == ExerciseType::BodyWeight {
            match bodyweight_to_use {
                 Some(bw) => Some(bw + weight_arg.unwrap_or(0.0)),
                 None => bail!("Bodyweight is required for exercise '{}' but was not provided.", canonical_exercise_name),
            }
        } else {
             weight_arg // Use the provided weight directly for non-bodyweight exercises
        };

        // 3. Determine timestamp (Feature 3)
        // Use noon UTC on the given date to represent the day without time specifics
        let date_and_time = NaiveDateTime::from(date);
        let timestamp = Utc.from_utc_datetime(&date_and_time);

         // 4. Check for PBs *before* adding the new workout (Feature 4)
         let previous_max_weight = db::get_max_weight_for_exercise(&self.conn, &canonical_exercise_name)?;
         let previous_max_reps = db::get_max_reps_for_exercise(&self.conn, &canonical_exercise_name)?;


        // 5. Add the workout entry using the canonical exercise name, final weight, and timestamp
        let inserted_id = db::add_workout(
            &self.conn,
            &canonical_exercise_name, // Use canonical name
            timestamp,
            sets,
            reps,
            final_weight, // Use calculated weight
            duration,
            notes,
        )
        .context("Failed to add workout to database")?;

         // 6. Determine if a PB was achieved (Feature 4)
         let mut pb_info: Option<PBInfo> = None;
         let mut is_weight_pb = false;
         let mut is_reps_pb = false;

         if let Some(current_weight) = final_weight {
              if current_weight > 0.0 && current_weight > previous_max_weight.unwrap_or(0.0) {
                  is_weight_pb = true;
              }
         }
         if let Some(current_reps) = reps {
             if current_reps > 0 && current_reps > previous_max_reps.unwrap_or(0) {
                  is_reps_pb = true;
             }
         }

         if is_weight_pb && is_reps_pb {
             pb_info = Some(PBInfo { pb_type: PBType::Both, new_weight: final_weight, previous_weight: previous_max_weight, new_reps: reps, previous_reps: previous_max_reps });
         } else if is_weight_pb {
             pb_info = Some(PBInfo { pb_type: PBType::Weight, new_weight: final_weight, previous_weight: previous_max_weight, new_reps: None, previous_reps: None });
         } else if is_reps_pb {
             pb_info = Some(PBInfo { pb_type: PBType::Reps, new_weight: None, previous_weight: None, new_reps: reps, previous_reps: previous_max_reps });
         }

        Ok((inserted_id, pb_info)) // Return ID and optional PB info
    }


    /// Edits an existing workout entry.
    pub fn edit_workout(
        &self,
        id: i64,
        new_exercise_identifier: Option<String>,
        new_sets: Option<i64>,
        new_reps: Option<i64>,
        new_weight: Option<f64>, // Weight is set directly, no bodyweight logic re-applied
        new_duration: Option<i64>,
        new_notes: Option<String>,
        new_date: Option<NaiveDate>, // Feature 3: Allow editing date
    ) -> Result<u64> {
        // Resolve the new exercise identifier to its canonical name if provided
        let new_canonical_name: Option<String> = match new_exercise_identifier {
            Some(ident) => Some(self.resolve_identifier_to_canonical_name(&ident)?
                              .ok_or_else(|| DbError::ExerciseNotFound(ident.clone()))?),
            None => None,
        };

        // Convert new_date to new_timestamp if provided
        let new_timestamp: Option<DateTime<Utc>> = match new_date {
            Some(date) => Some(date.and_hms_opt(12, 0, 0) // Create NaiveDateTime first
                              .and_then(|naive_dt| Utc.from_local_datetime(&naive_dt).single()) // Convert to DateTime<Utc>
                              .ok_or_else(|| anyhow::anyhow!("Failed to create valid timestamp from date {}", date))?),
            None => None,
        };

        // Call function from db module
        db::update_workout(
            &self.conn,
            id,
            new_canonical_name.as_deref(), // Pass Option<&str>
            new_sets,
            new_reps,
            new_weight, // Pass Option<f64> directly
            new_duration,
            new_notes.as_deref(), // Pass Option<&str>
            new_timestamp, // Pass Option<DateTime<Utc>>
        )
        .with_context(|| format!("Failed to update workout ID {}", id))
    }


    /// Deletes a workout entry by ID.
    pub fn delete_workout(&self, id: i64) -> Result<u64> {
        // Call function from db module
        db::delete_workout(&self.conn, id)
             .map_err(|db_err| match db_err {
                  DbError::WorkoutNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
                  _ => anyhow::Error::new(db_err).context(format!("Failed to delete workout ID {}", id)),
             })
    }

    /// Lists workouts based on filters. Resolves exercise identifier if provided.
    pub fn list_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>> {
        // Resolve exercise identifier filter to canonical name if present
        let canonical_exercise_name = match filters.exercise_name {
             Some(ident) => Some(self.resolve_identifier_to_canonical_name(ident)?
                               .ok_or_else(|| {
                                    // If identifier doesn't resolve, treat as no matching workouts found
                                    eprintln!("Warning: Exercise identifier '{}' not found for filtering.", ident);
                                    DbError::ExerciseNotFound(ident.to_string()) // Or return Ok(vec![])? Let's error.
                               })?),
             None => None,
        };

         // Create new filters struct with resolved name
        let resolved_filters = WorkoutFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            date: filters.date,
            exercise_type: filters.exercise_type,
            muscle: filters.muscle,
            limit: filters.limit,
        };

        // Call function from db module
        db::list_workouts_filtered(&self.conn, resolved_filters)
            .context("Failed to list workouts from database")
    }

     /// Lists workouts for the Nth most recent day a specific exercise (ID, Alias, Name) was performed.
     pub fn list_workouts_for_exercise_on_nth_last_day(
         &self,
         exercise_identifier: &str,
         n: u32,
     ) -> Result<Vec<Workout>> {
         // Resolve identifier to canonical name
         let canonical_name = self.resolve_identifier_to_canonical_name(exercise_identifier)?
             .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

         // Call function from db module
         db::list_workouts_for_exercise_on_nth_last_day(&self.conn, &canonical_name, n)
             .map_err(|e| anyhow::Error::new(e)) // Convert DbError to anyhow::Error
             .with_context(|| format!("Failed to list workouts for exercise '{}' on nth last day {}", canonical_name, n))
     }

}


//src/main.rs
mod cli; // Keep cli module for parsing args

use anyhow::{bail, Context, Result};
use chrono::{Utc, Duration}; // Keep Duration if needed, remove if not
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use std::io::{stdin, Write}; // For prompts

use workout_tracker_lib::{
    AppService, ConfigError, ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters,
    PBInfo, PBType, // Import PB types
};

fn main() -> Result<()> {
    // Initialize the application service (loads config, connects to DB)
    let mut service = AppService::initialize().context("Failed to initialize application service")?;

    // Parse command-line arguments using the cli module
    let cli_args = cli::parse_args();

    // --- Execute Commands using AppService ---
    match cli_args.command {
        // --- Exercise Definition Commands ---
        cli::Commands::CreateExercise { name, type_, muscles } => {
            let db_type = cli_type_to_db_type(type_);
            match service.create_exercise(&name, db_type, muscles.as_deref()) {
                Ok(id) => println!(
                    "Successfully defined exercise: '{}' (Type: {}, Muscles: {}) ID: {}",
                    name.trim(),
                    db_type, // Use Display impl from lib::ExerciseType
                    muscles.as_deref().unwrap_or("None"),
                    id
                ),
                Err(e) => bail!("Error creating exercise: {}", e), // Handles unique name error message from service
            }
        }
        cli::Commands::EditExercise { identifier, name, type_, muscles, } => {
            let db_type = type_.map(cli_type_to_db_type);
            let muscles_update = match muscles {
                Some(ref s) if s.trim().is_empty() => Some(None), // Clear
                Some(ref s) => Some(Some(s.trim())),             // Set
                None => None,                                    // Don't change
            };

            match service.edit_exercise(&identifier, name.as_deref(), db_type, muscles_update) {
                 Ok(0) => println!("Exercise '{}' not found or no changes specified.", identifier),
                 Ok(rows) => {
                     println!("Successfully updated exercise definition '{}' ({} row(s) affected).", identifier, rows);
                    if name.is_some() {
                        println!("Note: If the name was changed, corresponding workout entries and aliases were also updated.");
                    }
                 }
                 Err(e) => bail!("Error editing exercise '{}': {}", identifier, e), // Handles unique name error from service
            }
        }
        cli::Commands::DeleteExercise { identifier } => {
             match service.delete_exercise(&identifier) {
                Ok(0) => println!("Exercise definition '{}' not found.", identifier), // Should ideally be Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted exercise definition '{}' ({} row(s) affected). Associated aliases were also deleted.", identifier, rows),
                Err(e) => bail!("Error deleting exercise '{}': {}", identifier, e),
            }
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise, date, // Feature 3: Get date from args
            sets, reps, weight, duration, notes,
            implicit_type, implicit_muscles,
        } => {
            let identifier_trimmed = exercise.trim();
             if identifier_trimmed.is_empty() {
                 bail!("Exercise identifier cannot be empty for adding a workout.");
             }

             // Determine if bodyweight might be needed *before* calling add_workout
             let mut bodyweight_to_use: Option<f64> = None;
             let mut needs_bw_check = false;

             // Peek at exercise type using the service resolver
             let exercise_def_peek = service.get_exercise_by_identifier_service(identifier_trimmed)?;
             if let Some(ref def) = exercise_def_peek {
                 if def.type_ == ExerciseType::BodyWeight { needs_bw_check = true; }
             } else if let Some(cli::ExerciseTypeCli::BodyWeight) = implicit_type {
                 needs_bw_check = true;
             }


             // If bodyweight exercise, check config and potentially prompt
             if needs_bw_check {
                  match service.get_required_bodyweight() {
                     Ok(bw) => {
                         bodyweight_to_use = Some(bw);
                         println!("Using configured bodyweight: {} {:?} (+ {} additional)",
                             bw, service.config.units, weight.unwrap_or(0.0));
                     }
                     Err(ConfigError::BodyweightNotSet(_)) => {
                         if service.config.prompt_for_bodyweight {
                             match prompt_and_set_bodyweight_cli(&mut service) {
                                 Ok(bw_from_prompt) => {
                                     bodyweight_to_use = Some(bw_from_prompt);
                                      println!("Using newly set bodyweight: {} {:?} (+ {} additional)",
                                        bw_from_prompt, service.config.units, weight.unwrap_or(0.0));
                                 }
                                 Err(ConfigError::BodyweightPromptCancelled) => {
                                     bail!("Bodyweight not set. Cannot add bodyweight exercise entry. Prompt disabled.");
                                 }
                                 Err(e) => bail!("Failed to get bodyweight via prompt: {}", e),
                             }
                         } else {
                              bail!(ConfigError::BodyweightNotSet(service.get_config_path().to_path_buf()));
                         }
                     }
                     Err(e) => bail!("Error checking bodyweight configuration: {}", e),
                  }
             }


            // Call the service add_workout method
            let db_implicit_type = implicit_type.map(cli_type_to_db_type);
            let units = service.config.units;
            match service.add_workout(
                identifier_trimmed, date, // Pass date
                sets, reps, weight, duration, notes,
                db_implicit_type, implicit_muscles, // Pass implicit creation details
                bodyweight_to_use, // Pass the resolved bodyweight (if applicable)
            ) {
                 Ok((id, pb_info_opt)) => { // Feature 4: Get PB info
                     // Use the potentially *canonical* name if implicit creation happened or alias used
                     let final_exercise_name = service.get_exercise_by_identifier_service(identifier_trimmed)?
                                                     .map(|def| def.name)
                                                     .unwrap_or_else(|| identifier_trimmed.to_string()); // Fallback if refetch fails (shouldn't happen)
                     println!(
                         "Successfully added workout for '{}' on {} ID: {}",
                         final_exercise_name, date.format("%Y-%m-%d"), id
                     );

                     // Handle PB notification (Feature 4)
                     if let Some(pb_info) = pb_info_opt {
                         handle_pb_notification(&mut service, &pb_info, units)?;
                     }
                 }
                 Err(e) => bail!("Error adding workout: {}", e),
             }
        }

        cli::Commands::EditWorkout { id, exercise, sets, reps, weight, duration, notes, date } => { // Feature 3: Handle date edit
            match service.edit_workout(id, exercise, sets, reps, weight, duration, notes, date) { // Pass date to service
                Ok(0) => println!("Workout ID {} not found or no changes specified.", id),
                Ok(rows) => println!("Successfully updated workout ID {} ({} row(s) affected).", id, rows),
                Err(e) => bail!("Error editing workout ID {}: {}", id, e),
            }
        }
        cli::Commands::DeleteWorkout { id } => {
            match service.delete_workout(id) {
                Ok(0) => println!("Workout ID {} not found.", id),
                Ok(rows) => println!("Successfully deleted workout ID {} ({} row(s) affected).", id, rows),
                Err(e) => bail!("Error deleting workout ID {}: {}", id, e),
            }
        }

        cli::Commands::List {
            limit, today_flag, yesterday_flag, date, exercise, type_, muscle,
            nth_last_day_exercise, nth_last_day_n,
        } => {
             // Determine date based on flags or explicit date arg
             let effective_date = if today_flag { Some(Utc::now().date_naive()) }
                                else if yesterday_flag { Some((Utc::now() - Duration::days(1)).date_naive()) }
                                else { date };

             let workouts_result = if let Some(ex_ident) = nth_last_day_exercise {
                  let n = nth_last_day_n.context("Missing N value for --nth-last-day")?;
                  // Service method now resolves identifier internally
                  service.list_workouts_for_exercise_on_nth_last_day(&ex_ident, n)
             } else {
                  let db_type_filter = type_.map(cli_type_to_db_type);
                  // Limit applies only if no date filter and not using nth_last_day
                  let effective_limit = if effective_date.is_none() && nth_last_day_n.is_none() { Some(limit) } else { None };

                  // Service method now resolves identifier internally if provided
                  let filters = WorkoutFilters {
                      exercise_name: exercise.as_deref(), // Pass identifier directly
                      date: effective_date,
                      exercise_type: db_type_filter,
                      muscle: muscle.as_deref(),
                      limit: effective_limit,
                  };
                  service.list_workouts(filters)
             };

             match workouts_result {
                Ok(workouts) if workouts.is_empty() => {
                    println!("No workouts found matching the criteria.");
                }
                Ok(workouts) => {
                    let header_color = workout_tracker_lib::parse_color(&service.config.theme.header_color)
                        .map(Color::from)
                        .unwrap_or(Color::Green); // Fallback
                    print_workout_table(workouts, header_color, service.config.units);
                }
                Err(e) => bail!("Error listing workouts: {}", e),
             }
        }
        cli::Commands::ListExercises { type_, muscle } => {
            let db_type_filter = type_.map(cli_type_to_db_type);
            match service.list_exercises(db_type_filter, muscle.as_deref()) {
                Ok(exercises) if exercises.is_empty() => {
                    println!("No exercise definitions found matching the criteria.");
                }
                Ok(exercises) => {
                     let header_color = workout_tracker_lib::parse_color(&service.config.theme.header_color)
                         .map(Color::from)
                         .unwrap_or(Color::Cyan); // Fallback
                     print_exercise_definition_table(exercises, header_color);
                }
                Err(e) => bail!("Error listing exercises: {}", e),
            }
        }
        // --- Alias Commands (Feature 1) ---
        cli::Commands::Alias { alias_name, exercise_identifier } => {
            match service.create_alias(&alias_name, &exercise_identifier) {
                Ok(()) => println!("Successfully created alias '{}' for exercise '{}'.", alias_name, exercise_identifier),
                Err(e) => bail!("Error creating alias: {}", e),
            }
        }
        cli::Commands::Unalias { alias_name } => {
            match service.delete_alias(&alias_name) {
                Ok(0) => println!("Alias '{}' not found.", alias_name), // Should be Err from service
                Ok(rows) => println!("Successfully deleted alias '{}' ({} row(s) affected).", alias_name, rows),
                Err(e) => bail!("Error deleting alias '{}': {}", alias_name, e),
            }
        }
        cli::Commands::ListAliases => {
            match service.list_aliases() {
                Ok(aliases) if aliases.is_empty() => println!("No aliases defined."),
                Ok(aliases) => print_alias_table(aliases),
                Err(e) => bail!("Error listing aliases: {}", e),
            }
        }
        // --- Config/Path Commands ---
        cli::Commands::DbPath => {
            println!("Database file is located at: {:?}", service.get_db_path());
        }
        cli::Commands::ConfigPath => {
            println!("Config file is located at: {:?}", service.get_config_path());
        }
        cli::Commands::SetBodyweight { weight } => {
            match service.set_bodyweight(weight) {
                 Ok(()) => {
                     println!( "Successfully set bodyweight to: {} {:?}", weight, service.config.units );
                     println!("Config file updated: {:?}", service.get_config_path());
                 }
                 Err(e) => bail!("Error setting bodyweight: {}", e),
            }
        }
         cli::Commands::SetPbNotification { enabled } => { // Feature 4
            match service.set_pb_notification(enabled) {
                Ok(()) => {
                    println!(
                        "Successfully {} Personal Best notifications.",
                        if enabled { "enabled" } else { "disabled" }
                    );
                    println!("Config file updated: {:?}", service.get_config_path());
                }
                Err(e) => bail!("Error updating PB notification setting: {}", e),
            }
        }
    }

    Ok(())
}

// --- CLI Specific Helper Functions ---

/// Converts CLI ExerciseType enum to DB ExerciseType enum (from lib)
fn cli_type_to_db_type(cli_type: cli::ExerciseTypeCli) -> ExerciseType {
    match cli_type {
        cli::ExerciseTypeCli::Resistance => ExerciseType::Resistance,
        cli::ExerciseTypeCli::Cardio => ExerciseType::Cardio,
        cli::ExerciseTypeCli::BodyWeight => ExerciseType::BodyWeight,
    }
}

/// Interactive prompt for bodyweight, specific to the CLI.
/// Updates the service's config and saves it.
fn prompt_and_set_bodyweight_cli(service: &mut AppService) -> Result<f64, ConfigError> {
    // Prompt is needed (caller should ensure service.config.prompt_for_bodyweight is true)
    println!("Bodyweight is required for this exercise type but is not set.");
    println!("Please enter your current bodyweight (in {:?}).", service.config.units);
    print!("Enter weight, or 'N' to not be asked again (use 'set-bodyweight' later): ");
    std::io::stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?; // Use ConfigError::Io
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!("Please use the 'set-bodyweight <weight>' command to set it manually.");
        // Update config via service method to handle saving
        service.disable_bodyweight_prompt()?;
        Err(ConfigError::BodyweightPromptCancelled)
    } else {
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!("Setting bodyweight to {} {:?}", weight, service.config.units);
                // Update config via service method
                service.set_bodyweight(weight)?; // This also saves the config
                Ok(weight)
            }
            Ok(_) => Err(ConfigError::InvalidBodyweightInput("Weight must be a positive number.".to_string())),
            Err(e) => Err(ConfigError::InvalidBodyweightInput(format!("Could not parse '{}': {}", trimmed_input, e))),
        }
    }
}


/// Handles PB notification logic, including prompting if config not set (Feature 4)
fn handle_pb_notification(service: &mut AppService, pb_info: &PBInfo, units: Units) -> Result<()> {
    let print_notification = match service.check_pb_notification_config() {
        Ok(enabled) => enabled, // Config is set, use the value
        Err(ConfigError::PbNotificationNotSet) => {
             // Config not set, prompt the user
             prompt_and_set_pb_notification_cli(service)? // Returns true if user enables, false if disables
        }
        Err(e) => return Err(e.into()), // Other config error
    };

    if print_notification {
        print_pb_message(pb_info, units);
    }
    Ok(())
}

/// Prints the formatted PB message.
fn print_pb_message(pb_info: &PBInfo, units: Units) {
    let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
    println!("*********************************");
    println!("*     🎉 Personal Best! 🎉     *");
    match pb_info.pb_type {
        PBType::Weight => {
             println!("* New Max Weight: {:.2} {} {}",
                pb_info.new_weight.unwrap_or(0.0),
                weight_unit_str,
                pb_info.previous_weight.map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
            );
        },
        PBType::Reps => {
            println!("* New Max Reps: {} {}",
                pb_info.new_reps.unwrap_or(0),
                pb_info.previous_reps.map_or("".to_string(), |p| format!("(Previous: {})", p))
            );
        },
        PBType::Both => {
            println!("* New Max Weight: {:.2} {} {}",
                pb_info.new_weight.unwrap_or(0.0),
                weight_unit_str,
                pb_info.previous_weight.map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
            );
            println!("* New Max Reps: {} {}",
                pb_info.new_reps.unwrap_or(0),
                pb_info.previous_reps.map_or("".to_string(), |p| format!("(Previous: {})", p))
            );
        },
    }
     println!("*********************************");
}

/// Interactive prompt for PB notification setting, specific to the CLI (Feature 4)
/// Updates the service's config and saves it. Returns the chosen setting (true/false).
fn prompt_and_set_pb_notification_cli(service: &mut AppService) -> Result<bool, ConfigError> {
    println!("You achieved a Personal Best!");
    print!("Do you want to be notified about PBs in the future? (Y/N): ");
    std::io::stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("y") {
        println!("Okay, enabling future PB notifications.");
        service.set_pb_notification(true)?;
        Ok(true)
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future PB notifications.");
        service.set_pb_notification(false)?;
        Ok(false)
    } else {
         // Invalid input, treat as cancellation for this time, don't update config
         println!("Invalid input. PB notifications remain unset for now.");
         Err(ConfigError::PbNotificationPromptCancelled) // Indicate cancellation/invalid input
    }
}


// --- Table Printing Functions (Remain in CLI) ---

/// Prints workout entries in a formatted table.
fn print_workout_table(workouts: Vec<Workout>, header_color: Color, units: Units) {
    let mut table = Table::new();
    let weight_unit_str = match units {
        Units::Metric => "(kg)",
        Units::Imperial => "(lbs)",
    };

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(header_color),
            Cell::new("Timestamp (UTC)").fg(header_color), // Display full timestamp
            Cell::new("Exercise").fg(header_color),
            Cell::new("Type").fg(header_color),
            Cell::new("Sets").fg(header_color),
            Cell::new("Reps").fg(header_color),
            Cell::new(format!("Weight {}", weight_unit_str)).fg(header_color),
            Cell::new("Duration (min)").fg(header_color),
            Cell::new("Notes").fg(header_color),
        ]);

    for workout in workouts {
        table.add_row(vec![
            Cell::new(workout.id.to_string()),
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()), // Format for display
            Cell::new(workout.exercise_name), // Canonical name
            Cell::new(workout.exercise_type.map_or("-".to_string(), |t| t.to_string())),
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.weight.map_or("-".to_string(), |v| format!("{:.2}", v))),
            Cell::new(workout.duration_minutes.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.notes.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
}

/// Prints exercise definitions in a formatted table.
fn print_exercise_definition_table(exercises: Vec<ExerciseDefinition>, header_color: Color) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(header_color),
            Cell::new("Name").fg(header_color),
            Cell::new("Type").fg(header_color),
            Cell::new("Muscles").fg(header_color),
        ]);

    for exercise in exercises {
        table.add_row(vec![
            Cell::new(exercise.id.to_string()),
            Cell::new(exercise.name),
            Cell::new(exercise.type_.to_string()), // Uses Display impl from lib
            Cell::new(exercise.muscles.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
}

/// Prints aliases in a formatted table. (Feature 1)
fn print_alias_table(aliases: std::collections::HashMap<String, String>) {
    let mut table = Table::new();
    let header_color = Color::Magenta; // Use a different color for aliases
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Alias").fg(header_color),
            Cell::new("Canonical Exercise Name").fg(header_color),
        ]);

    // Sort aliases for consistent output
    let mut sorted_aliases: Vec<_> = aliases.into_iter().collect();
    sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

    for (alias, canonical_name) in sorted_aliases {
        table.add_row(vec![
            Cell::new(alias),
            Cell::new(canonical_name),
        ]);
    }
    println!("{table}");
}


//tests/lib_test.rs
use anyhow::Result;
use chrono::{Utc, Duration, NaiveDate}; // Import NaiveDate
use workout_tracker_lib::{
    AppService, Config, ConfigError, DbError, ExerciseType, Units, 
    WorkoutFilters,
};
use std::thread; // For adding delays in PB tests
use std::time::Duration as StdDuration; // For delays


// Helper function to create a test service with in-memory database
fn create_test_service() -> Result<AppService> {
    // Create an in-memory database for testing
    let conn = rusqlite::Connection::open_in_memory()?;
    workout_tracker_lib::db::init_db(&conn)?;

    // Create a default config for testing
    let config = Config {
        bodyweight: Some(70.0), // Set a default bodyweight for tests
        units: Units::Metric,
        prompt_for_bodyweight: true,
        ..Default::default()
    };

    Ok(AppService {
        config,
        conn,
        db_path: ":memory:".into(),
        config_path: "test_config.toml".into(),
    })
}


#[test]
fn test_create_exercise_unique_name() -> Result<()> {
    let service = create_test_service()?;
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Try creating with same name (case-insensitive)
    let result = service.create_exercise("bench press", ExerciseType::Cardio, None);
    assert!(result.is_err());
    // Check for the specific error type/message if desired
    assert!(result.unwrap_err().to_string().contains("Exercise name must be unique"));

    // Try creating with different name
    let result = service.create_exercise("Squat", ExerciseType::Resistance, Some("legs"));
    assert!(result.is_ok());

    Ok(())
}


#[test]
fn test_exercise_aliases() -> Result<()> {
    let mut service = create_test_service()?;

    let ex_id = service.create_exercise("Barbell Bench Press", ExerciseType::Resistance, Some("chest"))?;
    service.create_exercise("Squat", ExerciseType::Resistance, Some("Legs"))?;

    // 1. Create Alias
    service.create_alias("bp", "Barbell Bench Press")?;

    // 2. List Aliases
    let aliases = service.list_aliases()?;
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases.get("bp").unwrap(), "Barbell Bench Press");

    // 3. Resolve Alias
    let resolved_def = service.resolve_exercise_identifier("bp")?.unwrap();
    assert_eq!(resolved_def.name, "Barbell Bench Press");
    assert_eq!(resolved_def.id, ex_id);

     // 4. Try creating duplicate alias
     let result = service.create_alias("bp", "Squat"); // Different exercise, same alias
     assert!(result.is_err());
     println!("{:?}",result);
     assert!(result.unwrap_err().to_string().contains("Alias already exists"));

     // 5. Try creating alias conflicting with name/id
     let result = service.create_alias("Barbell Bench Press", "Squat"); // Alias conflicts with name
     assert!(result.is_err());
     assert!(result.unwrap_err().to_string().contains("conflicts with an existing exercise name"));

     let result = service.create_alias(&ex_id.to_string(), "Squat"); // Alias conflicts with ID
     assert!(result.is_err());
     assert!(result.unwrap_err().to_string().contains("conflicts with an existing exercise ID"));


    // 6. Use Alias in Add Workout
    let today = Utc::now().date_naive();
    let (workout_id, _) = service.add_workout("bp", today, Some(3), Some(5), Some(100.0), None, None, None, None, None)?;
    let workouts = service.list_workouts(WorkoutFilters{ exercise_name: Some("bp"), ..Default::default() })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].exercise_name, "Barbell Bench Press"); // Stored canonical name


    // 7. Delete Alias
    let deleted_count = service.delete_alias("bp")?;
    assert_eq!(deleted_count, 1);
    let aliases_after_delete = service.list_aliases()?;
    assert!(aliases_after_delete.is_empty());

    // 8. Try deleting non-existent alias
    let result = service.delete_alias("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Alias not found"));

    Ok(())
}

// TODO
// #[test]
// fn test_edit_exercise_with_alias_and_name_change() -> Result<()> {
//     // Need mutable connection for the transaction in db::update_exercise
//     // AppService doesn't hold mut conn, so create one separately
//     let mut service = create_test_service()?; // Setup schema and initial data via service
//     let mut conn = create_mutable_conn_to_test_db()?; // Get a mutable connection

//     service.create_exercise("Old Name", ExerciseType::Resistance, Some("muscle1"))?;
//     service.create_alias("on", "Old Name")?;

//     // Add a workout using alias
//     let today = Utc::now().date_naive();
//     service.add_workout("on", today, Some(1), Some(1), Some(1.0), None, None, None, None, None)?;

//     // Edit using alias, change name
//     // Use the separate mutable connection for the update operation
//     let canonical_name = service.resolve_identifier_to_canonical_name("on")?.unwrap();
//     workout_tracker_lib::db::update_exercise(
//         &mut conn, // Pass the mutable connection
//         &canonical_name,
//         Some("New Name"),
//         None,
//         Some(Some("muscle1,muscle2")),
//     )?;

//     // Verify changes using service (which uses its own immutable connection)
//     // Check old alias points to new name (DB function handles this)
//     let aliases = service.list_aliases()?;
//     assert_eq!(aliases.get("on").unwrap(), "New Name");

//     // Check definition update
//     let new_def = service.resolve_exercise_identifier("on")?.unwrap();
//     assert_eq!(new_def.name, "New Name");
//     assert_eq!(new_def.muscles, Some("muscle1,muscle2".to_string()));

//     // Check workout entry was updated
//     let workouts = service.list_workouts(WorkoutFilters { exercise_name: Some("on"), ..Default::default() })?;
//     assert_eq!(workouts[0].exercise_name, "New Name");

//     Ok(())
// }

#[test]
fn test_delete_exercise_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("To Delete", ExerciseType::Cardio, None)?;
    service.create_alias("td", "To Delete")?;

    // Delete exercise using alias
    let result = service.delete_exercise("td")?;
    assert_eq!(result, 1);

    // Verify exercise is gone
    assert!(service.resolve_exercise_identifier("To Delete")?.is_none());
    assert!(service.resolve_exercise_identifier("td")?.is_none());

    // Verify alias is gone
    let aliases = service.list_aliases()?;
    assert!(aliases.is_empty());

    Ok(())
}

#[test]
fn test_add_workout_past_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Rowing", ExerciseType::Cardio, None)?;

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let two_days_ago = Utc::now().date_naive() - Duration::days(2);

    service.add_workout("Rowing", yesterday, None, None, None, Some(30), None, None, None, None)?;
    service.add_workout("Rowing", two_days_ago, None, None, None, Some(25), None, None, None, None)?;

    // List for yesterday
    let workouts_yesterday = service.list_workouts(WorkoutFilters{ date: Some(yesterday), ..Default::default() })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].duration_minutes, Some(30));
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    // List for two days ago
    let workouts_two_days_ago = service.list_workouts(WorkoutFilters{ date: Some(two_days_ago), ..Default::default() })?;
    assert_eq!(workouts_two_days_ago.len(), 1);
    assert_eq!(workouts_two_days_ago[0].duration_minutes, Some(25));
    assert_eq!(workouts_two_days_ago[0].timestamp.date_naive(), two_days_ago);


    Ok(())
}

#[test]
fn test_edit_workout_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Push-ups", ExerciseType::BodyWeight, None)?;
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    let (workout_id, _) = service.add_workout("Push-ups", today, Some(3), Some(15), None, None, None, None, None, Some(70.0))?;

    // Edit the date
    service.edit_workout(workout_id, None, None, None, None, None, None, Some(yesterday))?;

    // Verify date change by listing
    let workouts_today = service.list_workouts(WorkoutFilters{ date: Some(today), ..Default::default() })?;
    assert!(workouts_today.is_empty());

    let workouts_yesterday = service.list_workouts(WorkoutFilters{ date: Some(yesterday), ..Default::default() })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].id, workout_id);
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);


    Ok(())
}


// TODO
// #[test]
// fn test_pb_detection() -> Result<()> {
//     let mut service = create_test_service()?;
//     service.create_exercise("Deadlift", ExerciseType::Resistance, Some("back,legs"))?;
//     let today = Utc::now().date_naive();

//     // Workout 1: Establish baseline
//     let (_, pb1) = service.add_workout("Deadlift", today, Some(1), Some(5), Some(100.0), None, None, None, None, None)?;
//     assert!(pb1.is_none(), "First workout shouldn't be a PB"); // PB relative to previous entries

//     thread::sleep(StdDuration::from_millis(10)); // Ensure timestamp differs slightly

//     // Workout 2: Higher weight PB
//     let (_, pb2) = service.add_workout("Deadlift", today, Some(1), Some(3), Some(110.0), None, None, None, None, None)?;
//     assert!(pb2.is_some(), "Should detect weight PB");
//     assert_eq!(pb2, Some(PBInfo { pb_type: PBType::Weight, new_weight: Some(110.0), previous_weight: Some(100.0), new_reps: None, previous_reps: None }));

//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 3: Higher reps PB (at lower weight)
//     let (_, pb3) = service.add_workout("Deadlift", today, Some(3), Some(6), Some(90.0), None, None, None, None, None)?;
//     assert!(pb3.is_some(), "Should detect reps PB");
//     assert_eq!(pb3, Some(PBInfo { pb_type: PBType::Reps, new_weight: None, previous_weight: None, new_reps: Some(6), previous_reps: Some(5) })); // Max reps was 5 previously

//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 4: Both weight and reps PB
//     let (_, pb4) = service.add_workout("Deadlift", today, Some(1), Some(7), Some(120.0), None, None, None, None, None)?;
//     assert!(pb4.is_some(), "Should detect both PB");
//      assert_eq!(pb4, Some(PBInfo { pb_type: PBType::Both, new_weight: Some(120.0), previous_weight: Some(110.0), new_reps: Some(7), previous_reps: Some(6) }));

//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 5: No PB (lower weight and reps than max)
//     let (_, pb5) = service.add_workout("Deadlift", today, Some(5), Some(5), Some(105.0), None, None, None, None, None)?;
//     assert!(pb5.is_none(), "Should not detect PB");

//     Ok(())
// }



#[test]
fn test_create_and_list_exercises() -> Result<()> {
    let service = create_test_service()?;

    // Create some exercises
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest,triceps"))?;
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back,biceps"))?;

    // List all exercises
    let exercises = service.list_exercises(None, None)?;
    assert_eq!(exercises.len(), 3);

    // Filter by type
    let resistance_exercises = service.list_exercises(Some(ExerciseType::Resistance), None)?;
    assert_eq!(resistance_exercises.len(), 1);
    assert_eq!(resistance_exercises[0].name, "Bench Press");

    // Filter by muscle
    let leg_exercises = service.list_exercises(None, Some("legs"))?;
    assert_eq!(leg_exercises.len(), 1);
    assert_eq!(leg_exercises[0].name, "Running");

    Ok(())
}

#[test]
fn test_pb_config_interaction() -> Result<()> {
    let mut service = create_test_service()?; // PB notifications default to Some(true) here
    service.set_pb_notification(true)?;

    // Check initial state
    assert_eq!(service.check_pb_notification_config()?, true);

    // Disable PB notifications
    service.set_pb_notification(false)?;
    assert_eq!(service.config.notify_on_pb, Some(false));
    assert_eq!(service.check_pb_notification_config()?, false);

     // Re-enable PB notifications
     service.set_pb_notification(true)?;
     assert_eq!(service.config.notify_on_pb, Some(true));
     assert_eq!(service.check_pb_notification_config()?, true);

    // Test case where config starts as None (simulate first run)
    service.config.notify_on_pb = None;
    let result = service.check_pb_notification_config();
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::PbNotificationNotSet => {}, // Correct error
        _ => panic!("Expected PbNotificationNotSet error"),
    }


    Ok(())
}

// Test list filtering with aliases
#[test]
fn test_list_filter_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Overhead Press", ExerciseType::Resistance, Some("shoulders"))?;
    service.create_alias("ohp", "Overhead Press")?;
    let today = Utc::now().date_naive();

    service.add_workout("ohp", today, Some(5), Some(5), Some(50.0), None, None, None, None, None)?;

    // Filter list using alias
    let workouts = service.list_workouts(WorkoutFilters { exercise_name: Some("ohp"), ..Default::default() })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].exercise_name, "Overhead Press");

    // Filter list using canonical name
    let workouts2 = service.list_workouts(WorkoutFilters { exercise_name: Some("Overhead Press"), ..Default::default() })?;
    assert_eq!(workouts2.len(), 1);
    assert_eq!(workouts2[0].exercise_name, "Overhead Press");

    Ok(())
}


#[test]
fn test_add_and_list_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise first
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Add some workouts
    service.add_workout(
        "Bench Press",
        NaiveDate::from_ymd_opt(2015, 6, 2).unwrap(),
        Some(3),
        Some(10),
        Some(60.0),
        None,
        Some("First workout".to_string()),
        None,
        None,
        None,
    )?;

    service.add_workout(
        "Bench Press",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        Some(4),
        Some(8),
        Some(70.0),
        None,
        Some("Second workout".to_string()),
        None,
        None,
        None,
    )?;

    // List workouts
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Bench Press"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 2);
    assert_eq!(workouts[0].sets, Some(4)); // Most recent first
    assert_eq!(workouts[1].sets, Some(3));

    Ok(())
}

#[test]
fn test_bodyweight_workouts() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.bodyweight = Some(70.0); // Set bodyweight

    // Create a bodyweight exercise
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?;

    // Add workout with additional weight
    service.add_workout(
        "Pull-ups",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        Some(3),
        Some(10),
        Some(5.0), // Additional weight
        None,
        None,
        None,
        None,
        Some(70.0), // Pass bodyweight
    )?;

    // Check that weight was calculated correctly
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].weight, Some(75.0)); // 70 + 5

    Ok(())
}

#[test]
fn test_edit_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Edit the exercise
    service.edit_exercise(
        "Bench Press",
        Some("Barbell Bench Press"),
        Some(ExerciseType::Resistance),
        Some(Some("chest,triceps,shoulders")),
    )?;

    // Verify changes
    let exercise = service
        .get_exercise_by_identifier_service("Barbell Bench Press")?
        .unwrap();
    assert_eq!(exercise.name, "Barbell Bench Press");
    assert_eq!(exercise.muscles, Some("chest,triceps,shoulders".to_string()));

    Ok(())
}

#[test]
fn test_delete_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Delete it
    let result = service.delete_exercise("Bench Press")?;
    assert_eq!(result, 1);

    // Verify it's gone
    let exercise = service.get_exercise_by_identifier_service("Bench Press")?;
    assert!(exercise.is_none());

    Ok(())
}

#[test]
fn test_workout_filters() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Curl", ExerciseType::Resistance, Some("Biceps"))?;

    // Add workouts on different dates
    // Hack: We can't set the timestamp directly, so we'll add with a small delay
    service.add_workout(
        "Running",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        None,
        None,
        None,
        Some(30),
        None,
        None,
        None,
        None,
    )?;

    // Add another workout
    service.add_workout(
        "Curl",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        None,
        None,
        None,
        Some(45),
        None,
        None,
        None,
        None,
    )?;

    // Filter by type
    let resistance_workout = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Resistance),
        ..Default::default()
    })?;
    assert_eq!(resistance_workout.len(), 1);
    assert_eq!(resistance_workout[0].duration_minutes, Some(45));


    Ok(())
}

#[test]
fn test_nth_last_day_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Add workouts on different dates
    // First workout (older)
    service.add_workout(
        "Squats",
        NaiveDate::from_ymd_opt(2015, 6, 2).unwrap(),
        Some(3),
        Some(10),
        Some(100.0),
        None,
        Some("First workout".to_string()),
        None,
        None,
        None,
    )?;

    // Second workout (more recent)
    service.add_workout(
        "Squats",
        NaiveDate::from_ymd_opt(2015, 6, 7).unwrap(),
        Some(5),
        Some(5),
        Some(120.0),
        None,
        Some("Second workout".to_string()),
        None,
        None,
        None,
    )?;

    // Get workouts for the most recent day (n=1)
    let recent_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 1)?;
    assert_eq!(recent_workouts.len(), 1);
    assert_eq!(recent_workouts[0].sets, Some(5));

    // Get workouts for the previous day (n=2)
    let previous_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 2)?;
    assert_eq!(previous_workouts.len(), 1);
    assert_eq!(previous_workouts[0].sets, Some(3));

    Ok(())
}

#[test]
fn test_config_operations() -> Result<()> {
    let mut service = create_test_service()?;

    // Test setting bodyweight
    service.set_bodyweight(75.5)?;
    assert_eq!(service.config.bodyweight, Some(75.5));

    // Test getting required bodyweight
    let bw = service.get_required_bodyweight()?;
    assert_eq!(bw, 75.5);

    // Test disabling prompt
    service.disable_bodyweight_prompt()?;
    assert!(!service.config.prompt_for_bodyweight);

    Ok(())
}

#[test]
fn test_exercise_not_found() -> Result<()> {
    let mut service = create_test_service()?;

    // Try to get non-existent exercise
    let result = service.get_exercise_by_identifier_service("Non-existent");
    assert!(result.is_ok()); // Should return Ok(None)
    assert!(result?.is_none());

    // Try to edit non-existent exercise
    let result = service.edit_exercise("Non-existent", None, None, None);
    assert!(result.is_err());
    match result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::ExerciseNotFound(_)) => (),
        _ => panic!("Expected ExerciseNotFound error"),
    }

    Ok(())
}

#[test]
fn test_workout_not_found() -> Result<()> {
    let service = create_test_service()?;

    // Try to edit non-existent workout
    let result = service.edit_workout(999, None, None, None, None, None, None, None);
    println!("testing {:?}", result);
    assert!(result.is_err());

    // Try to delete non-existent workout
    let result = service.delete_workout(999);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_bodyweight_validation() -> Result<()> {
    let mut service = create_test_service()?;

    // Test invalid bodyweight
    let result = service.set_bodyweight(0.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    let result = service.set_bodyweight(-10.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    Ok(())
}

