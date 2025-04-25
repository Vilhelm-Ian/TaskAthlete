use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use db::list_workouts_filtered;
use std::collections::BTreeMap;
// Add Duration, TimeZone
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf}; // For list_aliases return type

// --- Declare modules to load from separate files ---
mod config;
pub mod db;

// --- Expose public types ---
pub use config::{
    get_config_path as get_config_path_util, // Rename utility function
    load_config as load_config_util,         // Rename utility function
    parse_color,                             // PbMetricScope removed
    save_config as save_config_util,         // Rename utility function
    Config,
    Error as ConfigError,
    StandardColor,
    Theme,
    Units,
};
pub use db::{
    get_db_path as get_db_path_util, // Rename utility function
    DbError,
    ExerciseDefinition,
    ExerciseType,
    ResolvedByType,
    VolumeFilters,
    Workout,
    WorkoutFilters,
};


const KM_TO_MILE: f64 = 0.621_371;
const MILE_TO_KM: f64 = 1.60934;
// Helper struct to hold previous bests
struct PreviousBests {
    weight: Option<f64>,
    reps: Option<i64>,
    duration: Option<i64>,
    distance_km: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] // Add Hash for potential future use
pub enum GraphType {
    Estimated1RM,
    MaxWeight, // Changed from MaxWeight (ambiguous)
    MaxReps,   // Changed from MaxReps (ambiguous)
    WorkoutVolume,
    WorkoutReps, // Replaced MaxWeightForReps
    WorkoutDuration,
    WorkoutDistance,
    // Add more as needed, e.g., MaxWeightForReps(u32) if clarified
}

#[derive(Debug, Clone, PartialEq, Default)] // Add Default
pub struct PBInfo {
    pub achieved_weight_pb: bool,
    pub new_weight: Option<f64>,
    pub previous_weight: Option<f64>,
    pub achieved_reps_pb: bool,
    pub new_reps: Option<i64>,
    pub previous_reps: Option<i64>,
    pub achieved_duration_pb: bool,
    pub new_duration: Option<i64>,
    pub previous_duration: Option<i64>,
    pub achieved_distance_pb: bool,
    pub new_distance: Option<f64>,      // Stored as km
    pub previous_distance: Option<f64>, // Stored as km
}

impl PBInfo {
    /// Helper to check if any PB was achieved.
    pub fn any_pb(&self) -> bool {
        self.achieved_weight_pb
            || self.achieved_reps_pb
            || self.achieved_duration_pb
            || self.achieved_distance_pb
    }
}

// --- Statistics Types ---

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PersonalBests {
    pub max_weight: Option<f64>,
    pub max_reps: Option<i64>,
    pub max_duration_minutes: Option<i64>,
    pub max_distance_km: Option<f64>, // Always store in km
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseStats {
    pub canonical_name: String,
    pub total_workouts: usize,
    pub first_workout_date: Option<NaiveDate>,
    pub last_workout_date: Option<NaiveDate>,
    pub avg_workouts_per_week: Option<f64>,
    pub longest_gap_days: Option<u64>,
    pub personal_bests: PersonalBests,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub streak_interval_days: u32, // From config
}

// --- Service Layer ---

/// Main application service holding configuration and database connection.
pub struct AppService {
    pub config: Config,   // Public for reading by UI layers (CLI, TUI)
    pub conn: Connection, // Make mutable ONLY if needed (e.g., transactions directly in service) - currently DB funcs handle transactions
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

impl AppService {
    /// Initializes the application service by loading config and connecting to the DB.
    pub fn initialize() -> Result<Self> {
        let config_path =
            config::get_config_path().context("Failed to determine configuration file path")?;
        // Use the load_config function from the config module
        let config = config::load_config(&config_path)
            .context(format!("Failed to load config from {:?}", config_path))?;

        let db_path = db::get_db_path().context("Failed to determine database path")?;
        let mut conn =
            db::open_db(&db_path) // Use mutable conn for init potentially
                .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        db::init_db(&mut conn).context("Failed to initialize database schema")?; // Pass mutable conn

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
            return Err(ConfigError::InvalidBodyweightInput(
                "Weight must be a positive number.".to_string(),
            ));
        }
        self.config.bodyweight = Some(weight);
        self.save_config()?;
        Ok(())
    }

    /// Checks if bodyweight is needed and returns it, or returns error if needed but not set.
    /// Does NOT prompt.
    pub fn get_required_bodyweight(&self) -> Result<f64, ConfigError> {
        self.config
            .bodyweight
            .ok_or_else(|| ConfigError::BodyweightNotSet(self.config_path.clone()))
    }

    /// Disables the bodyweight prompt in the config and saves it.
    pub fn disable_bodyweight_prompt(&mut self) -> Result<(), ConfigError> {
        self.config.prompt_for_bodyweight = false;
        self.save_config()
    }

    /// Sets the streak interval in the config and saves it.
    pub fn set_streak_interval(&mut self, days: u32) -> Result<(), ConfigError> {
        if days == 0 {
            // Although CLI parser prevents 0, add safeguard here
            return Err(ConfigError::InvalidBodyweightInput(
                "Streak interval must be at least 1 day.".to_string(),
            ));
        }
        self.config.streak_interval_days = days;
        self.save_config()
    }

    // --- PB Notification Config Methods ---

    /// Sets the global PB notification preference in the config and saves it.
    pub fn set_pb_notification_enabled(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_enabled = Some(enabled);
        self.save_config()
    }

    /// Checks the global PB notification config setting. Returns error if not set (needs prompt).
    pub fn check_pb_notification_config(&self) -> Result<bool, ConfigError> {
        self.config
            .notify_pb_enabled
            .ok_or(ConfigError::PbNotificationNotSet)
    }

    pub fn set_pb_notify_weight(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_weight = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_reps(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_reps = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_duration(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_duration = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_distance(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_distance = enabled;
        self.save_config()
    }

    // --- Target Bodyweight Config Methods ---

    /// Sets the target bodyweight in the config and saves it.
    pub fn set_target_bodyweight(&mut self, weight: Option<f64>) -> Result<(), ConfigError> {
        if let Some(w) = weight {
            if w <= 0.0 {
                return Err(ConfigError::InvalidBodyweightInput(
                    "Target weight must be a positive number.".to_string(),
                ));
            }
        }
        self.config.target_bodyweight = weight;
        self.save_config()
    }

    /// Gets the target bodyweight from the config.
    pub const fn get_target_bodyweight(&self) -> Option<f64> {
        self.config.target_bodyweight
    }

    // --- Units Config ---
    pub fn set_units(&mut self, units: Units) -> Result<(), ConfigError> {
        self.config.units = units;
        // Potentially add logic here later to convert existing weights/distances if desired,
        // but for now, just change the unit label.
        self.save_config()?;
        Ok(())
    }

    // --- Bodyweight Tracking Methods ---

    /// Adds a new bodyweight entry to the database.
    pub fn add_bodyweight_entry(&self, timestamp: DateTime<Utc>, weight: f64) -> Result<i64> {
        if weight <= 0.0 {
            bail!(ConfigError::InvalidBodyweightInput(
                "Bodyweight must be a positive number.".to_string()
            ));
        }
        db::add_bodyweight(&self.conn, timestamp, weight)
            .context("Failed to add bodyweight entry to database")
    }

    /// Retrieves the most recent bodyweight entry from the database.
    pub fn get_latest_bodyweight(&self) -> Result<Option<f64>> {
        db::get_latest_bodyweight(&self.conn)
            .context("Failed to retrieve latest bodyweight from database")
    }

    /// Lists logged bodyweight entries.
    pub fn list_bodyweights(&self, limit: u32) -> Result<Vec<(usize, DateTime<Utc>, f64)>> {
        db::list_bodyweights(&self.conn, limit).context("Failed to list bodyweights from database")
    }

    pub fn delete_bodyweight(&mut self, id: i64) -> Result<usize, DbError> {
        db::delete_bodyweight(&self.conn, id)
    }

    // --- Database Path ---
    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }

    // --- Exercise Identifier Resolution (Helper) ---

    /// Resolves an identifier (ID, Alias, Name) to an ExerciseDefinition.
    /// Returns Ok(None) if not found, Err if DB error occurs.
    pub fn resolve_exercise_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        // Call function from db module
        db::get_exercise_by_identifier(&self.conn, trimmed)
            .map(|opt_result| opt_result.map(|(definition, _)| definition)) // Discard ResolvedByType here
            .context(format!(
                "Failed to resolve exercise identifier '{identifier}'",
                
            ))
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
        db::create_exercise(&self.conn, trimmed_name, &type_, muscles).map_err(
            |db_err| match db_err {
                DbError::ExerciseNameNotUnique(_) => anyhow::anyhow!(db_err), // Keep specific error message
                _ => anyhow::Error::new(db_err).context(format!(
                    "Failed to create exercise '{trimmed_name}' in database",
                )),
            },
        )
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
        let current_def = self
            .resolve_exercise_identifier(identifier)?
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
                anyhow::anyhow!(
                    "Failed to rename exercise: the name '{}' is already taken.",
                    failed_name
                )
            }
            DbError::ExerciseNotFound(_) => {
                anyhow::anyhow!("Exercise '{}' not found to edit.", identifier)
            } // Make not found error specific
            _ => anyhow::Error::new(db_err).context(format!(
                "Failed to update exercise '{identifier}' in database"
            )),
        })
    }

    /// Deletes an exercise definition (identified by ID, Alias, or Name). Returns number of definitions deleted (0 or 1).
    /// Includes warnings about associated workouts and deletes associated aliases.
    pub fn delete_exercise(&mut self, identifiers: &Vec<String>) -> Result<u64> {
        let mut num_deleted = 0;
        for identifier in identifiers {
            // 1. Resolve identifier to get canonical name and check existence
            let exercise_def = self
                .resolve_exercise_identifier(identifier)?
                .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
            let canonical_name = exercise_def.name.clone(); // Clone needed for messages/DB call

            // 2. Check for associated workouts (using canonical name)
            let workout_count: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE",
                    [&canonical_name], // Use canonical name
                    |row| row.get(0),
                )
                .context(format!(
                    "Failed to check for associated workouts for '{canonical_name}'"
                ))?;

            if workout_count > 0 {
                // Print warning here. Ideally UI layer formats this, but simpler here for now.
                eprintln!(
                "Warning: Deleting exercise '{canonical_name}'. {workout_count} associated workout entries will remain but reference a deleted definition."
             );
            }

            // 3. Call DB function to delete exercise and its aliases (using canonical name)
            // Need mutable connection borrow for the transaction inside db::delete_exercise
            let mut_conn = &mut self.conn; // Create a mutable reference
            db::delete_exercise(mut_conn, &canonical_name).map_err(|e| match e {
                DbError::ExerciseNotFound(_) => {
                    anyhow::anyhow!("Exercise '{identifier}' not found to delete.")
                } // Should not happen if resolve worked, but handle anyway
                _ => anyhow::Error::new(e).context(format!(
                    "Failed to delete exercise '{canonical_name}' from database"
                )),
            })?;
            num_deleted += 1;
        }
        Ok(num_deleted)
    }

    /// Retrieves an exercise definition by ID, Alias or name.
    pub fn get_exercise_by_identifier_service(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
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
        if let Some((_, resolved_type)) = db::get_exercise_by_identifier(&self.conn, trimmed_alias)?
        {
            match resolved_type {
                ResolvedByType::Id => bail!(
                    "Alias '{trimmed_alias}' conflicts with an existing exercise ID.",
                ),
                ResolvedByType::Name => bail!(
                    "Alias '{trimmed_alias}' conflicts with an existing exercise name.",
                ),
                ResolvedByType::Alias => { /* This is handled by the INSERT constraint */ }
            }
        }

        // Resolve the target exercise identifier to its canonical name
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Call DB function
        db::create_alias(&self.conn, trimmed_alias, &canonical_name).map_err(
            |db_err| match db_err {
                DbError::AliasAlreadyExists(_) => anyhow::anyhow!(db_err), // Keep specific error
                _ => anyhow::Error::new(db_err).context(format!(
                    "Failed to create alias '{trimmed_alias}' for exercise '{canonical_name}'"
                )),
            },
        )
    }

    /// Deletes an exercise alias.
    pub fn delete_alias(&self, alias_name: &str) -> Result<u64> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        db::delete_alias(&self.conn, trimmed_alias).map_err(|db_err| match db_err {
            DbError::AliasNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
            _ => anyhow::Error::new(db_err)
                .context(format!("Failed to delete alias '{trimmed_alias}'")),
        })
    }

    /// Lists all defined aliases.
    pub fn list_aliases(&self) -> Result<HashMap<String, String>> {
        db::list_aliases(&self.conn).context("Failed to list aliases from database")
    }

    // --- Workout Entry Methods ---

    /// Adds a workout entry. Handles implicit exercise creation, bodyweight logic, past dates, and PB checking.
    /// Stores distance in km.
    /// Returns Result<(workout_id, Option<PBInfo>)>
    pub fn add_workout(
        &mut self,
        exercise_identifier: &str,
        date: NaiveDate,
        sets: Option<i64>,
        reps: Option<i64>,
        weight_arg: Option<f64>,
        duration: Option<i64>,
        distance_arg: Option<f64>,
        notes: Option<String>,
        implicit_type: Option<ExerciseType>,
        implicit_muscles: Option<String>,
        bodyweight_to_use: Option<f64>,
    ) -> Result<(i64, Option<PBInfo>)> {
        // 1. Resolve or implicitly create exercise definition
        let exercise_def =
            self.resolve_or_create_exercise(exercise_identifier, implicit_type, implicit_muscles)?;
        let canonical_exercise_name = &exercise_def.name;

        // 2. Determine final weight and distance values
        let final_weight =
            calculate_final_weight(&exercise_def, weight_arg, bodyweight_to_use)?;

        let final_distance_km = self.convert_distance(distance_arg);

        // 3. Get timestamp for the workout
        let timestamp = create_timestamp_from_date(date)?;

        // 4. Check for previous personal bests before adding new workout
        let previous_bests = self.get_previous_bests(canonical_exercise_name)?;

        // 5. Add the workout to the database
        let inserted_id = self.insert_workout_record(
            canonical_exercise_name,
            timestamp,
            sets,
            reps,
            final_weight,
            duration,
            final_distance_km,
            notes,
        )?;

        // 6. Check if any PBs were achieved and return results
        let pb_info = self.check_for_new_pbs(
            &previous_bests,
            final_weight,
            reps,
            duration,
            final_distance_km,
        );

        Ok((inserted_id, pb_info))
    }

    // Helper to resolve existing exercise or create a new one
    fn resolve_or_create_exercise(
        &self,
        exercise_identifier: &str,
        implicit_type: Option<ExerciseType>,
        implicit_muscles: Option<String>,
    ) -> Result<ExerciseDefinition> {
        match self.resolve_exercise_identifier(exercise_identifier)? {
            Some(def) => Ok(def),
            None => {
                // Try implicit creation if type and muscles provided
                if let (Some(db_type), Some(muscle_list)) = (implicit_type, implicit_muscles) {
                    println!(
                        "Exercise '{exercise_identifier}' not found, defining it implicitly...",
                    );

                    let muscles_opt = if muscle_list.trim().is_empty() {
                        None
                    } else {
                        Some(muscle_list.as_str())
                    };

                    match self.create_exercise(exercise_identifier, db_type, muscles_opt) {
                        Ok(id) => {
                            println!(
                                "Implicitly defined exercise: '{exercise_identifier}' (ID: {id})"
                            );

                            // Refetch the newly created definition
                            self.resolve_exercise_identifier(exercise_identifier)?
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Failed to re-fetch implicitly created exercise '{}'",
                                        exercise_identifier
                                    )
                                })
                        }
                        Err(e) => Err(e).context(format!(
                            "Failed to implicitly define exercise '{exercise_identifier}'"
                        )),
                    }
                } else {
                    // Not found and no implicit creation info provided
                    bail!(
                        "Exercise '{exercise_identifier}' not found. Define it first using 'create-exercise', use an alias, or provide details for implicit creation.",
                        
                    );
                }
            }
        }
    }


    // Helper to convert distance to kilometers
    fn convert_distance(&self, distance_arg: Option<f64>) -> Option<f64> {
        distance_arg.map(|dist| match self.config.units {
            Units::Metric => dist,
            Units::Imperial => dist * MILE_TO_KM 
        })
    }


    // Helper to get previous personal bests for an exercise
    fn get_previous_bests(&self, exercise_name: &str) -> Result<PreviousBests> {
        Ok(PreviousBests {
            weight: db::get_max_weight_for_exercise(&self.conn, exercise_name)?,
            reps: db::get_max_reps_for_exercise(&self.conn, exercise_name)?,
            duration: db::get_max_duration_for_exercise(&self.conn, exercise_name)?,
            distance_km: db::get_max_distance_for_exercise(&self.conn, exercise_name)?,
        })
    }

    // Helper to insert workout record into database
    fn insert_workout_record(
        &self,
        exercise_name: &str,
        timestamp: DateTime<Utc>,
        sets: Option<i64>,
        reps: Option<i64>,
        weight: Option<f64>,
        duration: Option<i64>,
        distance_km: Option<f64>,
        notes: Option<String>,
    ) -> Result<i64> {
        db::add_workout(
            &self.conn,
            exercise_name,
            timestamp,
            sets,
            reps,
            weight,
            duration,
            distance_km,
            notes,
        )
        .context("Failed to add workout to database")
    }

    // Helper to check for new personal bests
    fn check_for_new_pbs(
        &self,
        previous: &PreviousBests,
        weight: Option<f64>,
        reps: Option<i64>,
        duration: Option<i64>,
        distance_km: Option<f64>,
    ) -> Option<PBInfo> {
        let mut pb_info = PBInfo {
            previous_weight: previous.weight,
            previous_reps: previous.reps,
            previous_duration: previous.duration,
            previous_distance: previous.distance_km,
            new_weight: weight,
            new_reps: reps,
            new_duration: duration,
            new_distance: distance_km,
            ..Default::default()
        };

        // Check weight PB
        if self.config.notify_pb_weight {
            if let Some(current_weight) = weight {
                if current_weight > 0.0 && current_weight > previous.weight.unwrap_or(0.0) {
                    pb_info.achieved_weight_pb = true;
                }
            }
        }

        // Check reps PB
        if self.config.notify_pb_reps {
            if let Some(current_reps) = reps {
                if current_reps > 0 && current_reps > previous.reps.unwrap_or(0) {
                    pb_info.achieved_reps_pb = true;
                }
            }
        }

        // Check duration PB
        if self.config.notify_pb_duration {
            if let Some(current_duration) = duration {
                if current_duration > 0 && current_duration > previous.duration.unwrap_or(0) {
                    pb_info.achieved_duration_pb = true;
                }
            }
        }

        // Check distance PB
        if self.config.notify_pb_distance {
            if let Some(current_distance_km) = distance_km {
                if current_distance_km > 0.0
                    && current_distance_km > previous.distance_km.unwrap_or(0.0)
                {
                    pb_info.achieved_distance_pb = true;
                }
            }
        }

        // Return PB info only if a PB was actually achieved
        if pb_info.any_pb() {
            Some(pb_info)
        } else {
            None
        }
    }

    /// Edits an existing workout entry. Converts distance to km if units are Imperial.
    pub fn edit_workout(
        &self,
        id: i64,
        new_exercise_identifier: Option<String>,
        new_sets: Option<i64>,
        new_reps: Option<i64>,
        new_weight: Option<f64>, // Weight is set directly, no bodyweight logic re-applied
        new_duration: Option<i64>,
        new_distance_arg: Option<f64>, // Distance argument from CLI/TUI
        new_notes: Option<String>,
        new_date: Option<NaiveDate>, // Feature 3: Allow editing date
    ) -> Result<u64> {
        let mut new_workout = Workout {
            reps : new_reps,
            id,
            weight : new_weight,
            duration_minutes : new_duration,
            notes : new_notes,
            sets : new_sets,
            ..Default::default()
        };
        // Resolve the new exercise identifier to its canonical name if provided
        let new_canonical_name: Option<String> = match new_exercise_identifier {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(&ident)?
                    .ok_or_else(|| DbError::ExerciseNotFound(ident.clone()))?,
            ),
            None => None,
        };

        // Convert new_date to new_timestamp if provided
        let new_timestamp: Option<DateTime<Utc>> = match new_date {
            Some(date) => Some(
                date.and_hms_opt(12, 0, 0) // Create NaiveDateTime first
                    .and_then(|naive_dt| Utc.from_local_datetime(&naive_dt).single()) // Convert to DateTime<Utc>
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to create valid timestamp from date {}", date)
                    })?,
            ),
            None => None,
        };

        // Convert distance to km if necessary
        let new_distance_km = new_distance_arg.map(|dist| match self.config.units  {
                    Units::Metric => dist,             // Assume input is already km
                    Units::Imperial => dist * MILE_TO_KM, // Convert miles to km
        });

        new_workout.distance = new_distance_km;

        // Call function from db module
        db::update_workout(&self.conn, new_workout, new_canonical_name, new_timestamp)
            .with_context(|| format!("Failed to update workout ID {id}"))
    }

    /// Deletes a workout entry by ID.
    pub fn delete_workouts(&self, ids: &Vec<i64>) -> Result<Vec<u64>> {
        // Call function from db module
        let mut workouts_delete = vec![];
        for id in ids {
            db::delete_workout(&self.conn, *id).map_err(|db_err| match db_err {
                DbError::WorkoutNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
                _ => anyhow::Error::new(db_err)
                    .context(format!("Failed to delete workout ID {id}")),
            })?;
            workouts_delete.push(*id as u64);
        }
        Ok(workouts_delete)
    }

    /// Lists workouts based on filters. Resolves exercise identifier if provided.
    pub fn list_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>> {
        // Resolve exercise identifier filter to canonical name if present
        let canonical_exercise_name = match filters.exercise_name {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(ident)?
                    .ok_or_else(|| {
                        // If identifier doesn't resolve, treat as no matching workouts found
                        eprintln!(
                            "Warning: Exercise identifier '{ident}' not found for filtering."
                        );
                        DbError::ExerciseNotFound(ident.to_string()) // Return specific error
                    })?,
            ),
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
    ///
    /// # Arguments
    ///
    /// * `exercise_identifier` - The ID, alias, or name of the exercise to look up
    /// * `n` - Which occurrence to find (1 for most recent day, 2 for second most recent, etc.)
    ///
    /// # Returns
    ///
    /// A vector of `Workout` objects from the specified day
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The exercise identifier cannot be resolved to a canonical name
    /// * The exercise is not found in the database
    /// * There was an error accessing the database
    /// * The specified nth occurrence doesn't exist (not enough workout history)
    pub fn list_workouts_for_exercise_on_nth_last_day(
        &self,
        exercise_identifier: &str,
        n: u32,
    ) -> Result<Vec<Workout>> {
        // Resolve identifier to canonical name
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Call function from db module
        db::list_workouts_for_exercise_on_nth_last_day(&self.conn, &canonical_name, n)
            .map_err(anyhow::Error::new) // Convert DbError to anyhow::Error
            .with_context(|| {
                format!(
                    "Failed to list workouts for exercise '{canonical_name}' on nth last day {n}"
                )
            })
    }

    // --- Statistics Method ---
    pub fn get_exercise_stats(&self, exercise_identifier: &str) -> Result<ExerciseStats> {
        // 1. Resolve identifier
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // 2. Get all timestamps for the exercise
        let timestamps = db::get_workout_timestamps_for_exercise(&self.conn, &canonical_name)
            .context(format!(
                "Failed to retrieve workout history for '{canonical_name}'"
            ))?;

        if timestamps.is_empty() {
            return Err(DbError::NoWorkoutDataFound(canonical_name).into());
        }

        // 3. Calculate basic stats
        let total_workouts = timestamps.len();
        let first_timestamp = timestamps.first().unwrap(); // Safe due to is_empty check
        let last_timestamp = timestamps.last().unwrap(); // Safe due to is_empty check
        let first_workout_date = Some(first_timestamp.date_naive());
        let last_workout_date = Some(last_timestamp.date_naive());

        // 4. Calculate average workouts per week
        let avg_workouts_per_week = if total_workouts <= 1 {
            None // Cannot calculate average for 0 or 1 workout
        } else {
            let duration_days = (*last_timestamp - *first_timestamp).num_days();
            if duration_days == 0 {
                // Multiple workouts on the same day - technically infinite avg/week, return None or handle differently?
                // Let's consider it as "at least daily" which doesn't fit avg/week well. Return None.
                None
            } else {
                let duration_weeks = (duration_days as f64 / 7.0).max(1.0 / 7.0); // Avoid division by zero, ensure at least 1 day = 1/7 week
                Some(total_workouts as f64 / duration_weeks)
            }
        };

        // 5. Calculate longest gap
        let longest_gap_days: Option<u64> = if total_workouts > 1 {
            let mut max_gap: i64 = 0;
            for i in 1..total_workouts {
                let gap =
                    (timestamps[i].date_naive() - timestamps[i - 1].date_naive()).num_days() - 1;
                if gap > max_gap {
                    max_gap = gap;
                }
            }
            Some(max_gap as u64)
        } else {None};

        // 6. Calculate streaks
        let streak_interval = Duration::days(self.config.streak_interval_days as i64);
        let mut current_streak = 0u32;
        let mut longest_streak = 0u32;

        if total_workouts > 0 {
            current_streak = 1; // Start with 1 for the first workout
            longest_streak = 1;
            let mut last_streak_date = timestamps[0].date_naive();

            for time_stamp in timestamps.iter().skip(1).take(total_workouts - 1) {
                let current_date = time_stamp.date_naive();
                // Ignore multiple workouts on the same day for streak calculation
                if current_date == last_streak_date {
                    continue;
                }
                // Check if the gap is within the allowed interval
                if current_date - last_streak_date <= streak_interval {
                    current_streak += 1;
                } else {
                    // Streak broken, reset current streak
                    current_streak = 1;
                }
                // Update longest streak if current is longer
                if current_streak > longest_streak {
                    longest_streak = current_streak;
                }
                last_streak_date = current_date; // Update the date for the next comparison
            }

            // Check if the *current* streak is still active based on the last workout date and today
            let today = Utc::now().date_naive();
            if today - last_timestamp.date_naive() > streak_interval {
                current_streak = 0; // Current streak is broken if the last workout is too old
            }
        }

        // 7. Get Personal Bests
        let personal_bests = PersonalBests {
            max_weight: db::get_max_weight_for_exercise(&self.conn, &canonical_name)?,
            max_reps: db::get_max_reps_for_exercise(&self.conn, &canonical_name)?,
            max_duration_minutes: db::get_max_duration_for_exercise(&self.conn, &canonical_name)?,
            max_distance_km: db::get_max_distance_for_exercise(&self.conn, &canonical_name)?,
        };

        Ok(ExerciseStats {
            canonical_name,
            total_workouts,
            first_workout_date,
            last_workout_date,
            avg_workouts_per_week,
            longest_gap_days,
            personal_bests,
            current_streak,
            longest_streak,
            streak_interval_days: self.config.streak_interval_days,
        })
    }

    pub fn calculate_daily_volume(
        &self,
        filters: VolumeFilters,
    ) -> Result<Vec<(NaiveDate, String, f64)>> {
        // Resolve exercise identifier filter to canonical name if present
        let canonical_exercise_name = match filters.exercise_name {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(ident)?
                    .ok_or_else(|| {
                        eprintln!(
                            "Warning: Exercise identifier '{ident}' not found for filtering volume."
                        );
                        DbError::ExerciseNotFound(ident.to_string())
                    })?,
            ),
            None => None,
        };

        // Create new filters struct with resolved name
        let resolved_filters = VolumeFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            ..filters // Copy other filters (dates, type, muscle, limit)
        };

        db::calculate_daily_volume_filtered(&self.conn, resolved_filters)
            .context("Failed to calculate workout volume from database")
    }

    pub fn get_all_dates_with_exercise(&self) -> Result<Vec<NaiveDate>, DbError> {
        db::get_all_dates_with_exercise(&self.conn)
    }

    /// Fetches and processes workout data for the specified graph type,
    /// aggregating data per day.
    /// Returns Vec<(f64, f64)> where x is days since first workout, y is the metric.
    pub fn get_data_for_graph(
        &self,
        exercise_identifier: &str,
        graph_type: GraphType,
    ) -> Result<Vec<(f64, f64)>> {
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Fetch raw history, sorted chronologically
        let filter = WorkoutFilters {
            exercise_name: Some(&canonical_name),
            ..Default::default()
        };
        let history = list_workouts_filtered(&self.conn, filter)?;
        if history.is_empty() {
            return Ok(vec![]); // No data, return empty vec
        }

        // Use BTreeMap to store aggregated data per day (NaiveDate -> aggregated value)
        // BTreeMap keeps keys (dates) sorted automatically.
        let mut daily_aggregated_data: BTreeMap<NaiveDate, f64> = BTreeMap::new();

        for w in history {
            let date = w.timestamp.date_naive();

            match graph_type {
                GraphType::Estimated1RM => {
                    if let (Some(weight), Some(reps)) = (w.weight, w.reps) {
                        if let Some(e1rm) = calculate_e1rm(weight, reps) {
                            let current_max = daily_aggregated_data.entry(date).or_insert(0.0);
                            *current_max = current_max.max(e1rm); // Keep the max E1RM for the day
                        }
                    }
                }
                GraphType::MaxWeight => {
                    if let Some(weight) = w.weight.filter(|&wg| wg > 0.0) {
                        let current_max = daily_aggregated_data.entry(date).or_insert(0.0);
                        *current_max = current_max.max(weight); // Keep the max weight for the day
                    }
                }
                GraphType::MaxReps => {
                    // Keep the max reps *in a single set* for the day
                    if let Some(reps) = w.reps.filter(|&r| r > 0) {
                        let current_max = daily_aggregated_data.entry(date).or_insert(0.0);
                        *current_max = current_max.max(reps as f64);
                    }
                }
                GraphType::WorkoutVolume => {
                    // Sum the volume (sets * reps * weight) for the day
                    let sets = w.sets.unwrap_or(1).max(1); // Assume at least 1 set
                    let reps = w.reps.unwrap_or(0);
                    let weight = w.weight.unwrap_or(0.0);
                    let volume = sets as f64 * reps as f64 * weight;
                    if volume > 0.0 {
                        *daily_aggregated_data.entry(date).or_insert(0.0) += volume;
                    }
                }
                GraphType::WorkoutReps => {
                    // Sum the total reps (sets * reps) for the day
                    let sets = w.sets.unwrap_or(1).max(1);
                    let reps = w.reps.unwrap_or(0);
                    let total_reps = sets * reps;
                    if total_reps > 0 {
                        *daily_aggregated_data.entry(date).or_insert(0.0) += total_reps as f64;
                    }
                }
                GraphType::WorkoutDuration => {
                    // Sum the duration for the day
                    if let Some(duration) = w.duration_minutes.filter(|&d| d > 0) {
                        *daily_aggregated_data.entry(date).or_insert(0.0) += duration as f64;
                    }
                }
                GraphType::WorkoutDistance => {
                    // Sum the distance (in km) for the day
                    if let Some(distance_km) = w.distance.filter(|&d| d > 0.0) {
                        *daily_aggregated_data.entry(date).or_insert(0.0) += distance_km;
                    }
                }
            }
        }

        // Find the first day with data to calculate relative days
        let first_day_ce = match daily_aggregated_data.keys().next() {
            Some(first_date) => first_date.num_days_from_ce(),
            None => return Ok(vec![]), // Should not happen if history wasn't empty, but handle defensively
        };

        // Convert the aggregated map to the final Vec<(f64, f64)> format
        let data_points: Vec<(f64, f64)> = daily_aggregated_data
            .into_iter()
            .map(|(date, value)| {
                let days_since_first = (date.num_days_from_ce() - first_day_ce) as f64;

                // Apply final unit conversion only for distance
                let final_value = if graph_type == GraphType::WorkoutDistance {
                    match self.config.units {
                        Units::Metric => value,              // Value is already km sum
                        Units::Imperial => value * KM_TO_MILE, // Convert km sum to miles
                    }
                } else {
                    value // Other types don't need unit conversion here
                };

                (days_since_first, final_value)
            })
            // Filter out potential zero values that might remain if only zero-value workouts existed for a day
            // For volume/reps/duration/distance sums, this is fine.
            // For MaxWeight/MaxReps/E1RM, the `.filter()` earlier should prevent zeros unless the max *was* technically 0 (unlikely for valid data).
            .filter(|&(_, y)| y > 0.0) // Only include days where the aggregated metric is > 0
            .collect();

        Ok(data_points)
    }
    pub fn list_all_muscles(&self) -> Result<Vec<String>> {
        db::list_all_muscles(&self.conn)
            .context("Failed to retrieve list of all muscles from the database")
    }
}

fn calculate_e1rm(weight: f64, reps: i64) -> Option<f64> {
    if reps > 0 && weight > 0.0 {
        // Ensure reps is converted to f64 for the division
        let e1rm = weight * (1.0 + (reps as f64 / 30.0));
        Some(e1rm)
    } else {
        // Cannot calculate E1RM for non-positive reps or weight
        None
    }
}
    // Helper to create a timestamp from a date
    fn create_timestamp_from_date(date: NaiveDate) -> Result<DateTime<Utc>> {
        // Use noon UTC on the given date to represent the day without time specifics
        let date_and_time = date
            .and_hms_opt(12, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Internal error creating NaiveDateTime from date"))?;

        Ok(Utc.from_utc_datetime(&date_and_time))
    }
    // Helper to calculate final weight based on exercise type
    fn calculate_final_weight(
        exercise_def: &ExerciseDefinition,
        weight_arg: Option<f64>,
        bodyweight_to_use: Option<f64>,
    ) -> Result<Option<f64>> {
        if exercise_def.type_ == ExerciseType::BodyWeight {
            match bodyweight_to_use {
                Some(bw) => Ok(Some(bw + weight_arg.unwrap_or(0.0))),
                None => bail!(
                    "Bodyweight is required for exercise '{}' but was not provided.",
                    exercise_def.name
                ),
            }
        } else {
            Ok(weight_arg) // Use the provided weight directly for non-bodyweight exercises
        }
    }
