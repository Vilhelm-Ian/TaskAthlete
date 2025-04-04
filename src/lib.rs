// src/lib.rs
use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

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
        // Call function from db module
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
        // Call function from db module
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

        // Call function from db module
        db::delete_exercise(&self.conn, trimmed_identifier) // delete_exercise internally finds by ID/name again
            .context(format!("Failed to delete exercise '{}' from database", trimmed_identifier))
    }

    /// Retrieves an exercise definition by ID or name.
    pub fn get_exercise_by_identifier(&self, identifier: &str) -> Result<Option<ExerciseDefinition>> {
        // Call function from db module
        db::get_exercise_by_identifier(&self.conn, identifier)
            .context(format!("Failed to query exercise by identifier '{}'", identifier))
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
        // Call function from db module
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
        )
        .with_context(|| format!("Failed to update workout ID {}", id))
    }


    /// Deletes a workout entry by ID.
    pub fn delete_workout(&self, id: i64) -> Result<u64> {
        // Call function from db module
        db::delete_workout(&self.conn, id)
            .context(format!("Failed to delete workout ID {}", id))
    }

    /// Lists workouts based on filters.
    pub fn list_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>> {
        // Call function from db module
        db::list_workouts_filtered(&self.conn, filters)
            .context("Failed to list workouts from database")
    }

     /// Lists workouts for the Nth most recent day a specific exercise was performed.
     pub fn list_workouts_for_exercise_on_nth_last_day(
         &self,
         exercise_name: &str,
         n: u32,
     ) -> Result<Vec<Workout>> {
         // Call function from db module
         db::list_workouts_for_exercise_on_nth_last_day(&self.conn, exercise_name, n)
             .with_context(|| format!("Failed to list workouts for exercise '{}' on nth last day {}", exercise_name, n))
     }

}
