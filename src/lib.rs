// src/lib.rs
use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};


// --- Expose public types ---
pub use config::{
    Config, ConfigError, ThemeConfig, Units, StandardColor, parse_color,
    get_config_path as get_config_path_util, // Rename utility function
    load_config as load_config_util,         // Rename utility function
    save_config as save_config_util,         // Rename utility function
};
pub use db::{
    DbError, ExerciseDefinition, ExerciseType, Workout, WorkoutFilters,
    get_db_path as get_db_path_util, // Rename utility function
};


// --- Service Layer ---

/// Main application service holding configuration and database connection.
pub struct AppService {
    pub config: Config, // Public for reading by UI layers (CLI, TUI)
    pub conn: Connection,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

impl AppService {
    /// Initializes the application service by loading config and connecting to the DB.
    pub fn initialize() -> Result<Self> {
        let config_path = config::get_config_path()
            .context("Failed to determine configuration file path")?;
        let config = config::load_config() // load_config now returns mutable config
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
             // Use ConfigError or anyhow::bail! here? Let's use ConfigError for consistency
             return Err(ConfigError::InvalidBodyweightInput("Weight must be a positive number.".to_string()));
         }
         self.config.bodyweight = Some(weight);
         // Decide if setting manually should re-enable prompt. Let's keep current behavior.
         // self.config.prompt_for_bodyweight = true;
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

    // --- Database Path ---
    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }

    // --- Exercise Definition Methods ---

    /// Creates a new exercise definition.
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
        db::create_exercise(&self.conn, trimmed_name, &type_, muscles)
            .context(format!("Failed to create exercise '{}' in database", trimmed_name))
    }

    /// Edits an existing exercise definition.
    pub fn edit_exercise(
        &mut self, // Takes mut self because it uses a transaction via db::update_exercise
        identifier: &str,
        new_name: Option<&str>,
        new_type: Option<ExerciseType>,
        new_muscles: Option<Option<&str>>, // None = don't change, Some(None) = clear, Some(Some("val")) = set
    ) -> Result<u64> {
        let trimmed_identifier = identifier.trim();
        if trimmed_identifier.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        db::update_exercise(
            &mut self.conn, // Pass mutable reference
            trimmed_identifier,
            new_name,
            new_type.as_ref(), // Pass Option<&DbType>
            new_muscles,
        )
        .context(format!("Failed to update exercise '{}' in database", trimmed_identifier))
    }

    /// Deletes an exercise definition. Returns number of definitions deleted (0 or 1).
    /// Includes warnings about associated workouts.
    pub fn delete_exercise(&self, identifier: &str) -> Result<u64> {
        let trimmed_identifier = identifier.trim();
        if trimmed_identifier.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        // Fetch definition first to print warnings based on its name
        let exercise = self.get_exercise_by_identifier(trimmed_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(trimmed_identifier.to_string()))?;

        // Check for associated workouts (using canonical name)
        let workout_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE",
            [&exercise.name], // Use canonical name
            |row| row.get(0),
        ).context("Failed to check for associated workouts")?;

        if workout_count > 0 {
             // How should the library signal this? Return a struct? Print warning?
             // For now, let's return Ok but maybe include a warning string or struct later.
             // We print the warning here for now, although ideally the caller (CLI/TUI) would format it.
             eprintln!(
                "Warning: {} workout entries reference the exercise '{}'.",
                workout_count, exercise.name
             );
             eprintln!("These entries will remain but will reference a now-deleted exercise definition.");
        }


        db::delete_exercise(&self.conn, trimmed_identifier) // delete_exercise internally finds by ID/name again
            .context(format!("Failed to delete exercise '{}' from database", trimmed_identifier))
    }

    /// Retrieves an exercise definition by ID or name.
    pub fn get_exercise_by_identifier(&self, identifier: &str) -> Result<Option<ExerciseDefinition>> {
        db::get_exercise_by_identifier(&self.conn, identifier)
            .context(format!("Failed to query exercise by identifier '{}'", identifier))
    }

    /// Lists exercise definitions based on filters.
    pub fn list_exercises(
        &self,
        type_filter: Option<ExerciseType>,
        muscle_filter: Option<&str>,
    ) -> Result<Vec<ExerciseDefinition>> {
        db::list_exercises(&self.conn, type_filter, muscle_filter)
            .context("Failed to list exercise definitions from database")
    }

    // --- Workout Entry Methods ---

    /// Adds a workout entry. Handles implicit exercise creation and bodyweight logic.
    pub fn add_workout(
        &mut self, // Needs mut to potentially save config if bodyweight prompt was used by caller
        exercise_identifier: &str,
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
    ) -> Result<i64> {
        let identifier_trimmed = exercise_identifier.trim();
        if identifier_trimmed.is_empty() {
            bail!("Exercise identifier cannot be empty for adding a workout.");
        }

        // 1. Find or implicitly create the Exercise Definition
        let mut exercise_def = self.get_exercise_by_identifier(identifier_trimmed)?;

        if exercise_def.is_none() {
            if let (Some(db_type), Some(muscle_list)) = (implicit_type, implicit_muscles) {
                println!( // Keep CLI print for now, could return info struct later
                    "Exercise '{}' not found, defining it implicitly...",
                    identifier_trimmed
                );
                let muscles_opt = if muscle_list.trim().is_empty() {
                    None
                } else {
                    Some(muscle_list.as_str())
                };
                match self.create_exercise(identifier_trimmed, db_type, muscles_opt) {
                    Ok(id) => {
                         println!("Implicitly defined exercise: '{}' (ID: {})", identifier_trimmed, id);
                         // Refetch the newly created definition
                         exercise_def = Some(self.get_exercise_by_identifier(identifier_trimmed)?
                             .ok_or_else(|| anyhow::anyhow!("Failed to re-fetch implicitly created exercise '{}'", identifier_trimmed))?);
                    }
                    Err(e) => {
                         // If implicit creation fails, propagate the error
                         return Err(e).context(format!("Failed to implicitly define exercise '{}'", identifier_trimmed));
                    }
                }
            } else {
                // Not found and no implicit creation info provided
                bail!(
                    "Exercise '{}' not found. Define it first using 'create-exercise' or provide details for implicit creation.",
                    identifier_trimmed
                );
            }
        }

        let current_exercise_def = exercise_def.unwrap(); // Safe to unwrap here

        // 2. Determine final weight based on type and provided bodyweight
        let final_weight = if current_exercise_def.type_ == ExerciseType::BodyWeight {
            match bodyweight_to_use {
                 Some(bw) => Some(bw + weight_arg.unwrap_or(0.0)),
                 None => {
                     // This case should ideally be caught by the caller checking BodyweightNotSet
                     // before calling add_workout. If we reach here, it's an issue.
                     bail!("Bodyweight is required for exercise '{}' but was not provided.", current_exercise_def.name);
                 }
            }
        } else {
             weight_arg // Use the provided weight directly for non-bodyweight exercises
        };


        // 3. Add the workout entry using the canonical exercise name and final weight
        let inserted_id = db::add_workout(
            &self.conn,
            &current_exercise_def.name, // Use canonical name
            sets,
            reps,
            final_weight, // Use calculated weight
            duration,
            notes,
        )
        .context("Failed to add workout to database")?;

        Ok(inserted_id)
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
    ) -> Result<u64> {
        // Resolve the new exercise identifier to its canonical name if provided
        let new_canonical_name: Option<String> =
            if let Some(ident) = new_exercise_identifier.as_deref() {
                let trimmed_ident = ident.trim();
                if trimmed_ident.is_empty() {
                    bail!("New exercise identifier cannot be empty string if provided.");
                }
                let maybe_def = self.get_exercise_by_identifier(trimmed_ident)?;
                match maybe_def {
                    Some(def) => Some(def.name), // Use canonical name
                    None => bail!("New exercise '{}' not found.", trimmed_ident),
                }
            } else {
                None
            };
        
        db::update_workout(
            &self.conn,
            id,
            new_canonical_name.as_deref(), // Pass Option<&str>
            new_sets,
            new_reps,
            new_weight, // Pass Option<f64> directly
            new_duration,
            new_notes.as_deref(), // Pass Option<&str>
        )
        .with_context(|| format!("Failed to update workout ID {}", id))
    }


    /// Deletes a workout entry by ID.
    pub fn delete_workout(&self, id: i64) -> Result<u64> {
        db::delete_workout(&self.conn, id)
            .context(format!("Failed to delete workout ID {}", id))
    }

    /// Lists workouts based on filters.
    pub fn list_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>> {
        db::list_workouts_filtered(&self.conn, filters)
            .context("Failed to list workouts from database")
    }

     /// Lists workouts for the Nth most recent day a specific exercise was performed.
     pub fn list_workouts_for_exercise_on_nth_last_day(
         &self,
         exercise_name: &str,
         n: u32,
     ) -> Result<Vec<Workout>> {
         db::list_workouts_for_exercise_on_nth_last_day(&self.conn, exercise_name, n)
             .with_context(|| format!("Failed to list workouts for exercise '{}' on nth last day {}", exercise_name, n))
     }

}

// --- Re-export nested modules if needed (or keep them private) ---
// If db or config needed direct access from outside the lib (unlikely now with AppService)
// pub mod db;
// pub mod config;

// --- Move internal implementations of db.rs and config.rs here ---
mod config {
    // All content from the original src/config.rs
    // Adjust imports if needed (e.g., crate:: instead of super::)
    // Make sure functions like get_config_path, load_config, save_config are defined here
    // (even if also exposed at the crate root with different names)

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
        #[error("Bodyweight input cancelled by user.")] // Keep for potential interactive use
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

    /// Loads the configuration from the TOML file.
    pub fn load_config() -> Result<Config, ConfigError> {
        let config_path = get_config_path()?;

        if !config_path.exists() {
             // Don't print here, let caller decide how to inform user
             // println!( "Configuration file not found at {:?}. Creating a default one.", config_path );
             let default_config = Config::new_default();
             save_config(&config_path, &default_config)?;
             Ok(default_config)
        } else {
             let config_content = fs::read_to_string(&config_path)?;
             let config: Config = toml::from_str(&config_content)?;
             Ok(config)
        }
    }

    /// Saves the configuration to the TOML file.
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
}

pub mod db {
    // All content from the original src/db.rs
    // Adjust imports if needed (e.g., crate:: instead of super::)
    // Ensure functions like get_db_path, open_db, init_db are defined here
    // (even if also exposed at the crate root with different names)

    // src/db.rs
    use anyhow::{Context, Result};
    use chrono::{DateTime, NaiveDate, Utc};
    use rusqlite::{params, Connection, OptionalExtension, Row, ToSql};
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
        pub exercise_name: String,
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
    }

    const DB_FILE_NAME: &str = "workouts.sqlite";

    /// Gets the path to the SQLite database file within the app's data directory.
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
        Connection::open(path).map_err(DbError::Connection)
    }

    /// Initializes the database tables if they don't exist.
    pub fn init_db(conn: &Connection) -> Result<(), DbError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workouts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                exercise_name TEXT NOT NULL COLLATE NOCASE,
                sets INTEGER, reps INTEGER, weight REAL, duration_minutes INTEGER, notes TEXT
            )",
            [],
        ).map_err(DbError::Connection)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS exercises (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE,
                type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio', 'body-weight')),
                muscles TEXT
            )",
            [],
        ).map_err(DbError::Connection)?;
        Ok(())
    }

    /// Adds a new workout entry to the database.
    pub fn add_workout(
        conn: &Connection,
        exercise_name: &str,
        sets: Option<i64>, reps: Option<i64>, weight: Option<f64>,
        duration: Option<i64>, notes: Option<String>,
    ) -> Result<i64, DbError> { // Return DbError
        let timestamp = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![timestamp, exercise_name, sets, reps, weight, duration, notes],
        ).map_err(DbError::InsertFailed)?;
        Ok(conn.last_insert_rowid())
    }

    /// Updates an existing workout entry in the database by its ID.
    pub fn update_workout(
        conn: &Connection,
        id: i64, new_exercise_name: Option<&str>, new_sets: Option<i64>, new_reps: Option<i64>,
        new_weight: Option<f64>, new_duration: Option<i64>, new_notes: Option<&str>,
    ) -> Result<u64, DbError> { // Return DbError
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let mut updates = Vec::new();

        if let Some(ex) = new_exercise_name { updates.push("exercise_name = ?"); params.push(Box::new(ex.to_string())); }
        if let Some(s) = new_sets { updates.push("sets = ?"); params.push(Box::new(s)); }
        if let Some(r) = new_reps { updates.push("reps = ?"); params.push(Box::new(r)); }
        if new_weight.is_some() { updates.push("weight = ?"); params.push(Box::new(new_weight)); }
        if new_duration.is_some() { updates.push("duration_minutes = ?"); params.push(Box::new(new_duration)); }
        if new_notes.is_some() { updates.push("notes = ?"); params.push(Box::new(new_notes)); }

        if updates.is_empty() {
            // This should ideally be an error type, not panic/bail from lib
            // For now, returning Ok(0) might be acceptable, or a specific DbError variant.
             // Let's return an error:
            return Err(DbError::UpdateFailed(rusqlite::Error::ExecuteReturnedResults)); // Re-use an error or make a new one
            // Or maybe just Ok(0) is fine and caller checks. Let's stick with Ok(0).
            // return Ok(0);
        }

        let sql = format!("UPDATE workouts SET {} WHERE id = ?", updates.join(", "));
        params.push(Box::new(id));
        let params_slice: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();

        let rows_affected = conn.execute(&sql, params_slice.as_slice()).map_err(DbError::UpdateFailed)?;
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
        let exercise_name: String = row.get(2)?;
        let sets: Option<i64> = row.get(3)?;
        let reps: Option<i64> = row.get(4)?;
        let weight: Option<f64> = row.get(5)?;
        let duration_minutes: Option<i64> = row.get(6)?;
        let notes: Option<String> = row.get(7)?;
        let type_str_opt: Option<String> = row.get(8)?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;

        let exercise_type = match type_str_opt {
            Some(type_str) => match ExerciseType::try_from(type_str.as_str()) {
                Ok(et) => Some(et),
                Err(_) => { /* eprintln warning removed from lib */ None }
            },
            None => None,
        };

        Ok(Workout { id, timestamp, exercise_name, sets, reps, weight, duration_minutes, notes, exercise_type })
    }

    #[derive(Default, Debug)]
    pub struct WorkoutFilters<'a> {
        pub exercise_name: Option<&'a str>,
        pub date: Option<NaiveDate>,
        pub exercise_type: Option<ExerciseType>,
        pub muscle: Option<&'a str>,
        pub limit: Option<u32>,
    }

    /// Lists workout entries from the database based on various filters.
    pub fn list_workouts_filtered(conn: &Connection, filters: WorkoutFilters) -> Result<Vec<Workout>, DbError> { // Return DbError
        let mut sql = "SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
                       FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(name) = filters.exercise_name { sql.push_str(&format!(" AND w.exercise_name = ?{}", params_vec.len() + 1)); params_vec.push(Box::new(name.to_string())); }
        if let Some(date) = filters.date { sql.push_str(&format!(" AND date(w.timestamp) = date(?{})", params_vec.len() + 1)); params_vec.push(Box::new(date.format("%Y-%m-%d").to_string())); }
        if let Some(ex_type) = filters.exercise_type { sql.push_str(&format!(" AND e.type = ?{}", params_vec.len() + 1)); params_vec.push(Box::new(ex_type.to_string())); }
        if let Some(m) = filters.muscle { sql.push_str(&format!(" AND e.muscles LIKE ?{}", params_vec.len() + 1)); params_vec.push(Box::new(format!("%{}%", m))); }

        if filters.date.is_some() { sql.push_str(" ORDER BY w.timestamp ASC"); }
        else { sql.push_str(" ORDER BY w.timestamp DESC"); }

        if filters.date.is_none() {
            if let Some(limit) = filters.limit { sql.push_str(&format!(" LIMIT ?{}", params_vec.len() + 1)); params_vec.push(Box::new(limit)); }
        }

        let params_slice: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
        let workout_iter = stmt.query_map(params_slice.as_slice(), map_row_to_workout).map_err(DbError::QueryFailed)?;

        workout_iter.collect::<Result<Vec<_>, _>>().map_err(DbError::QueryFailed) // Collect results
    }

     /// Lists workouts for a specific exercise performed on the Nth most recent day it was done.
     pub fn list_workouts_for_exercise_on_nth_last_day(
         conn: &Connection, exercise_name: &str, n: u32,
     ) -> Result<Vec<Workout>, anyhow::Error> { // Keep anyhow::Error here or use DbError?
         if n == 0 { anyhow::bail!("Nth last day (N) must be 1 or greater."); } // Bail seems ok for invalid input
         let offset = n - 1;
         let sql = "WITH RankedDays AS (SELECT DISTINCT date(timestamp) as workout_date FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE ORDER BY workout_date DESC LIMIT 1 OFFSET ?2)
                    SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
                    FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
                    WHERE w.exercise_name = ?1 COLLATE NOCASE ORDER BY w.timestamp ASC;";

         let mut stmt = conn.prepare(sql).map_err(DbError::QueryFailed)?;
         let workout_iter = stmt.query_map(params![exercise_name, offset], map_row_to_workout)
                                .map_err(DbError::QueryFailed)?;

         workout_iter.collect::<Result<Vec<_>, _>>()
                     .map_err(|e| DbError::QueryFailed(e).into()) // Map rusqlite error to anyhow::Error via DbError
     }

    // ---- Exercise Definition Functions ----

    /// Creates a new exercise definition. Returns ID. Handles UNIQUE constraint.
    pub fn create_exercise(
        conn: &Connection, name: &str, type_: &ExerciseType, muscles: Option<&str>,
    ) -> Result<i64, anyhow::Error> { // Return anyhow::Error to allow custom message
        let type_str = type_.to_string();
        match conn.execute("INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)", params![name, type_str, muscles]) {
            Ok(_) => Ok(conn.last_insert_rowid()),
            Err(e) => {
                if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                    if err.code == rusqlite::ErrorCode::ConstraintViolation {
                        // Check if it really exists (case-insensitive)
                        if get_exercise_by_name(conn, name)?.is_some() {
                            return Err(anyhow::anyhow!("Exercise '{}' already exists (case-insensitive).", name));
                        }
                    }
                }
                Err(DbError::InsertFailed(e).into()) // Wrap other errors
            }
        }
    }

    /// Updates an existing exercise definition (found by ID or name). Requires mutable conn for transaction.
    pub fn update_exercise(
        conn: &mut Connection, // Use mutable connection for transaction
        identifier: &str, new_name: Option<&str>, new_type: Option<&ExerciseType>,
        new_muscles: Option<Option<&str>>,
    ) -> Result<u64, DbError> { // Return DbError
        let exercise = get_exercise_by_identifier(conn, identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let id = exercise.id;
        let original_name = exercise.name;
        let name_being_changed = new_name.is_some() && new_name != Some(original_name.as_str());
        let target_new_name = new_name.unwrap_or(&original_name);

        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let mut updates = Vec::new();

        if let Some(name) = new_name { updates.push("name = ?"); params.push(Box::new(name.to_string())); }
        if let Some(t) = new_type { updates.push("type = ?"); params.push(Box::new(t.to_string())); }
        if let Some(m_opt) = new_muscles { updates.push("muscles = ?"); params.push(Box::new(m_opt)); }

        if updates.is_empty() {
            // Return Ok(0) or error? Let's return Ok(0).
            return Ok(0);
            // Could also be: Err(DbError::UpdateFailed(rusqlite::Error::QueryReturnedNoRows)) // Or a custom error
        }

        // Use a transaction
        let tx = conn.transaction().map_err(DbError::Connection)?;

        // 1. Update exercises table
        let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = ?", updates.join(", "));
        params.push(Box::new(id));
        let params_slice_update: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows_affected = tx.execute(&sql_update_exercise, params_slice_update.as_slice())
                              .map_err(DbError::UpdateFailed)?;

        // 2. Update workouts table if name changed
        if name_being_changed {
            // eprintln removed from lib
            tx.execute("UPDATE workouts SET exercise_name = ?1 WHERE exercise_name = ?2", params![target_new_name, original_name])
              .map_err(DbError::UpdateFailed)?;
            // eprintln removed from lib
        }

        tx.commit().map_err(DbError::Connection)?; // Commit transaction

        if rows_affected == 0 { Err(DbError::ExerciseNotFound(identifier.to_string())) } // Should have been caught earlier
        else { Ok(rows_affected as u64) }
    }


     /// Deletes an exercise definition (found by ID or name).
     /// Note: Warning about associated workouts is now handled in the AppService layer.
    pub fn delete_exercise(conn: &Connection, identifier: &str) -> Result<u64, DbError> { // Return DbError
        let exercise = get_exercise_by_identifier(conn, identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let id = exercise.id;

        // Proceed with deletion from exercises table
        let rows_affected = conn.execute("DELETE FROM exercises WHERE id = ?", params![id])
            .map_err(DbError::DeleteFailed)?;

        if rows_affected == 0 { Err(DbError::ExerciseNotFound(identifier.to_string())) }
        else { Ok(rows_affected as u64) }
    }


    fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
         let id: i64 = row.get(0)?;
         let name: String = row.get(1)?;
         let type_str: String = row.get(2)?;
         let muscles: Option<String> = row.get(3)?;
         let ex_type = ExerciseType::try_from(type_str.as_str()).map_err(|e| {
            // eprintln removed from lib
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

    /// Retrieves an exercise definition by trying ID first, then name.
    pub fn get_exercise_by_identifier(conn: &Connection, identifier: &str) -> Result<Option<ExerciseDefinition>, DbError> { // Return DbError
        if let Ok(id) = identifier.parse::<i64>() {
            match get_exercise_by_id(conn, id)? {
                Some(exercise) => Ok(Some(exercise)),
                None => Ok(None), // Parsed as ID but not found, don't try as name
            }
        } else {
            get_exercise_by_name(conn, identifier) // Treat as name
        }
    }

    /// Lists defined exercises, optionally filtering by type and/or muscle.
    pub fn list_exercises(
        conn: &Connection, type_filter: Option<ExerciseType>, muscle_filter: Option<&str>,
    ) -> Result<Vec<ExerciseDefinition>, DbError> { // Return DbError
        let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(t) = type_filter { sql.push_str(&format!(" AND type = ?{}", params_vec.len() + 1)); params_vec.push(Box::new(t.to_string())); }
        if let Some(m) = muscle_filter { sql.push_str(&format!(" AND muscles LIKE ?{}", params_vec.len() + 1)); params_vec.push(Box::new(format!("%{}%", m))); }
        sql.push_str(" ORDER BY name ASC");

        let params_slice: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
        let exercise_iter = stmt.query_map(params_slice.as_slice(), map_row_to_exercise_definition).map_err(DbError::QueryFailed)?;

        exercise_iter.collect::<Result<Vec<_>, _>>().map_err(DbError::QueryFailed) // Collect results
    }
}
