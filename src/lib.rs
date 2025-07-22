use anyhow::{bail, Context, Result};
// Use anyhow::Result as standard Result for service layer
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use crate::sync_client::{ChangesPayload, ConfigChange};
use db::NewWorkoutData; // Import specific struct needed
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toml;

// --- Declare modules ---
mod config;
pub mod db;
pub mod sync_client;

// --- Expose public types ---
pub use config::{
    get_config_path as get_config_path_util,
    load as load_config_util,
    parse_color,
    save as save_config_util,
    Config,
    ConfigError, // Renamed from Error
    PbNotificationConfig,
    StandardColor,
    Theme,
    Units,
};


pub use db::{
    get_db_path as get_db_path_util,
    Error as DbError, // Renamed from DbError
    // list_aliases as list_aliases_util, // Example if needed
    ExerciseDefinition,
    ExerciseType,
    ResolvedByType,
    VolumeFilters,
    Workout,
    WorkoutFilters,
};

pub const KM_TO_MILE: f64 = 0.621_371;
pub const MILE_TO_KM: f64 = 1.60934;

// Helper struct to hold previous bests internally
#[derive(Debug)]
struct PreviousBests {
    weight: Option<f64>,
    reps: Option<i64>,
    duration: Option<i64>,
    distance_km: Option<f64>,
}

impl PreviousBests {
    const fn no_records(&self) -> bool {
        self.weight.is_none()
            && self.reps.is_none()
            && self.duration.is_none()
            && self.distance_km.is_none()
    }
}

#[derive(Default)]
pub struct EditWorkoutParams {
    pub id: i64,
    pub new_exercise_identifier: Option<String>,
    pub new_sets: Option<i64>,
    pub new_reps: Option<i64>,
    pub new_weight: Option<f64>,
    pub new_bodyweight: Option<f64>,
    pub new_duration: Option<i64>,
    pub new_distance_arg: Option<f64>,
    pub new_notes: Option<String>,
    pub new_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphType {
    Estimated1RM,
    MaxWeight,
    MaxReps,
    WorkoutVolume,
    WorkoutReps,
    WorkoutDuration,
    WorkoutDistance,
}

#[derive(Default, Clone)]
pub struct AddWorkoutParams<'a> {
    pub exercise_identifier: &'a str,
    pub date: DateTime<Utc>,
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration: Option<i64>,
    pub distance: Option<f64>,
    pub notes: Option<String>,
    pub implicit_type: Option<ExerciseType>,
    pub implicit_muscles: Option<String>,
    pub bodyweight_to_use: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct PbMetricInfo<T: PartialEq + Default + Copy> {
    pub achieved: bool,
    pub new_value: Option<T>,
    pub previous_value: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PBInfo {
    pub weight: PbMetricInfo<f64>,
    pub reps: PbMetricInfo<i64>,
    pub duration: PbMetricInfo<i64>,
    pub distance: PbMetricInfo<f64>, // Always stored/compared as km
}

impl PBInfo {
    /// Helper to check if any PB was achieved.
    #[must_use]
    pub const fn any_pb(&self) -> bool {
        self.weight.achieved
            || self.reps.achieved
            || self.duration.achieved
            || self.distance.achieved
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct PersonalBests {
    pub max_weight: Option<f64>,
    pub max_reps: Option<i64>,
    pub max_duration_minutes: Option<i64>,
    pub max_distance_km: Option<f64>, // Always store in km
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

pub struct AppService {
    pub config: Config,
    pub conn: Connection,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct SyncSummary {
    pub config: bool,
    pub exercises: usize,
    pub workouts: usize,
    pub aliases: usize,
    pub bodyweights: usize,
}




impl AppService {
    /// Initializes the application service.
    /// # Errors
    /// Returns `anyhow::Error` if config/db path determination, loading, or initialization fails.
    pub fn initialize() -> Result<Self> {
        let config_path =
            config::get_config_path().context("Failed to determine configuration file path")?;
        let config = config::load(&config_path)
            .context(format!("Failed to load config from {config_path:?}"))?;

        let db_path = db::get_db_path().context("Failed to determine database path")?;
        let conn = db::open_db(&db_path)
            .with_context(|| format!("Failed to open database at {db_path:?}"))?;

        db::init(&conn).context("Failed to initialize database schema")?;

        Ok(Self {
            config,
            conn,
            db_path,
            config_path,
        })
    }

    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    /// Saves the current configuration state.
    /// # Errors
    /// Returns `ConfigError` if saving fails.
    pub fn save_config(&self) -> Result<(), ConfigError> {
        config::save(&self.config_path, &self.config)
    }

    /// Sets the bodyweight in the configuration.
    /// # Errors
    /// - `ConfigError::InvalidBodyweightInput` if weight is not positive.
    /// - `ConfigError` variants if saving fails.
    pub fn set_bodyweight(&mut self, weight: f64) -> Result<(), ConfigError> {
        if weight <= 0.0 {
            return Err(ConfigError::InvalidBodyweightInput(
                "Weight must be a positive number.".to_string(),
            ));
        }
        self.config.bodyweight = Some(weight);
        self.save_config()
    }

    /// Gets the configured bodyweight if set.
    /// # Errors
    /// Returns `ConfigError::BodyweightNotSet` if bodyweight is `None`.
    pub fn get_required_bodyweight(&self) -> Result<f64, ConfigError> {
        self.config
            .bodyweight
            .ok_or_else(|| ConfigError::BodyweightNotSet(self.config_path.clone()))
    }

    /// Disables the bodyweight prompt.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn disable_bodyweight_prompt(&mut self) -> Result<(), ConfigError> {
        self.config.prompt_for_bodyweight = false;
        self.save_config()
    }

    /// Sets the streak interval (in days).
    /// # Errors
    /// - `ConfigError::InvalidStreakInterval` if `days` is 0.
    /// - `ConfigError` variants if saving fails.
    pub fn set_streak_interval(&mut self, days: u32) -> Result<(), ConfigError> {
        if days == 0 {
            return Err(ConfigError::InvalidStreakInterval(days));
        }
        self.config.streak_interval_days = days;
        self.save_config()
    }

    /// Sets the global PB notification preference.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_pb_notification_enabled(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.pb_notifications.enabled = Some(enabled);
        self.save_config()
    }

    /// Checks the global PB notification config.
    /// # Errors
    /// Returns `ConfigError::PbNotificationNotSet` if the `enabled` flag is `None`.
    pub fn check_pb_notification_config(&self) -> Result<bool, ConfigError> {
        self.config
            .pb_notifications
            .enabled
            .ok_or(ConfigError::PbNotificationNotSet)
    }

    /// Sets the weight PB notification flag.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_pb_notify_weight(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.pb_notifications.notify_weight = enabled;
        self.save_config()
    }
    /// Sets the reps PB notification flag.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_pb_notify_reps(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.pb_notifications.notify_reps = enabled;
        self.save_config()
    }
    /// Sets the duration PB notification flag.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_pb_notify_duration(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.pb_notifications.notify_duration = enabled;
        self.save_config()
    }
    /// Sets the distance PB notification flag.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_pb_notify_distance(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.pb_notifications.notify_distance = enabled;
        self.save_config()
    }

    /// Sets the target bodyweight.
    /// # Errors
    /// - `ConfigError::InvalidBodyweightInput` if weight is not positive.
    /// - `ConfigError` variants if saving fails.
    pub fn set_target_bodyweight(&mut self, weight: Option<f64>) -> Result<(), ConfigError> {
        if let Some(w) = weight {
            if w <= 0.0 {
                return Err(ConfigError::InvalidBodyweightInput(
                    "Target weight must be positive.".to_string(),
                ));
            }
        }
        self.config.target_bodyweight = weight;
        self.save_config()
    }

    pub const fn get_target_bodyweight(&self) -> Option<f64> {
        self.config.target_bodyweight
    }

    /// Sets the measurement units.
    /// # Errors
    /// Returns `ConfigError` variants if saving fails.
    pub fn set_units(&mut self, units: Units) -> Result<(), ConfigError> {
        self.config.units = units;
        self.save_config()
    }

    /// Adds a new bodyweight entry.
    /// # Errors
    /// - `ConfigError::InvalidBodyweightInput` if weight not positive.
    /// - `anyhow::Error` wrapping `DbError` variants.
    pub fn add_bodyweight_entry(&self, timestamp: DateTime<Utc>, weight: f64) -> Result<i64> {
        if weight <= 0.0 {
            bail!(ConfigError::InvalidBodyweightInput(
                "Bodyweight must be positive.".to_string()
            ));
        }
        db::add_bodyweight(&self.conn, timestamp, weight)
            .context("Failed to add bodyweight entry")
            .map_err(Into::into)
    }

    /// Retrieves the most recent bodyweight entry.
    /// # Errors
    /// - `anyhow::Error` wrapping `DbError` variants.
    pub fn get_latest_bodyweight(&self) -> Result<Option<f64>> {
        db::get_latest_bodyweight(&self.conn)
            .context("Failed to retrieve latest bodyweight")
            .map_err(Into::into)
    }

    /// Lists logged bodyweight entries.
    /// # Errors
    /// - `anyhow::Error` wrapping `DbError` variants.
    pub fn list_bodyweights(&self, limit: u32) -> Result<Vec<(i64, DateTime<Utc>, f64)>> {
        db::list_bodyweights(&self.conn, limit)
            .context("Failed to list bodyweights")
            .map_err(Into::into)
    }

    /// Deletes a bodyweight entry by ID.
    /// # Errors
    /// Returns `DbError` variants if deletion fails.
    pub fn delete_bodyweight(&mut self, id: i64) -> Result<usize, DbError> {
        db::delete_bodyweight(&self.conn, id)
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }

    /// Resolves an identifier (ID, Alias, Name) to an `ExerciseDefinition`.
    /// # Errors
    /// Returns `anyhow::Error` if identifier is empty or resolution fails.
    pub fn resolve_exercise_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        db::get_exercise_by_identifier(&self.conn, trimmed)
            .map(|opt_res| opt_res.map(|(def, _)| def))
            .with_context(|| format!("Failed to resolve exercise identifier '{trimmed}'"))
            .map_err(Into::into)
    }

    /// Resolves an identifier (ID, Alias, Name) to its canonical name.
    /// # Errors
    /// Returns `anyhow::Error` if identifier is empty or resolution fails.
    fn resolve_identifier_to_canonical_name(&self, identifier: &str) -> Result<Option<String>> {
        self.resolve_exercise_identifier(identifier)
            .map(|opt_def| opt_def.map(|def| def.name))
    }

    /// Creates a new exercise definition.
    /// # Errors
    /// Returns `anyhow::Error` if name is empty or DB insertion fails.
    pub fn create_exercise(
        &self,
        name: &str,
        type_: ExerciseType,
        log_flags: Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)>, // (w, r, dur, dist)
        muscles: Option<&str>,
    ) -> Result<i64> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            bail!("Exercise name cannot be empty.");
        }
        let (log_w, log_r, log_dur, log_dist) = match log_flags {
            Some((w, r, dur, dist)) => (w, r, dur, dist),
            None => (None, None, None, None), // Use DB defaults based on type
        };
        db::create_exercise(
            &self.conn,
            trimmed_name,
            &type_,
            muscles,
            log_w,
            log_r,
            log_dur,
            log_dist,
        )
        .map_err(|db_err| match db_err {
            DbError::ExerciseNameNotUnique(_) => anyhow::anyhow!(db_err),
            _ => anyhow::Error::new(db_err)
                .context(format!("Failed to create exercise '{trimmed_name}'")),
        })
    }

    /// Edits an existing exercise definition.
    /// # Arguments
    /// * `log_flags`: Optional tuple of new flags (w, r, dur, dist). Use `None` for flags you don't want to change.
    /// # Errors
    /// Returns `anyhow::Error` if identifier invalid, new name invalid, or DB update fails.
    pub fn edit_exercise(
        &mut self,
        identifier: &str,
        new_name: Option<&str>,
        new_type: Option<ExerciseType>,
        log_flags: Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)>, // (w, r, dur, dist)
        new_muscles: Option<Option<&str>>,
    ) -> Result<u64> {
        let current_def = self
            .resolve_exercise_identifier(identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let canonical_name_to_update = current_def.name;

        let trimmed_new_name = new_name.map(str::trim).filter(|n| !n.is_empty());
        if new_name.is_some() && trimmed_new_name.is_none() {
            bail!("New exercise name cannot be empty if provided.");
        }
        let (log_w, log_r, log_dur, log_dist) = match log_flags {
            Some((w, r, dur, dist)) => (w, r, dur, dist),
            None => (None, None, None, None), // No changes to flags
        };

        db::update_exercise(
            &mut self.conn,
            &canonical_name_to_update,
            trimmed_new_name,
            new_type.as_ref(),
            new_muscles,
            log_w,
            log_r,
            log_dur,
            log_dist,
        )
        .map_err(|db_err| match db_err {
            DbError::ExerciseNameNotUnique(name) => {
                anyhow::anyhow!("Name '{name}' is already taken.")
            }
            DbError::ExerciseNotFound(_) => {
                anyhow::anyhow!("Exercise '{identifier}' not found to edit.")
            }
            _ => anyhow::Error::new(db_err)
                .context(format!("Failed to update exercise '{identifier}'")),
        })
    }

    /// Deletes exercise definitions.
    /// # Errors
    /// Returns `anyhow::Error` if an identifier invalid or DB deletion fails.
    pub fn delete_exercise(&mut self, identifiers: &[String]) -> Result<u64> {
        let mut total_deleted: u64 = 0;
        for identifier in identifiers {
            let exercise_def = self
                .resolve_exercise_identifier(identifier)?
                .ok_or_else(|| DbError::ExerciseNotFound(identifier.clone()))?;
            let canonical_name = exercise_def.name;

            let workout_count: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND deleted = FALSE", // Only count active workouts
                    [&canonical_name],
                    |row| row.get(0),
                )
                .with_context(|| format!("Failed workout count check for '{canonical_name}'"))?;

            if workout_count > 0 {
                // This warning now pertains to *active* workouts. Soft-deleted workouts associated with this exercise will remain soft-deleted.
                eprintln!("Warning: Soft-deleting '{canonical_name}'. {workout_count} active associated workout(s) will remain (but will effectively be orphaned if not also deleted or reassigned). Consider deleting or reassigning them.");
            }

            let deleted_count =
                db::delete_exercise(&mut self.conn, &canonical_name).map_err(|e| match e {
                    DbError::ExerciseNotFound(_) => {
                        // This means it was already deleted or never existed in an active state.
                        anyhow::anyhow!("Exercise '{identifier}' not found or already deleted.")
                    }
                    _ => anyhow::Error::new(e)
                        .context(format!("Failed to soft-delete exercise '{canonical_name}'")),
                })?;
            total_deleted += deleted_count;
        }
        Ok(total_deleted)
    }

    /// Retrieves an exercise definition by identifier.
    /// # Errors
    /// Returns `anyhow::Error` if identifier empty or resolution fails.
    pub fn get_exercise_by_identifier_service(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
        self.resolve_exercise_identifier(identifier)
    }

    /// Lists exercise definitions based on filters.
    /// # Errors
    /// Returns `anyhow::Error` wrapping DB errors.
    pub fn list_exercises(
        &self,
        type_filter: Option<ExerciseType>,
        muscle_filter: Option<Vec<&str>>,
    ) -> Result<Vec<ExerciseDefinition>> {
        db::list_exercises(&self.conn, type_filter, muscle_filter)
            .context("Failed to list exercise definitions")
            .map_err(Into::into)
    }

    /// Creates a new alias for an exercise.
    /// # Errors
    /// Returns `anyhow::Error` if alias/identifier invalid or DB creation fails.
    pub fn create_alias(&self, alias_name: &str, exercise_identifier: &str) -> Result<()> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        // Check against active exercises/aliases
        if let Some((_, res_type)) = db::get_exercise_by_identifier(&self.conn, trimmed_alias)? {
            match res_type {
                ResolvedByType::Id => {
                    bail!("Alias '{trimmed_alias}' conflicts with an existing exercise ID.")
                }
                ResolvedByType::Name => {
                    bail!("Alias '{trimmed_alias}' conflicts with an existing exercise name.")
                }
                ResolvedByType::Alias => {} // If it's an alias for something else, create_alias in db will fail if name taken
            }
        }
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        println!("deleted2");
        db::create_alias(&self.conn, trimmed_alias, &canonical_name).map_err(
            |db_err| match db_err {
                DbError::AliasAlreadyExists(_) => anyhow::anyhow!(db_err), // Will trigger if alias name (PK) is taken, even if soft-deleted
                _ => anyhow::Error::new(db_err)
                    .context(format!("Failed to create alias '{trimmed_alias}'")),
            },
        )
    }

    /// Deletes an exercise alias.
    /// # Errors
    /// Returns `anyhow::Error` if alias name empty or DB deletion fails.
    pub fn delete_alias(&self, alias_name: &str) -> Result<u64> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        db::delete_alias(&self.conn, trimmed_alias).map_err(|db_err| match db_err {
            DbError::AliasNotFound(_) => anyhow::anyhow!(db_err), // Not found or already deleted
            _ => anyhow::Error::new(db_err)
                .context(format!("Failed to soft-delete alias '{trimmed_alias}'")),
        })
    }

    /// Lists all defined aliases.
    /// # Errors
    /// Returns `anyhow::Error` wrapping DB errors.
    pub fn list_aliases(&self) -> Result<HashMap<String, String>> {
        db::list_aliases(&self.conn)
            .context("Failed to list aliases")
            .map_err(Into::into)
    }

    /// Adds a workout entry.
    /// # Returns
    /// A `Result` containing `(workout_id, Option<PBInfo>)`.
    /// # Errors
    /// Returns `anyhow::Error` if exercise invalid, bodyweight needed but missing,
    /// logging restricted metrics, or DB add fails.
    pub fn add_workout(&mut self, params: AddWorkoutParams) -> Result<(i64, Option<PBInfo>)> {
        let exercise_def = self.resolve_or_create_exercise(
            params.exercise_identifier,
            params.implicit_type,
            params.implicit_muscles,
        )?;
        let canonical_exercise_name = &exercise_def.name;
        println!("here");

        let mut violations = vec![];

        if !exercise_def.log_weight && params.weight.is_some() {
            violations.push("weight");
        }
        if !exercise_def.log_reps && params.reps.is_some() {
            violations.push("reps");
        }
        if !exercise_def.log_duration && params.duration.is_some() {
            violations.push("duration");
        }
        if !exercise_def.log_distance && params.distance.is_some() {
            violations.push("distance");
        }

        if !violations.is_empty() {
            let violation_msg = violations.join(", ");
            bail!(
                "Exercise '{}' is not configured to log the following: {}. Use 'edit exercise {}' to change its logging settings.",
                canonical_exercise_name,
                violation_msg,
                canonical_exercise_name 
            );
        }
        
        if exercise_def.type_ == ExerciseType::BodyWeight && params.bodyweight_to_use.is_none() {
            bail!(
                 "Bodyweight log required for BodyWeight exercise '{}'. Use 'log-bw <weight>' or ensure a recent log exists and is passed to the command.",
                 canonical_exercise_name
             );
        }
        
        let additional_weight_for_db = params.weight;
        let bodyweight_for_db = params.bodyweight_to_use; 

        let timestamp = params.date;
        let previous_bests = self.get_previous_bests(canonical_exercise_name)?;

        let workout_data = NewWorkoutData {
            exercise_name: canonical_exercise_name,
            timestamp,
            sets: params.sets,
            weight: additional_weight_for_db, 
            bodyweight_to_use: bodyweight_for_db, 
            reps: params.reps,
            duration: params.duration,
            distance: self.convert_distance_input_to_km(params.distance), 
            notes: params.notes.as_deref(),
        };

        let inserted_id = self.insert_workout_record(&workout_data)?;
        
        let effective_weight_for_pb_check = calculate_effective_weight(
            &exercise_def,
            additional_weight_for_db, 
            bodyweight_for_db,        
        );

        let pb_info = self.check_for_new_pbs(
            &previous_bests,
            effective_weight_for_pb_check,
            params.reps,
            params.duration,
            self.convert_distance_input_to_km(params.distance), 
        );

        Ok((inserted_id, pb_info))
    }

    fn resolve_or_create_exercise(
        &self,
        identifier: &str,
        imp_type: Option<ExerciseType>,
        imp_muscles: Option<String>,
    ) -> Result<ExerciseDefinition> {
        if let Some(def) = self.resolve_exercise_identifier(identifier)? { // resolve_exercise_identifier only gets active
            Ok(def)
        } else if let (Some(ex_type), Some(muscles)) = (imp_type, imp_muscles) {
            // At this point, exercise is not active. It might be soft-deleted or not exist.
            // db::create_exercise will fail if the name is taken by a soft-deleted exercise due to UNIQUE constraint.
            println!("Exercise '{identifier}' not found, defining implicitly...");
            let muscles_opt = if muscles.trim().is_empty() {
                None
            } else {
                Some(muscles.as_str())
            };
            match self.create_exercise(identifier, ex_type, None, muscles_opt) {
                Ok(id) => {
                    println!("Implicitly defined '{identifier}' (ID: {id})");
                    self.resolve_exercise_identifier(identifier)? // Re-fetch the now active exercise
                        .ok_or_else(|| {
                            anyhow::anyhow!("Failed to re-fetch implicitly created '{identifier}'")
                        })
                }
                Err(e) => { // Could be DbError::ExerciseNameNotUnique if name taken by soft-deleted
                    Err(e).context(format!("Implicit definition failed for '{identifier}' (name might be taken by a soft-deleted exercise)"))
                }
            }
        } else {
            bail!(
                "Exercise '{identifier}' not found. Define it first or provide --type/--muscles."
            );
        }
    }

    fn convert_distance_input_to_km(&self, dist_arg: Option<f64>) -> Option<f64> {
        dist_arg.map(|d| match self.config.units {
            Units::Metric => d,
            Units::Imperial => d * MILE_TO_KM,
        })
    }

    fn get_previous_bests(&self, name: &str) -> Result<PreviousBests> {
        Ok(PreviousBests {
            weight: db::get_max_effective_weight_for_exercise(&self.conn, name)?,
            reps: db::get_max_reps_for_exercise(&self.conn, name)?,
            duration: db::get_max_duration_for_exercise(&self.conn, name)?,
            distance_km: db::get_max_distance_for_exercise(&self.conn, name)?,
        })
    }

    fn insert_workout_record(&self, data: &NewWorkoutData) -> Result<i64> {
        db::add_workout(&self.conn, data).map_err(Into::into)
    }

    fn check_for_new_pbs(
        &self,
        prev: &PreviousBests,
        cur_w: Option<f64>,
        cur_r: Option<i64>,
        cur_d: Option<i64>,
        cur_dist: Option<f64>,
    ) -> Option<PBInfo> {
        if prev.no_records() {
            return None;
        }
        let mut pb = PBInfo {
            weight: PbMetricInfo {
                previous_value: prev.weight,
                new_value: cur_w,
                ..Default::default()
            },
            reps: PbMetricInfo {
                previous_value: prev.reps,
                new_value: cur_r,
                ..Default::default()
            },
            duration: PbMetricInfo {
                previous_value: prev.duration,
                new_value: cur_d,
                ..Default::default()
            },
            distance: PbMetricInfo {
                previous_value: prev.distance_km,
                new_value: cur_dist,
                ..Default::default()
            },
        };
        let cfg = &self.config.pb_notifications;

        if cfg.notify_weight && cur_w.map_or(false, |w| w > 0.0 && w > prev.weight.unwrap_or(0.0)) {
            pb.weight.achieved = true;
        }
        if cfg.notify_reps && cur_r.map_or(false, |r| r > 0 && r > prev.reps.unwrap_or(0)) {
            pb.reps.achieved = true;
        }
        if cfg.notify_duration && cur_d.map_or(false, |d| d > 0 && d > prev.duration.unwrap_or(0)) {
            pb.duration.achieved = true;
        }
        if cfg.notify_distance
            && cur_dist.map_or(false, |d| d > 0.0 && d > prev.distance_km.unwrap_or(0.0))
        {
            pb.distance.achieved = true;
        }

        if pb.any_pb() {
            Some(pb)
        } else {
            None
        }
    }

    /// Edits an existing workout entry.
    /// # Errors
    /// Returns `anyhow::Error` if identifier/id invalid or DB update fails.
    pub fn edit_workout(&self, params: EditWorkoutParams) -> Result<u64> {
        let new_canonical_name = params
            .new_exercise_identifier
            .map(|ident| -> Result<String> {
                self.resolve_identifier_to_canonical_name(&ident)? // Resolves active exercise
                    .ok_or_else(|| DbError::ExerciseNotFound(ident).into()) 
            })
            .transpose()?; 

        let new_timestamp = params
            .new_date
            .map(create_timestamp_from_date)
            .transpose()?;
        let new_distance_km = self.convert_distance_input_to_km(params.new_distance_arg);

        let workout_updates = Workout {
            id: params.id,
            sets: params.new_sets,
            reps: params.new_reps,
            weight: params.new_weight,
            duration_minutes: params.new_duration,
            bodyweight: params.new_bodyweight,
            distance: new_distance_km,
            notes: params.new_notes,
            timestamp: Utc::now(),
            exercise_name: String::new(),
            exercise_type: None, 
            deleted: false,
            last_edited: Utc::now()
        };

        db::update_workout(
            &self.conn,
            workout_updates,
            new_canonical_name,
            new_timestamp,
        )
        .with_context(|| format!("Failed to update workout ID {}", params.id))
        .map_err(Into::into) 
    }

    /// Deletes workout entries by IDs.
    /// # Errors
    /// Returns `anyhow::Error` if any ID invalid or DB deletion fails.
    pub fn delete_workouts(&self, ids: &[i64]) -> Result<Vec<i64>> {
        let mut deleted_ids = Vec::with_capacity(ids.len());
        for &id in ids {
            db::delete_workout(&self.conn, id).map_err(|db_err| match db_err {
                DbError::WorkoutNotFound(_) => anyhow::anyhow!(db_err), // Not found or already deleted
                _ => {
                    anyhow::Error::new(db_err).context(format!("Failed to soft-delete workout ID {id}"))
                }
            })?;
            deleted_ids.push(id);
        }
        Ok(deleted_ids)
    }

    /// Lists workouts based on filters.
    /// # Errors
    /// Returns `anyhow::Error` if identifier invalid or DB list fails.
    pub fn list_workouts(&self, filters: &WorkoutFilters) -> Result<Vec<Workout>> {
        let canonical_exercise_name = filters
            .exercise_name
            .map(|ident| -> Result<String> {
                self.resolve_identifier_to_canonical_name(ident)? // Resolves active exercise
                    .ok_or_else(|| {
                        eprintln!(
                            "Warning: Exercise identifier '{ident}' not found (or is soft-deleted) for filtering."
                        );
                        DbError::ExerciseNotFound(ident.to_string()).into() 
                    })
            })
            .transpose()?; 

        let resolved_filters = WorkoutFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            date: filters.date,
            exercise_type: filters.exercise_type,
            muscle: filters.muscle,
            limit: filters.limit,
        };

        db::list_workouts_filtered(&self.conn, &resolved_filters)
            .context("Failed to list workouts")
            .map_err(Into::into) 
    }

    /// Lists workouts for the Nth most recent day an exercise was performed.
    /// # Arguments
    /// * `n` - Must be > 0.
    /// # Errors
    /// Returns `anyhow::Error` if identifier invalid, n=0, or DB list fails.
    pub fn list_workouts_for_exercise_on_nth_last_day(
        &self,
        identifier: &str,
        n: u32,
    ) -> Result<Vec<Workout>> {
        let canonical_name = self
            .resolve_identifier_to_canonical_name(identifier)? // Resolves active exercise
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        db::list_workouts_for_exercise_on_nth_last_day(&self.conn, &canonical_name, n)
            .with_context(|| format!("Failed nth day lookup for '{canonical_name}' (n={n})"))
            .map_err(Into::into)
    }

    /// Retrieves a list of unique dates ("YYYY-MM-DD") within a given month and year
    /// that have at least one workout recorded. Delegates to db module.
    /// # Errors
    /// Returns `anyhow::Error` if the database query fails or month is invalid.
    pub fn get_workout_dates_for_month(&self, year: i32, month: u32) -> Result<Vec<String>> {
        if !(1..=12).contains(&month) {
            bail!("Invalid month: {}. Month must be between 1 and 12.", month);
        }

        db::get_workout_dates_for_month_db(&self.conn, year, month)
            .with_context(|| {
                format!("Failed to get workout dates for {year}-{month:02} from database")
            })
            .map_err(Into::into) 
    }

    /// Calculates and returns statistics for an exercise.
    /// # Errors
    /// Returns `anyhow::Error` if identifier invalid or DB query fails.
    /// # Panics
    /// See `calculate_streaks` potential panic.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn get_exercise_stats(&self, identifier: &str) -> Result<ExerciseStats> {
        let canonical_name = self
            .resolve_identifier_to_canonical_name(identifier)? // Resolves active exercise
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let timestamps = db::get_workout_timestamps_for_exercise(&self.conn, &canonical_name)
            .context(format!("Failed history retrieval for '{canonical_name}'"))?;
        if timestamps.is_empty() {
            bail!(DbError::NoWorkoutDataFound(canonical_name));
        }

        let first_ts = timestamps.first().expect("Timestamps non-empty");
        let last_ts = timestamps.last().expect("Timestamps non-empty");

        let avg_workouts_per_week = if timestamps.len() <= 1 {
            None
        } else {
            let duration_days = (*last_ts - *first_ts).num_days();
            if duration_days <= 0 {
                None
            } else {
                let duration_weeks = (duration_days as f64 / 7.0).max(1.0 / 7.0);
                Some(timestamps.len() as f64 / duration_weeks)
            }
        };

        let longest_gap_days: Option<u64> = if timestamps.len() > 1 {
            timestamps
                .windows(2)
                .map(|w| (w[1].date_naive() - w[0].date_naive()).num_days() - 1)
                .filter(|&g| g >= 0)
                .max()
                .map(|g| g as u64)
        } else {
            None
        };

        let streak_interval = Duration::days(i64::from(self.config.streak_interval_days));
        let (current_streak, longest_streak) = calculate_streaks(&timestamps, streak_interval);

        let personal_bests = PersonalBests {
            max_weight: db::get_max_effective_weight_for_exercise(&self.conn, &canonical_name)?,
            max_reps: db::get_max_reps_for_exercise(&self.conn, &canonical_name)?,
            max_duration_minutes: db::get_max_duration_for_exercise(&self.conn, &canonical_name)?,
            max_distance_km: db::get_max_distance_for_exercise(&self.conn, &canonical_name)?,
        };

        Ok(ExerciseStats {
            canonical_name,
            total_workouts: timestamps.len(),
            first_workout_date: Some(first_ts.date_naive()),
            last_workout_date: Some(last_ts.date_naive()),
            avg_workouts_per_week,
            longest_gap_days,
            personal_bests,
            current_streak,
            longest_streak,
            streak_interval_days: self.config.streak_interval_days,
        })
    }

    /// Calculates workout volume based on filters.
    /// # Errors
    /// Returns `anyhow::Error` if identifier invalid or DB query fails.
    pub fn calculate_daily_volume(
        &self,
        filters: &VolumeFilters,
    ) -> Result<Vec<(NaiveDate, String, f64)>> {
        let canonical_exercise_name = filters
            .exercise_name
            .map(|ident| -> Result<String> {
                self.resolve_identifier_to_canonical_name(ident)? // Resolves active exercise
                    .ok_or_else(|| {
                        eprintln!(
                            "Warning: Exercise identifier '{ident}' not found (or is soft-deleted) for volume filter."
                        );
                        DbError::ExerciseNotFound(ident.to_string()).into() 
                    })
            })
            .transpose()?; 

        let resolved_filters = VolumeFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            start_date: filters.start_date,
            end_date: filters.end_date,
            exercise_type: filters.exercise_type,
            muscle: filters.muscle,
            limit_days: filters.limit_days,
        };

        db::calculate_daily_volume_filtered(&self.conn, &resolved_filters)
            .context("Failed to calculate workout volume")
            .map_err(Into::into) 
    }

    /// Gets a list of all unique dates with recorded workouts.
    /// # Errors
    /// Returns `DbError` variants if the query fails.
    pub fn get_all_dates_with_exercise(&self) -> Result<Vec<NaiveDate>, DbError> {
        db::get_all_dates_with_exercise(&self.conn)
    }

    /// Fetches and processes workout data for plotting, aggregated daily.
    ///
    /// Data is filtered by exercise identifier and an optional date range.
    /// The specific metric (e.g., max weight, total volume) is determined by `graph_type`.
    /// For `GraphType::WorkoutDistance`, the returned distance values are converted
    /// to the unit specified in the application configuration (`config.units`).
    /// All other metrics are returned as recorded or calculated (e.g., E1RM).
    ///
    /// # Arguments
    ///
    /// * `identifier` - The exercise identifier (name, alias, or ID as string) to fetch data for.
    /// * `graph_type` - The type of metric to calculate and return for each day.
    /// * `start_date_filter` - Optional start date (inclusive) for filtering workouts.
    /// * `end_date_filter` - Optional end date (inclusive) for filtering workouts.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<(NaiveDate, f64)>`.
    /// Each tuple represents `(date, aggregated_metric_value)`.
    /// - The `NaiveDate` is the date of the workout(s).
    /// - The `f64` is the aggregated value for that date based on the `graph_type`.
    ///   - For `Estimated1RM`, `MaxWeight`, `MaxReps`: the maximum value achieved on that day.
    ///   - For `WorkoutVolume`, `WorkoutReps`, `WorkoutDuration`, `WorkoutDistance`: the sum of values for that day.
    ///
    /// Returns an empty vector if no workouts match the criteria or if all aggregated values are non-positive.
    ///
    /// # Errors
    ///
    /// Returns `anyhow::Error` if:
    /// - The `identifier` cannot be resolved to a valid exercise.
    /// - There's an issue querying the database.
    /// - The `exercise_definition` for the resolved exercise cannot be found (should be rare if identifier resolves).
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    pub fn get_data_for_graph(
        &self,
        identifier: &str,
        graph_type: GraphType,
        start_date_filter: Option<NaiveDate>, 
        end_date_filter: Option<NaiveDate>,   
    ) -> Result<Vec<(NaiveDate, f64)>> {
        let canonical_name = self
            .resolve_identifier_to_canonical_name(identifier)? // Resolves active exercise
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;

        // Fetches active exercise definition
        let exercise_definition = db::get_exercise_by_identifier(&self.conn, &canonical_name)?
            .map(|(def, _)| def)
            .ok_or_else(|| DbError::ExerciseNotFound(canonical_name.clone()))?;

        let base_filter = WorkoutFilters {
            exercise_name: Some(&canonical_name),
            ..Default::default()
        };

        // list_workouts already filters for active workouts and active exercises
        let mut history = self
            .list_workouts(&base_filter) 
            .context(format!("Failed graph data fetch for '{canonical_name}'"))?;

        if let Some(start_date) = start_date_filter {
            history.retain(|w| w.timestamp.date_naive() >= start_date);
        }
        if let Some(end_date) = end_date_filter {
            history.retain(|w| w.timestamp.date_naive() <= end_date);
        }
        history.sort_by_key(|w| w.timestamp);

        if history.is_empty() {
            return Ok(vec![]);
        }

        let mut daily_aggregated_data: BTreeMap<NaiveDate, f64> = BTreeMap::new();
        for w in history { // w is a non-deleted workout for a non-deleted exercise
            let date = w.timestamp.date_naive();
            let entry = daily_aggregated_data.entry(date).or_insert(0.0);

            match graph_type {
                GraphType::Estimated1RM => {
                    if let (Some(wt), Some(r)) = (w.weight, w.reps) {
                        if let Some(e1rm) = calculate_e1rm(wt, r) {
                            *entry = entry.max(e1rm);
                        }
                    }
                }
                GraphType::MaxWeight => {
                    let effective_weight =
                        calculate_effective_weight(&exercise_definition, w.weight, w.bodyweight);
                    if let Some(wt) = effective_weight.filter(|&wg| wg > 0.0) {
                        *entry = entry.max(wt);
                    }
                }
                GraphType::MaxReps => {
                    if let Some(r) = w.reps.filter(|&rp| rp > 0) {
                        *entry = entry.max(r as f64);
                    }
                }
                GraphType::WorkoutVolume => {
                    let s = w.sets.unwrap_or(1).max(1);
                    let r = w.reps.unwrap_or(0);

                    let weight_for_volume = match exercise_definition.type_ {
                        ExerciseType::BodyWeight => {
                            calculate_effective_weight(&exercise_definition, w.weight, w.bodyweight)
                                .unwrap_or(0.0)
                        }
                        _ => w.weight.unwrap_or(0.0),
                    };

                    let v = s as f64 * r as f64 * weight_for_volume;
                    if v > 0.0 {
                        *entry += v;
                    }
                }
                GraphType::WorkoutReps => {
                    let s = w.sets.unwrap_or(1).max(1);
                    let r = w.reps.unwrap_or(0);
                    let tr = s * r;
                    if tr > 0 {
                        *entry += tr as f64;
                    }
                }
                GraphType::WorkoutDuration => {
                    if let Some(d) = w.duration_minutes.filter(|&dur| dur > 0) {
                        *entry += d as f64;
                    }
                }
                GraphType::WorkoutDistance => {
                    if let Some(d) = w.distance.filter(|&dist| dist > 0.0) {
                        *entry += d; 
                    }
                }
            }
        }

        let data_points: Vec<(NaiveDate, f64)> = daily_aggregated_data
            .into_iter()
            .filter_map(|(date, value)| {
                if value <= 0.0 {
                    return None;
                }

                let final_val = if graph_type == GraphType::WorkoutDistance {
                    match self.config.units {
                        Units::Metric => value, 
                        Units::Imperial => value * KM_TO_MILE,
                    }
                } else {
                    value
                };
                Some((date, final_val))
            })
            .collect();

        Ok(data_points)
    }

    /// Lists all unique muscle groups.
    /// # Errors
    /// Returns `anyhow::Error` wrapping DB errors.
    pub fn list_all_muscles(&self) -> Result<Vec<String>> {
        db::list_all_muscles(&self.conn)
            .context("Failed to list all muscles")
            .map_err(Into::into)
    }

    pub fn get_last_sync_timestamp(&self) -> Option<DateTime<Utc>> {
        self.config.last_sync_timestamp
    }

    pub fn set_last_sync_timestamp(&mut self, ts: DateTime<Utc>) -> Result<(), ConfigError> {
        self.config.last_sync_timestamp = Some(ts);
        self.save_config()
    }

    pub fn get_server_url(&self, server_url_override: Option<String>) -> Result<String> {
        server_url_override
            .or_else(|| self.config.sync_server_url.clone())
            .ok_or_else(|| anyhow::anyhow!("Sync server URL not configured and no override provided."))
    }

    pub fn set_sync_server_url(&mut self, url: Option<String>) -> Result<(), ConfigError> {
        self.config.sync_server_url = url.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.save_config()
    }

    pub fn collect_local_changes(&self, since: Option<DateTime<Utc>>) -> Result<ChangesPayload> {
        let config_path = self.get_config_path();
        let current_config_content = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file for sync: {:?}", config_path))?;

        let config_file_metadata = std::fs::metadata(config_path)
            .with_context(|| format!("Failed to get metadata for config file: {:?}", config_path))?;
        let config_mod_time: DateTime<Utc> = config_file_metadata.modified()
            .context("Failed to get modification time for config file")?.into();

        let config_change = if since.map_or(true, |s_ts| config_mod_time > s_ts) {
            Some(ConfigChange {
                content: current_config_content,
                last_edited: config_mod_time,
            })
        } else {
            None
        };

        Ok(ChangesPayload {
            config: config_change,
            exercises: db::get_exercises_modified_since(&self.conn, since)
                .context("Failed to get modified exercises for sync")?,
            workouts: db::get_workouts_modified_since(&self.conn, since)
                .context("Failed to get modified workouts for sync")?,
            aliases: db::get_aliases_modified_since(&self.conn, since)
                .context("Failed to get modified aliases for sync")?,
            bodyweights: db::get_bodyweights_modified_since(&self.conn, since)
                .context("Failed to get modified bodyweights for sync")?,
        })
    }

    pub fn apply_server_changes(&mut self, changes: ChangesPayload) -> Result<SyncSummary> {
        let mut summary = SyncSummary::default();
        let local_config_path = self.get_config_path().to_path_buf();
        let tx = self.conn.transaction().context("Failed to start transaction for applying server changes")?;

        if let Some(server_config_change) = changes.config {
            let local_config_mod_time: DateTime<Utc> = std::fs::metadata(&local_config_path)
                .and_then(|m| m.modified())
                .map(Into::into)
                .unwrap_or_else(|_| Utc.timestamp_opt(0,0).unwrap());

            if server_config_change.last_edited > local_config_mod_time {
                let new_config_from_server: Config = toml::from_str(&server_config_change.content)
                    .context("Failed to parse server config content")?;
                
                let old_last_sync = self.config.last_sync_timestamp;
                self.config = new_config_from_server;
                self.config.last_sync_timestamp = old_last_sync;

                config::save(&local_config_path, &self.config).context("Failed to save synced config")?;
                println!("Applied server config changes.");
                summary.config = true;
            } else {
                 println!("Server config not newer or local file missing, skipped applying server config. Server last_edited: {}, Local approx last_edited: {}", server_config_change.last_edited, local_config_mod_time);
            }
        }

        for exercise_def in changes.exercises {
            db::upsert_exercise(&tx, &exercise_def).context(format!("Failed to upsert exercise ID {}", exercise_def.id))?;
            summary.exercises += 1;
        }
        for workout in changes.workouts {
            db::upsert_workout(&tx, &workout).context(format!("Failed to upsert workout ID {}", workout.id))?;
            summary.workouts += 1;
        }
        for alias_entry in changes.aliases {
            db::upsert_alias(&tx, &alias_entry).context(format!("Failed to upsert alias '{}'", alias_entry.alias_name))?;
            summary.aliases += 1;
        }
        for bw_entry in changes.bodyweights {
            db::upsert_bodyweight_entry(&tx, &bw_entry).context(format!("Failed to upsert bodyweight entry ID {}", bw_entry.id))?;
            summary.bodyweights += 1;
        }
        
        tx.commit().context("Failed to commit transaction for server changes")?;
        Ok(summary)
    }
}

// --- Helper Functions ---

#[allow(clippy::cast_precision_loss)]
fn calculate_e1rm(weight: f64, reps: i64) -> Option<f64> {
    if reps > 0 && weight > 0.0 {
        Some(weight * (1.0 + (reps as f64 / 30.0)))
    } else {
        None
    }
}

/// Creates a UTC timestamp representing noon on the given date.
/// # Errors
/// Returns `anyhow::Error` if date components invalid.
fn create_timestamp_from_date(date: NaiveDate) -> Result<DateTime<Utc>> {
    let naive_dt = date
        .and_hms_opt(12, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid date components: {date}"))?;
    Ok(Utc.from_utc_datetime(&naive_dt))
}

/// Calculates current and longest streaks.
/// # Panics
/// Can panic if `timestamps` contains non-sensical dates leading to negative durations
/// in `num_days`, although this is unlikely with chrono dates.
fn calculate_streaks(timestamps: &[DateTime<Utc>], interval: Duration) -> (u32, u32) {
    if timestamps.is_empty() {
        return (0, 0);
    }
    let mut current = 0u32;
    let mut longest = 0u32;
    let mut last_date = timestamps[0].date_naive();

    for ts in timestamps {
        let cur_date = ts.date_naive();
        if current > 0 && cur_date == last_date {
            continue;
        } 
        if current == 0 || cur_date - last_date <= interval {
            if current == 0 {
                current = 1;
            }
            else if cur_date > last_date {
                current += 1;
            } 
        } else {
            current = 1;
        } 
        last_date = cur_date;
        longest = longest.max(current);
    }
    if timestamps.last().map_or(true, |last| {
        Utc::now().date_naive() - last.date_naive() > interval
    }) {
        current = 0;
    }
    (current, longest)
}

/// Calculates the effective weight (additional + bodyweight) for calculations.
fn calculate_effective_weight(
    ex_def: &ExerciseDefinition,
    additional_weight: Option<f64>,
    stored_bodyweight: Option<f64>,
) -> Option<f64> {
    match ex_def.type_ {
        ExerciseType::BodyWeight => {
            Some(additional_weight.unwrap_or(0.0) + stored_bodyweight.unwrap_or(0.0))
        }
        _ => additional_weight, 
    }
}

