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
    Config, ConfigError, ThemeConfig, Units, StandardColor, parse_color, PbMetricScope,
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

     /// Sets the PB metric scope in the config and saves it.
     pub fn set_pb_scope(&mut self, scope: PbMetricScope) -> Result<(), ConfigError> {
         self.config.pb_metric_scope = scope;
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

    pub fn set_units(&mut self, units: Units) -> Result<(), ConfigError> {
        self.config.units = units;
        // Potentially add logic here later to convert existing weights if desired,
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
         // Check weight PB only if scope allows (Feature 2)
         if self.config.pb_metric_scope == PbMetricScope::All || self.config.pb_metric_scope == PbMetricScope::Weight {
             if let Some(current_weight) = final_weight {
                 if current_weight > 0.0 && current_weight > previous_max_weight.unwrap_or(0.0) {
                     is_weight_pb = true;
                 }
             }
         }
         if self.config.pb_metric_scope == PbMetricScope::All || self.config.pb_metric_scope == PbMetricScope::Reps {
             if let Some(current_reps) = reps {
                 if current_reps > 0 && current_reps > previous_max_reps.unwrap_or(0) {
                     is_reps_pb = true;
                 }
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

