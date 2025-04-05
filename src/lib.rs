use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Duration}; // Add Duration, TimeZone
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::collections::HashMap; // For list_aliases return type

// --- Declare modules to load from separate files ---
mod config;
pub mod db;

// --- Expose public types ---
pub use config::{
    Config, ConfigError, ThemeConfig, Units, StandardColor, parse_color, // PbMetricScope removed
    get_config_path as get_config_path_util, // Rename utility function
    load_config as load_config_util,         // Rename utility function
    save_config as save_config_util,         // Rename utility function
};
pub use db::{
    DbError, ExerciseDefinition, ExerciseType, Workout, WorkoutFilters, ResolvedByType,
    get_db_path as get_db_path_util, // Rename utility function
    VolumeFilters
};

// --- Personal Best Information (Feature 4) ---
// Replaced PBType with boolean flags within PBInfo
// #[derive(Debug, Clone, PartialEq)]
// pub enum PBType {
//     Weight,
//     Reps,
//     Duration,
//     Distance,
//     // Combinations could be added if needed, but individual flags are simpler
// }

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
    pub new_distance: Option<f64>,    // Stored as km
    pub previous_distance: Option<f64>, // Stored as km
}

impl PBInfo {
    /// Helper to check if any PB was achieved.
    pub fn any_pb(&self) -> bool {
        self.achieved_weight_pb || self.achieved_reps_pb || self.achieved_duration_pb || self.achieved_distance_pb
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
        let mut conn = db::open_db(&db_path) // Use mutable conn for init potentially
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

     /// Sets the streak interval in the config and saves it.
     pub fn set_streak_interval(&mut self, days: u32) -> Result<(), ConfigError> {
         if days == 0 {
             // Although CLI parser prevents 0, add safeguard here
             return Err(ConfigError::InvalidBodyweightInput("Streak interval must be at least 1 day.".to_string()));
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
         self.config.notify_pb_enabled.ok_or(ConfigError::PbNotificationNotSet)
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

    // --- Units Config ---
    pub fn set_units(&mut self, units: Units) -> Result<(), ConfigError> {
        self.config.units = units;
        // Potentially add logic here later to convert existing weights/distances if desired,
        // but for now, just change the unit label.
        self.save_config()?;
        Ok(())
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
             DbError::ExerciseNotFound(_) => anyhow::anyhow!("Exercise '{}' not found to edit.", identifier), // Make not found error specific
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
        // Need mutable connection borrow for the transaction inside db::delete_exercise
        let mut_conn = &mut self.conn; // Create a mutable reference
        db::delete_exercise(mut_conn, &canonical_name)
            .map_err(|e| match e {
                DbError::ExerciseNotFound(_) => anyhow::anyhow!("Exercise '{}' not found to delete.", identifier), // Should not happen if resolve worked, but handle anyway
                _ => anyhow::Error::new(e).context(format!("Failed to delete exercise '{}' from database", canonical_name)),
            })
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
    /// Stores distance in km.
    /// Returns Result<(workout_id, Option<PBInfo>)>
    pub fn add_workout(
        &mut self, // Needs mut because bodyweight prompt might update config via caller
        exercise_identifier: &str,
        date: NaiveDate,
        sets: Option<i64>,
        reps: Option<i64>,
        weight_arg: Option<f64>, // Weight from CLI/TUI args
        duration: Option<i64>,
        distance_arg: Option<f64>, // Distance from CLI/TUI args
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

        // 3. Convert distance to km if necessary and store
        let final_distance_km = match distance_arg {
            Some(dist) => {
                match self.config.units {
                    Units::Metric => Some(dist), // Assume input is already km
                    Units::Imperial => Some(dist * 1.60934), // Convert miles to km
                }
            }
            None => None,
        };


        // 4. Determine timestamp (Feature 3)
        // Use noon UTC on the given date to represent the day without time specifics
        let date_and_time = date.and_hms_opt(12, 0, 0)
                                .ok_or_else(|| anyhow::anyhow!("Internal error creating NaiveDateTime from date"))?;
        let timestamp = Utc.from_utc_datetime(&date_and_time);

         // 5. Check for PBs *before* adding the new workout (Feature 4)
         let previous_max_weight = db::get_max_weight_for_exercise(&self.conn, &canonical_exercise_name)?;
         let previous_max_reps = db::get_max_reps_for_exercise(&self.conn, &canonical_exercise_name)?;
         let previous_max_duration = db::get_max_duration_for_exercise(&self.conn, &canonical_exercise_name)?;
         let previous_max_distance_km = db::get_max_distance_for_exercise(&self.conn, &canonical_exercise_name)?;


        // 6. Add the workout entry using the canonical exercise name, final weight, distance(km), and timestamp
        let inserted_id = db::add_workout(
            &self.conn,
            &canonical_exercise_name, // Use canonical name
            timestamp,
            sets,
            reps,
            final_weight, // Use calculated weight
            duration,
            final_distance_km, // Store distance in km
            notes,
        )
        .context("Failed to add workout to database")?;

         // 7. Determine if a PB was achieved (Feature 4)
         let mut pb_info = PBInfo {
            previous_weight:previous_max_weight, previous_reps: previous_max_reps, previous_duration: previous_max_duration, previous_distance: previous_max_distance_km,
            new_weight: final_weight, new_reps: reps, new_duration: duration, new_distance: final_distance_km,
            ..Default::default() // Initialize achieved flags to false
         };

         // Check weight PB
         if self.config.notify_pb_weight {
             if let Some(current_weight) = final_weight {
                 if current_weight > 0.0 && current_weight > previous_max_weight.unwrap_or(0.0) {
                     pb_info.achieved_weight_pb = true;
                 }
             }
         }
         // Check reps PB
         if self.config.notify_pb_reps {
              if let Some(current_reps) = reps {
                  if current_reps > 0 && current_reps > previous_max_reps.unwrap_or(0) {
                      pb_info.achieved_reps_pb = true;
                  }
              }
         }
         // Check duration PB
         if self.config.notify_pb_duration {
              if let Some(current_duration) = duration {
                  if current_duration > 0 && current_duration > previous_max_duration.unwrap_or(0) {
                      pb_info.achieved_duration_pb = true;
                  }
              }
         }
          // Check distance PB
         if self.config.notify_pb_distance {
              if let Some(current_distance_km) = final_distance_km {
                  // Use a small epsilon for float comparison? Might be overkill for distance PBs.
                  if current_distance_km > 0.0 && current_distance_km > previous_max_distance_km.unwrap_or(0.0) {
                       pb_info.achieved_distance_pb = true;
                  }
              }
         }


         // Return ID and PB info only if a PB was actually achieved
        let result_pb_info = if pb_info.any_pb() { Some(pb_info) } else { None };
        Ok((inserted_id, result_pb_info))
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

        // Convert distance to km if necessary
        let new_distance_km = match new_distance_arg {
            Some(dist) => {
                match self.config.units {
                    Units::Metric => Some(dist), // Assume input is already km
                    Units::Imperial => Some(dist * 1.60934), // Convert miles to km
                }
            }
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
            new_distance_km, // Pass Option<f64> (km)
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
                                    DbError::ExerciseNotFound(ident.to_string()) // Return specific error
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

     // --- Statistics Method ---
     pub fn get_exercise_stats(&self, exercise_identifier: &str) -> Result<ExerciseStats> {
        // 1. Resolve identifier
        let canonical_name = self.resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // 2. Get all timestamps for the exercise
        let timestamps = db::get_workout_timestamps_for_exercise(&self.conn, &canonical_name)
            .context(format!("Failed to retrieve workout history for '{}'", canonical_name))?;

        if timestamps.is_empty() {
            return Err(DbError::NoWorkoutDataFound(canonical_name).into());
        }

        // 3. Calculate basic stats
        let total_workouts = timestamps.len();
        let first_timestamp = timestamps.first().unwrap(); // Safe due to is_empty check
        let last_timestamp = timestamps.last().unwrap();   // Safe due to is_empty check
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
        let mut longest_gap_days: Option<u64> = None;
        if total_workouts > 1 {
            let mut max_gap: i64 = 0;
            for i in 1..total_workouts {
                let gap = (timestamps[i].date_naive() - timestamps[i - 1].date_naive()).num_days() - 1;
                if gap > max_gap {
                    max_gap = gap;
                }
            }
             longest_gap_days = Some(max_gap as u64); // Convert to u64
        }

        // 6. Calculate streaks
        let streak_interval = Duration::days(self.config.streak_interval_days as i64);
        let mut current_streak = 0u32;
        let mut longest_streak = 0u32;

        if total_workouts > 0 {
            current_streak = 1; // Start with 1 for the first workout
            longest_streak = 1;
            let mut last_streak_date = timestamps[0].date_naive();

            for i in 1..total_workouts {
                let current_date = timestamps[i].date_naive();
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

     pub fn calculate_daily_volume(&self, filters: VolumeFilters) -> Result<Vec<(NaiveDate, String, f64)>> {
         // Resolve exercise identifier filter to canonical name if present
         let canonical_exercise_name = match filters.exercise_name {
             Some(ident) => Some(self.resolve_identifier_to_canonical_name(ident)?
                               .ok_or_else(|| {
                                    eprintln!("Warning: Exercise identifier '{}' not found for filtering volume.", ident);
                                    DbError::ExerciseNotFound(ident.to_string())
                                })?),
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

}

