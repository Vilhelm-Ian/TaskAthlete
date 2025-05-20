// src/db.rs
use anyhow::Result as AnyhowResult; // Use AnyhowResult alias where needed to avoid conflict
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error as StdError; // Use alias for standard Error trait
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

// Renamed from DbError to avoid repetition
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database connection failed: {0}")]
    Connection(#[from] rusqlite::Error),
    #[error("Failed to get application data directory")]
    DataDir,
    #[error("I/O error accessing database file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Bodyweight entry already exists for this timestamp: {0}")]
    BodyweightEntryExists(String),
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
    #[error("Alias not found: {0}")]
    AliasNotFound(String),
    #[error("Alias already exists: {0}")]
    AliasAlreadyExists(String),
    #[error("Exercise name must be unique (case-insensitive): '{0}' already exists.")]
    ExerciseNameNotUnique(String),
    #[error("No workout data found for exercise '{0}'")]
    NoWorkoutDataFound(String),
    #[error("BodyWeight Entry not found '{0}'")]
    BodyWeightEntryNotFound(i64),
    #[error("Invalid parameter count: expected > {1}, got {0}")] // Adjusted message slightly
    InvalidParameterCount(usize, usize),
    #[error("Invalid data conversion: {0}")]
    Conversion(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Eq, Clone, Copy)]
pub enum ExerciseType {
    Resistance,
    Cardio,
    BodyWeight,
}

// Convert string from DB to ExerciseType
impl TryFrom<&str> for ExerciseType {
    type Error = anyhow::Error; // Keep anyhow here as it's internal conversion detail

    fn try_from(value: &str) -> AnyhowResult<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "resistance" => Ok(Self::Resistance),
            "cardio" => Ok(Self::Cardio),
            "body-weight" | "bodyweight" | "bw" => Ok(Self::BodyWeight),
            _ => anyhow::bail!("Invalid exercise type string from DB: {value}"),
        }
    }
}

// Convert ExerciseType to string for DB storage
impl fmt::Display for ExerciseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resistance => write!(f, "resistance"),
            Self::Cardio => write!(f, "cardio"),
            Self::BodyWeight => write!(f, "body-weight"),
        }
    }
}

#[derive(Default, Debug)]
pub struct VolumeFilters<'a> {
    pub exercise_name: Option<&'a str>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit_days: Option<u32>,
}

/// Calculates the daily volume (sets * reps * weight) for exercises matching the filters.
///
/// Volume is calculated only for `Resistance` and `BodyWeight` exercises.
/// Results are ordered by date descending, then exercise name ascending.
///
/// # Arguments
///
/// * `conn` - A reference to the database connection.
/// * `filters` - A reference to the `VolumeFilters` specifying which workouts to include.
///
/// # Returns
///
/// A `Result` containing a vector of tuples `(NaiveDate, String, f64)`, where each tuple
/// represents the workout date, canonical exercise name, and calculated volume.
///
/// # Errors
///
/// Returns `Error::QueryFailed` if the database query fails.
/// Returns `Error::Conversion` if date parsing fails within the query mapping.
pub fn calculate_daily_volume_filtered(
    conn: &Connection,
    filters: &VolumeFilters,
) -> Result<Vec<(NaiveDate, String, f64)>, Error> {
    let mut sql = "
        SELECT
            date(w.timestamp) as workout_date,
            w.exercise_name,
            SUM(CASE e.type
                    WHEN 'resistance' THEN COALESCE(w.sets, 1) * COALESCE(w.reps, 0) * COALESCE(w.weight, 0)
                    WHEN 'body-weight' THEN COALESCE(w.sets, 1) * COALESCE(w.reps, 0) * (COALESCE(w.weight, 0) + COALESCE(w.bodyweight, 0))
                     ELSE 0
                END) as daily_volume
        FROM workouts w
        LEFT JOIN exercises e ON w.exercise_name = e.name
        WHERE 1=1"
        .to_string();

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(name) = filters.exercise_name {
        sql.push_str(" AND w.exercise_name = :ex_name COLLATE NOCASE");
        params_map.insert(":ex_name".into(), Box::new(name.to_string()));
    }
    if let Some(start) = filters.start_date {
        sql.push_str(" AND date(w.timestamp) >= date(:start_date)");
        params_map.insert(
            ":start_date".into(),
            Box::new(start.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(end) = filters.end_date {
        sql.push_str(" AND date(w.timestamp) <= date(:end_date)");
        params_map.insert(
            ":end_date".into(),
            Box::new(end.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(ex_type) = filters.exercise_type {
        sql.push_str(" AND e.type = :ex_type");
        params_map.insert(":ex_type".into(), Box::new(ex_type.to_string()));
    }
    if let Some(m) = filters.muscle {
        sql.push_str(" AND e.muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{m}%")));
    }

    sql.push_str(
        " GROUP BY workout_date, w.exercise_name ORDER BY workout_date DESC, w.exercise_name ASC",
    );

    if filters.start_date.is_none() && filters.end_date.is_none() {
        if let Some(limit) = filters.limit_days {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(Error::QueryFailed)?;
    let volume_iter = stmt
        .query_map(params_for_query.as_slice(), |row| {
            let date_str: String = row.get(0)?;
            // Fix: Map parse error to rusqlite::Error needed by closure
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(Error::Conversion(format!(
                        "Invalid date format '{date_str}': {e}"
                    ))) as Box<dyn StdError + Send + Sync>, // Explicit cast needed here
                )
            })?;
            let exercise_name: String = row.get(1)?;
            let volume: f64 = row.get(2)?;
            Ok((date, exercise_name, volume))
        })
        .map_err(Error::QueryFailed)?;

    // Fix: Adjust error handling after collect
    volume_iter.collect::<Result<Vec<_>, _>>().map_err(|e| {
        match e {
            rusqlite::Error::FromSqlConversionFailure(_, _, source) => {
                // Try to downcast the source error to our custom db::Error
                if let Some(db_error) = source.downcast_ref::<Error>() {
                    // If it's one of ours (likely Conversion), clone or convert its message
                    match db_error {
                        Error::Conversion(msg) => Error::Conversion(msg.clone()),
                        // Handle other potential db::Error variants if they could be boxed here
                        _ => Error::QueryFailed(rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            source,
                        )), // Re-wrap if needed
                    }
                } else {
                    // If downcast fails, it's some other conversion error, wrap it
                    Error::Conversion(format!("Unknown conversion error: {source}"))
                }
            }
            // Handle other rusqlite errors
            _ => Error::QueryFailed(e),
        }
    })
}

// --- Helper for handling collection errors ---
fn map_collect_error(e: rusqlite::Error) -> Error {
    match e {
        rusqlite::Error::FromSqlConversionFailure(_, _, source) => {
            if let Some(db_error) = source.downcast_ref::<Error>() {
                match db_error {
                    Error::Conversion(msg) => Error::Conversion(msg.clone()),
                    _ => Error::QueryFailed(rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        source,
                    )),
                }
            } else {
                Error::Conversion(format!("Unknown conversion error: {source}"))
            }
        }
        _ => Error::QueryFailed(e),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workout {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String, // Always the canonical name
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub bodyweight: Option<f64>,
    pub distance: Option<f64>,
    pub notes: Option<String>,
    pub exercise_type: Option<ExerciseType>, // Populated by JOIN
}

impl Workout {
    /// Calculates the effective weight (additional + bodyweight) for calculations.
    pub fn calculate_effective_weight(&self) -> Option<f64> {
        match self.exercise_type {
            Some(ExerciseType::BodyWeight) => {
                Some(self.weight.unwrap_or(0.0) + self.bodyweight.unwrap_or(0.0))
            }
            _ => self.weight, // For Resistance/Cardio, effective weight is just the additional weight
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExerciseDefinition {
    pub id: i64,
    pub name: String,
    pub type_: ExerciseType,
    pub muscles: Option<String>,
    pub log_weight: bool,
    pub log_reps: bool,
    pub log_duration: bool,
    pub log_distance: bool,
}

const DB_FILE_NAME: &str = "workouts.sqlite";

/// Gets the path to the `SQLite` database file within the app's data directory.
/// Exposed at crate root as `get_db_path_util`.
/// # Errors
/// - `Error::DataDir`: If the application data directory cannot be determined.
/// - `Error::Io`: If there's an error creating the application directory.
pub fn get_db_path() -> Result<PathBuf, Error> {
    #[cfg(target_os = "android")]
    {
        // On Android, just return the current directory joined with the config file name
    let path = PathBuf::from("/data/data/com.task_athlete_gui.app/files").join(DB_FILE_NAME);
    return Ok(path);
    }
    
    let data_dir = dirs::data_dir().ok_or(Error::DataDir)?;
    let app_dir = data_dir.join("workout-tracker-cli");
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }
    Ok(app_dir.join(DB_FILE_NAME))
}

/// Opens a connection to the `SQLite` database.
/// # Arguments
/// * `path` - A path-like object representing the location of the database file.
/// # Errors
/// - `Error::Connection`: If the database connection cannot be established.
pub fn open_db<P: AsRef<Path>>(path: P) -> Result<Connection, Error> {
    let conn = Connection::open(path).map_err(Error::Connection)?;
    Ok(conn)
}

/// Initializes the database tables if they don't exist.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// # Errors
/// - `Error::Connection`: If any SQL command fails during table/index creation or alteration.
pub fn init(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio', 'body-weight')),
            muscles TEXT,
            log_weight BOOLEAN NOT NULL DEFAULT TRUE,
            log_reps BOOLEAN NOT NULL DEFAULT TRUE,
            log_duration BOOLEAN NOT NULL DEFAULT TRUE,
            log_distance BOOLEAN NOT NULL DEFAULT TRUE

        );
        CREATE TABLE IF NOT EXISTS workouts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL, -- Store as RFC3339 string
            exercise_name TEXT NOT NULL COLLATE NOCASE,
            sets INTEGER,
            reps INTEGER,
            weight REAL,
            duration_minutes INTEGER,
            distance REAL,
            bodyweight REAL, -- Bodyweight at time of workout (for BW exercises)
            notes TEXT
        );
        CREATE TABLE IF NOT EXISTS aliases (
            alias_name TEXT PRIMARY KEY NOT NULL COLLATE NOCASE,
            exercise_name TEXT NOT NULL COLLATE NOCASE
        );
        CREATE TABLE IF NOT EXISTS bodyweights (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL UNIQUE,
            weight REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workouts_timestamp ON workouts(timestamp);
        CREATE INDEX IF NOT EXISTS idx_workouts_exercise_name ON workouts(exercise_name);
        CREATE INDEX IF NOT EXISTS idx_aliases_exercise_name ON aliases(exercise_name);
        CREATE INDEX IF NOT EXISTS idx_bodyweights_timestamp ON bodyweights(timestamp);
        COMMIT;",
    )?;

    // Add distance column separately for backward compatibility
    // Add columns separately for backward compatibility
    add_bodyweight_column_if_not_exists(conn)?;
    add_distance_column_if_not_exists(conn)?;
    // Add log flag columns to exercises table separately for backward compatibility
    // Use defaults consistent with the CREATE TABLE statement (1=TRUE, 0=FALSE)
    // Or choose defaults based on the most common expected behavior for existing exercises
    add_log_flag_column_if_not_exists(conn, "log_weight", 1)?; // Default TRUE
    add_log_flag_column_if_not_exists(conn, "log_reps", 1)?; // Default TRUE
    add_log_flag_column_if_not_exists(conn, "log_duration", 0)?; // Default FALSE
    add_log_flag_column_if_not_exists(conn, "log_distance", 0)?; // Default FALSE

    Ok(())
}

/// Adds the distance column to the workouts table if it doesn't exist.
fn add_distance_column_if_not_exists(conn: &Connection) -> Result<(), Error> {
    let mut stmt = conn.prepare("PRAGMA table_info(workouts)")?;
    let columns_exist = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col_res| col_res.map_or(false, |col| col == "distance"));

    if !columns_exist {
        println!("Adding 'distance' column to workouts table..."); // Inform user
        conn.execute("ALTER TABLE workouts ADD COLUMN distance REAL", [])?;
    }
    Ok(())
}

/// Adds the bodyweight column to the workouts table if it doesn't exist.
fn add_bodyweight_column_if_not_exists(conn: &Connection) -> Result<(), Error> {
    let mut stmt = conn.prepare("PRAGMA table_info(workouts)")?;
    let columns_exist = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col_res| col_res.map_or(false, |col| col == "bodyweight"));

    if !columns_exist {
        println!("Adding 'bodyweight' column to workouts table..."); // Inform user
        conn.execute("ALTER TABLE workouts ADD COLUMN bodyweight REAL", [])?;
    }
    Ok(())
}

pub struct NewWorkoutData<'a> {
    pub exercise_name: &'a str, // Canonical name
    pub timestamp: DateTime<Utc>,
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub bodyweight_to_use: Option<f64>, // BW at time of workout
    pub duration: Option<i64>,
    pub distance: Option<f64>,
    pub notes: Option<&'a str>, // Use &str
}

/// Adds a new workout entry to the database.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `data` - A reference to the `NewWorkoutData`.
/// # Returns
/// The `rowid` of the newly inserted workout.
/// # Errors
/// - `Error::InsertFailed`: If the SQL INSERT operation fails.
pub fn add_workout(conn: &Connection, data: &NewWorkoutData) -> Result<i64, Error> {
    let timestamp_str = data.timestamp.to_rfc3339();
    let sets_val = data.sets.unwrap_or(1);

    conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, distance, bodyweight, notes)
         VALUES (:ts, :ex_name, :sets, :reps, :weight, :duration, :distance, :bw, :notes)",

        named_params! {
            ":ts": timestamp_str,
            ":ex_name": data.exercise_name,
            ":sets": sets_val,
            ":reps": data.reps,
            // 'weight' is now only the *additional* weight
            ":weight": data.weight,
            ":duration": data.duration,
            ":distance": data.distance,
            ":bw": data.bodyweight_to_use,
            ":notes": data.notes,
        },
    ).map_err(Error::InsertFailed)?;
    Ok(conn.last_insert_rowid())
}

/// Updates an existing workout entry in the database by its ID.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `workout` - A `Workout` struct containing the new values. The `id` field identifies the workout.
/// * `new_name` - An optional new canonical exercise name.
/// * `new_timestamp` - An optional new timestamp.
/// # Returns
/// The number of rows affected (should be 1 if successful).
/// # Errors
/// - `Error::UpdateFailed`: If the SQL UPDATE operation fails.
/// - `Error::WorkoutNotFound`: If no workout with the given `workout.id` exists.
pub fn update_workout(
    conn: &Connection,
    workout: Workout,
    new_name: Option<String>,
    new_timestamp: Option<DateTime<Utc>>,
) -> Result<u64, Error> {
    let Workout {
        id,
        sets: new_sets,
        reps: new_reps,
        weight: new_weight,
        duration_minutes: new_duration,
        distance: new_distance,
        bodyweight: new_bodyweight, // Allow updating stored bodyweight
        notes: new_notes,
        ..
    } = workout;

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name {
        updates.push("exercise_name = :ex_name");
        params_map.insert(":ex_name".into(), Box::new(name));
    }
    if let Some(s) = new_sets {
        updates.push("sets = :sets");
        params_map.insert(":sets".into(), Box::new(s));
    }
    if let Some(r) = new_reps {
        updates.push("reps = :reps");
        params_map.insert(":reps".into(), Box::new(r));
    }
    if new_weight.is_some() {
        updates.push("weight = :weight");
        params_map.insert(":weight".into(), Box::new(new_weight));
    }
    if new_duration.is_some() {
        updates.push("duration_minutes = :duration");
        params_map.insert(":duration".into(), Box::new(new_duration));
    }
    if new_distance.is_some() {
        updates.push("distance = :distance");
        params_map.insert(":distance".into(), Box::new(new_distance));
    }
    if new_bodyweight.is_some() {
        updates.push("bodyweight = :bodyweight");
        params_map.insert(":distance".into(), Box::new(new_distance));
    }
    if new_notes.is_some() {
        updates.push("notes = :notes");
        params_map.insert(":notes".into(), Box::new(new_notes));
    }
    if let Some(ts) = new_timestamp {
        updates.push("timestamp = :ts");
        params_map.insert(":ts".into(), Box::new(ts.to_rfc3339()));
    }

    // if updates.is_empty() {
    //     return Ok(0);
    // }

    let sql = format!("UPDATE workouts SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));

    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = conn
        .execute(&sql, params_for_exec.as_slice())
        .map_err(Error::UpdateFailed)?;

    if rows_affected == 0 {
        Err(Error::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes a workout entry from the database by its ID.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `id` - The ID of the workout entry to delete.
/// # Returns
/// The number of rows affected (should be 1 if successful).
/// # Errors
/// - `Error::DeleteFailed`: If the SQL DELETE operation fails.
/// - `Error::WorkoutNotFound`: If no workout with the given `id` exists.
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64, Error> {
    let rows_affected = conn
        .execute("DELETE FROM workouts WHERE id = ?", params![id])
        .map_err(Error::DeleteFailed)?;
    if rows_affected == 0 {
        Err(Error::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
}

// Helper function to map a database row to a Workout struct
fn map_row_to_workout(row: &Row) -> Result<Workout, rusqlite::Error> {
    let timestamp_str: String = row.get("timestamp")?;
    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(Error::Conversion(format!(
                    "Invalid timestamp format '{timestamp_str}': {e}"
                ))) as Box<dyn StdError + Send + Sync>,
            )
        })?;

    let type_str_opt: Option<String> = row.get("type")?;
    let exercise_type = type_str_opt
        .map(|type_str| {
            ExerciseType::try_from(type_str.as_str()).map_err(|_e| {
                // Use _e as anyhow error isn't Send+Sync
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(Error::Conversion(format!(
                        "Invalid exercise type '{type_str}' from DB"
                    ))) as Box<dyn StdError + Send + Sync>,
                )
            })
        })
        .transpose()?; // Converts Option<Result<T, rusqlite::Error>> to Result<Option<T>, rusqlite::Error>

    Ok(Workout {
        id: row.get("id")?,
        timestamp,
        exercise_name: row.get("exercise_name")?,
        sets: row.get("sets")?,
        reps: row.get("reps")?,
        weight: row.get("weight")?,
        duration_minutes: row.get("duration_minutes")?,
        distance: row.get("distance")?,
        bodyweight: row.get("bodyweight")?,
        notes: row.get("notes")?,
        exercise_type,
    })
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
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `filters` - A reference to the `WorkoutFilters`.
/// # Returns
/// A `Result` containing a vector of `Workout` structs matching the criteria.
/// # Errors
/// - `Error::QueryFailed`: If the database query fails.
/// - `Error::Conversion`: If timestamp or exercise type conversion fails during row mapping.
pub fn list_workouts_filtered(
    conn: &Connection,
    filters: &WorkoutFilters,
) -> Result<Vec<Workout>, Error> {
    let mut sql = "SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.bodyweight, w.notes, e.type
                   FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(name) = filters.exercise_name {
        sql.push_str(" AND w.exercise_name = :ex_name COLLATE NOCASE");
        params_map.insert(":ex_name".into(), Box::new(name.to_string()));
    }
    if let Some(date) = filters.date {
        sql.push_str(" AND date(w.timestamp) = date(:date)");
        params_map.insert(
            ":date".into(),
            Box::new(date.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(ex_type) = filters.exercise_type {
        sql.push_str(" AND e.type = :ex_type");
        params_map.insert(":ex_type".into(), Box::new(ex_type.to_string()));
    }
    if let Some(m) = filters.muscle {
        sql.push_str(" AND e.muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{m}%")));
    }

    if filters.date.is_some() {
        sql.push_str(" ORDER BY w.timestamp ASC");
    } else {
        sql.push_str(" ORDER BY w.timestamp DESC");
    }

    if filters.date.is_none() {
        if let Some(limit) = filters.limit {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(Error::QueryFailed)?;
    let workout_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_workout)
        .map_err(Error::QueryFailed)?;

    // Use helper for collection error mapping
    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_collect_error)
}

/// Lists workouts for a specific exercise on the Nth most recent day it was done.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `exercise_name` - The canonical name of the exercise.
/// * `n` - Which occurrence (1 for most recent, etc.). Must be > 0.
/// # Returns
/// A `Result` containing a vector of `Workout` structs from the specified day.
/// # Errors
/// - `Error::InvalidParameterCount`: If `n` is 0.
/// - `Error::QueryFailed`: If the database query fails.
/// - `Error::Conversion`: If timestamp or exercise type conversion fails.
pub fn list_workouts_for_exercise_on_nth_last_day(
    conn: &Connection,
    exercise_name: &str,
    n: u32,
) -> Result<Vec<Workout>, Error> {
    if n == 0 {
        return Err(Error::InvalidParameterCount(0, 0)); // Expected > 0
    }
    let offset = n - 1;
    let sql = "WITH RankedDays AS (
                    SELECT DISTINCT date(timestamp) as workout_date
                    FROM workouts
                    WHERE exercise_name = :ex_name COLLATE NOCASE
                    ORDER BY workout_date DESC LIMIT 1 OFFSET :offset
                )
                SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.bodyweight, w.notes, e.type

                FROM workouts w
                LEFT JOIN exercises e ON w.exercise_name = e.name
                JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
                WHERE w.exercise_name = :ex_name COLLATE NOCASE
                ORDER BY w.timestamp ASC;";

    let mut stmt = conn.prepare(sql).map_err(Error::QueryFailed)?;
    let workout_iter = stmt
        .query_map(
            named_params! { ":ex_name": exercise_name, ":offset": offset },
            map_row_to_workout,
        )
        .map_err(Error::QueryFailed)?;

    // Use helper for collection error mapping
    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_collect_error)
}

/// Creates a new exercise definition.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `name` - The name of the new exercise.
/// * `ex_type` - The `ExerciseType`.
/// * `muscles` - An optional comma-separated string of muscles.
/// * `log_weight` - Optional override for logging weight (defaults based on type).
/// * `log_reps` - Optional override for logging reps (defaults based on type).
/// * `log_duration` - Optional override for logging duration (defaults based on type).
/// * `log_distance` - Optional override for logging distance (defaults based on type).
/// # Returns
/// The `rowid` of the newly inserted exercise definition.
/// # Errors
/// - `Error::InsertFailed`: If the SQL INSERT fails (other than uniqueness).
/// - `Error::ExerciseNameNotUnique`: If the name already exists (case-insensitive).
pub fn create_exercise(
    conn: &Connection,
    name: &str,
    ex_type: &ExerciseType,
    muscles: Option<&str>,
    log_weight: Option<bool>,
    log_reps: Option<bool>,
    log_duration: Option<bool>,
    log_distance: Option<bool>,
) -> Result<i64, Error> {
    let type_str = ex_type.to_string();

    // Determine default log flags based on type
    let (default_log_w, default_log_r, default_log_dur, default_log_dist) = match ex_type {
        ExerciseType::Resistance | ExerciseType::BodyWeight => (true, true, false, false),
        ExerciseType::Cardio => (false, false, true, true),
    };

    let final_log_w = log_weight.unwrap_or(default_log_w);
    let final_log_r = log_reps.unwrap_or(default_log_r);
    let final_log_dur = log_duration.unwrap_or(default_log_dur);
    let final_log_dist = log_distance.unwrap_or(default_log_dist);

    match conn.execute(
        "INSERT INTO exercises (name, type, muscles, log_weight, log_reps, log_duration, log_distance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            name,
            type_str,
            muscles,
            final_log_w,
            final_log_r,
            final_log_dur,
            final_log_dist
        ],
    ) {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(e) => {
            // Check if the error is specifically a unique constraint violation on the name column.
            // A generic ConstraintViolation could be for other reasons (like a NOT NULL constraint,
            // although the schema *should* allow NULL muscles, testing might reveal unexpected behavior).
            if let rusqlite::Error::SqliteFailure( // Pattern match on the error type
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::ConstraintViolation,
                    ..
                },
                Some(msg), // Get the attached message, which often contains details
            ) = &e // Borrow `e` to check its parts without consuming it
            {
                // Check if the message specifically mentions a unique constraint or the exercises.name column
                // SQLite messages for unique constraint are usually "UNIQUE constraint failed: table.column"
                let msg_lower = msg.to_lowercase();
                if msg_lower.contains("unique constraint failed") || msg_lower.contains("exercises.name") {
                     return Err(Error::ExerciseNameNotUnique(name.to_string()));
                }
                // If it's a different constraint violation (e.g., NOT NULL on muscles if the schema
                // was somehow misapplied or interpreted), fall through to the general InsertFailed.
            }
            // If it's not the unique name constraint, or not a ConstraintViolation at all,
            // map it to the general InsertFailed error.
            Err(Error::InsertFailed(e))
        }
    }
}

/// Updates an existing exercise definition. Handles updates to aliases and workouts if name changes.
/// # Arguments
/// * `conn` - A mutable reference to the database connection.
/// * `canonical_name_to_update` - The current canonical name.
/// * `new_name` - Optional new name.
/// * `new_type` - Optional new `ExerciseType`.
/// * `new_muscles` - Optional `Option<&str>` for muscles.
/// * `new_log_weight` - Optional flag.
/// * `new_log_reps` - Optional flag.
/// * `new_log_duration` - Optional flag.
/// * `new_log_distance` - Optional flag.
/// # Returns
/// The number of rows affected in `exercises` table (should be 1).
/// # Errors
/// - `Error::ExerciseNotFound`: If `canonical_name_to_update` doesn't exist.
/// - `Error::UpdateFailed`: If SQL UPDATE fails.
/// - `Error::ExerciseNameNotUnique`: If `new_name` conflicts.
/// - `Error::Connection`: If transaction fails.
#[allow(clippy::too_many_arguments)]
pub fn update_exercise(
    conn: &mut Connection,
    canonical_name_to_update: &str,
    new_name: Option<&str>,
    new_type: Option<&ExerciseType>,
    new_muscles: Option<Option<&str>>,
    new_log_weight: Option<bool>,
    new_log_reps: Option<bool>,
    new_log_duration: Option<bool>,
    new_log_distance: Option<bool>,
) -> Result<u64, Error> {
    let exercise = get_exercise_by_name(conn, canonical_name_to_update)?
        .ok_or_else(|| Error::ExerciseNotFound(canonical_name_to_update.to_string()))?;
    let id = exercise.id;
    let original_name = exercise.name;

    let name_being_changed = new_name.is_some() && new_name != Some(original_name.as_str());
    let target_new_name = new_name.unwrap_or(&original_name);

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name {
        updates.push("name = :name");
        params_map.insert(":name".into(), Box::new(name.to_string()));
    }
    if let Some(t) = new_type {
        updates.push("type = :type");
        params_map.insert(":type".into(), Box::new(t.to_string()));
    }
    if let Some(m_opt) = new_muscles {
        updates.push("muscles = :muscles");
        params_map.insert(":muscles".into(), Box::new(m_opt));
    }
    if let Some(b) = new_log_weight {
        updates.push("log_weight = :log_w");
        params_map.insert(":log_w".into(), Box::new(b));
    }
    if let Some(b) = new_log_reps {
        updates.push("log_reps = :log_r");
        params_map.insert(":log_r".into(), Box::new(b));
    }
    if let Some(b) = new_log_duration {
        updates.push("log_duration = :log_dur");
        params_map.insert(":log_dur".into(), Box::new(b));
    }
    if let Some(b) = new_log_distance {
        updates.push("log_distance = :log_dist");
        params_map.insert(":log_dist".into(), Box::new(b));
    }

    if updates.is_empty() {
        return Ok(0);
    }

    let tx = conn.transaction().map_err(Error::Connection)?;

    let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = match tx.execute(&sql_update_exercise, params_for_exec.as_slice()) {
        Ok(rows) => rows,
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::ConstraintViolation,
                    ..
                },
                _,
            ) = e
            {
                if name_being_changed {
                    return Err(Error::ExerciseNameNotUnique(target_new_name.to_string()));
                }
            }
            return Err(Error::UpdateFailed(e));
        }
    };

    if name_being_changed {
        tx.execute("UPDATE workouts SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE",
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(Error::UpdateFailed)?;
        tx.execute("UPDATE aliases SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE",
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(Error::UpdateFailed)?;
    }

    tx.commit().map_err(Error::Connection)?;

    if rows_affected == 0 {
        Err(Error::ExerciseNotFound(original_name))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes an exercise definition and its associated aliases.
/// # Arguments
/// * `conn` - A mutable reference to the database connection.
/// * `canonical_name` - The canonical name of the exercise to delete.
/// # Returns
/// The number of rows affected in `exercises` table (should be 1).
/// # Errors
/// - `Error::ExerciseNotFound`: If no exercise with `canonical_name` exists.
/// - `Error::DeleteFailed`: If SQL DELETE fails.
/// - `Error::Connection`: If transaction fails.
pub fn delete_exercise(conn: &mut Connection, canonical_name: &str) -> Result<u64, Error> {
    let exercise = get_exercise_by_name(conn, canonical_name)?
        .ok_or_else(|| Error::ExerciseNotFound(canonical_name.to_string()))?;
    let id = exercise.id;
    let name_to_delete = exercise.name;

    let tx = conn.transaction().map_err(Error::Connection)?;
    tx.execute(
        "DELETE FROM aliases WHERE exercise_name = ? COLLATE NOCASE",
        params![name_to_delete],
    )
    .map_err(Error::DeleteFailed)?;
    let rows_affected = tx
        .execute("DELETE FROM exercises WHERE id = ?", params![id])
        .map_err(Error::DeleteFailed)?;
    tx.commit().map_err(Error::Connection)?;

    if rows_affected == 0 {
        Err(Error::ExerciseNotFound(name_to_delete))
    } else {
        Ok(rows_affected as u64)
    }
}

// Helper to map row to ExerciseDefinition, handles type conversion error
fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
    let type_str: String = row.get("type")?;
    let ex_type = ExerciseType::try_from(type_str.as_str()).map_err(|_e| {
        // Use _e as anyhow error isn't Send+Sync
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(Error::Conversion(format!(
                "Invalid exercise type '{type_str}' from DB"
            ))) as Box<dyn StdError + Send + Sync>,
        )
    })?;

    Ok(ExerciseDefinition {
        id: row.get("id")?,
        name: row.get("name")?,
        type_: ex_type,
        muscles: row.get("muscles")?,
        log_weight: row.get("log_weight")?,
        log_reps: row.get("log_reps")?,
        log_duration: row.get("log_duration")?,
        log_distance: row.get("log_distance")?,
    })
}

/// Retrieves an exercise definition by its name (case-insensitive).
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `name` - The name of the exercise.
/// # Returns
/// An `Ok(Some(ExerciseDefinition))` if found, `Ok(None)` otherwise.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
/// - `Error::Conversion`: If exercise type conversion fails.
pub fn get_exercise_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<ExerciseDefinition>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, muscles, log_weight, log_reps, log_duration, log_distance
             FROM exercises WHERE name = ?1 COLLATE NOCASE",
        )
        .map_err(Error::QueryFailed)?;
    stmt.query_row(params![name], map_row_to_exercise_definition)
        .optional()
        .map_err(map_collect_error) // Use helper here too
}

/// Retrieves an exercise definition by its ID.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `id` - The ID of the exercise.
/// # Returns
/// An `Ok(Some(ExerciseDefinition))` if found, `Ok(None)` otherwise.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
/// - `Error::Conversion`: If exercise type conversion fails.
pub fn get_exercise_by_id(conn: &Connection, id: i64) -> Result<Option<ExerciseDefinition>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, type, muscles, log_weight, log_reps, log_duration, log_distance
             FROM exercises WHERE id = ?1",
        )
        .map_err(Error::QueryFailed)?;
    stmt.query_row(params![id], map_row_to_exercise_definition)
        .optional()
        .map_err(map_collect_error) // Use helper here too
}

/// Creates a new alias for a given canonical exercise name.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `alias_name` - The name of the alias.
/// * `canonical_exercise_name` - The canonical name of the exercise.
/// # Errors
/// - `Error::InsertFailed`: If SQL INSERT fails (other than uniqueness).
/// - `Error::AliasAlreadyExists`: If the alias name already exists (case-insensitive).
pub fn create_alias(
    conn: &Connection,
    alias_name: &str,
    canonical_exercise_name: &str,
) -> Result<(), Error> {
    match conn.execute(
        "INSERT INTO aliases (alias_name, exercise_name) VALUES (?1, ?2)",
        params![alias_name, canonical_exercise_name],
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::ConstraintViolation,
                    ..
                },
                _,
            ) = e
            {
                Err(Error::AliasAlreadyExists(alias_name.to_string()))
            } else {
                Err(Error::InsertFailed(e))
            }
        }
    }
}

/// Deletes an alias by its name (case-insensitive).
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `alias_name` - The name of the alias.
/// # Returns
/// The number of rows affected (should be 1).
/// # Errors
/// - `Error::DeleteFailed`: If SQL DELETE fails.
/// - `Error::AliasNotFound`: If the alias doesn't exist.
pub fn delete_alias(conn: &Connection, alias_name: &str) -> Result<u64, Error> {
    let rows_affected = conn
        .execute(
            "DELETE FROM aliases WHERE alias_name = ?1 COLLATE NOCASE",
            params![alias_name],
        )
        .map_err(Error::DeleteFailed)?;
    if rows_affected == 0 {
        Err(Error::AliasNotFound(alias_name.to_string()))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Retrieves the canonical exercise name associated with an alias (case-insensitive).
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `alias_name` - The alias name.
/// # Returns
/// An `Ok(Some(String))` with the canonical name if found, `Ok(None)` otherwise.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_canonical_name_for_alias(
    conn: &Connection,
    alias_name: &str,
) -> Result<Option<String>, Error> {
    let mut stmt = conn
        .prepare("SELECT exercise_name FROM aliases WHERE alias_name = ?1 COLLATE NOCASE")
        .map_err(Error::QueryFailed)?;
    stmt.query_row(params![alias_name], |row| row.get(0))
        .optional()
        .map_err(Error::QueryFailed)
}

/// Lists all defined aliases and their corresponding canonical exercise names.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// # Returns
/// A `Result` containing a `HashMap<String, String>`.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn list_aliases(conn: &Connection) -> Result<HashMap<String, String>, Error> {
    let mut stmt = conn
        .prepare("SELECT alias_name, exercise_name FROM aliases ORDER BY alias_name ASC")
        .map_err(Error::QueryFailed)?;
    let alias_iter = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(Error::QueryFailed)?;
    alias_iter
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(Error::QueryFailed)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolvedByType {
    Id,
    Alias,
    Name,
}

/// Retrieves an exercise definition by trying ID, then alias, then name.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `identifier` - The string identifier (ID, alias, or name).
/// # Returns
/// `Ok(Some((ExerciseDefinition, ResolvedByType)))` if found, `Ok(None)` otherwise.
/// # Errors
/// - `Error::QueryFailed`: If any query fails.
/// - `Error::Conversion`: If exercise type conversion fails.
pub fn get_exercise_by_identifier(
    conn: &Connection,
    identifier: &str,
) -> Result<Option<(ExerciseDefinition, ResolvedByType)>, Error> {
    if let Ok(id) = identifier.parse::<i64>() {
        if let Some(exercise) = get_exercise_by_id(conn, id)? {
            return Ok(Some((exercise, ResolvedByType::Id)));
        }
        return Ok(None); // Numeric but not found as ID
    }
    if let Some(canonical_name) = get_canonical_name_for_alias(conn, identifier)? {
        if let Some(exercise) = get_exercise_by_name(conn, &canonical_name)? {
            return Ok(Some((exercise, ResolvedByType::Alias)));
        }
        eprintln!(
            "Warning: Alias '{identifier}' points to non-existent exercise '{canonical_name}'."
        );
        return Ok(None);
    }
    match get_exercise_by_name(conn, identifier)? {
        Some(exercise) => Ok(Some((exercise, ResolvedByType::Name))),
        None => Ok(None),
    }
}

/// Lists defined exercises, optionally filtering by type and/or muscle.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `type_filter` - Optional `ExerciseType`.
/// * `muscle_filter` - Optional muscle name substring.
/// # Returns
/// A `Result` containing a vector of `ExerciseDefinition` structs.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
/// - `Error::Conversion`: If exercise type conversion fails.
pub fn list_exercises(
    conn: &Connection,
    type_filter: Option<ExerciseType>,
    muscle_filter: Option<Vec<&str>>,
) -> Result<Vec<ExerciseDefinition>, Error> {
    let mut sql = "SELECT id, name, type, muscles, log_weight, log_reps, log_duration, log_distance
                   FROM exercises WHERE 1=1"
        .to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(t) = type_filter {
        sql.push_str(" AND type = :type");
        params_map.insert(":type".into(), Box::new(t.to_string()));
    }

    if let Some(muscles) = muscle_filter {
        if !muscles.is_empty() {
            // Add a condition for each muscle in the vector
            for (i, muscle) in muscles.iter().enumerate() {
                let param_name = format!(":muscle{}", i);
                sql.push_str(&format!(" AND muscles LIKE {}", param_name));
                params_map.insert(param_name, Box::new(format!("%{}%", muscle)));
            }
        }
    }

    sql.push_str(" ORDER BY name ASC");

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(Error::QueryFailed)?;
    let exercise_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_exercise_definition)
        .map_err(Error::QueryFailed)?;

    exercise_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_collect_error)
}

/// Gets the maximum *effective* weight lifted for a specific exercise.
/// Effective weight = additional weight + bodyweight (for BW exercises) OR just additional weight.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `canonical_exercise_name` - The canonical name.
/// # Returns
/// An `Ok(Some(f64))` with max effective weight, `Ok(None)` if no relevant entries.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_max_effective_weight_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, Error> {
    conn.query_row(
        "SELECT MAX(
             CASE e.type
                 WHEN 'body-weight' THEN COALESCE(w.weight, 0) + COALESCE(w.bodyweight, 0)
                 ELSE w.weight
             END
         )
         FROM workouts w JOIN exercises e ON w.exercise_name = e.name COLLATE NOCASE
         WHERE w.exercise_name = ?1 COLLATE NOCASE",
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(Error::QueryFailed)
    .map(Option::flatten)
}

/// Gets the maximum reps performed for a specific exercise.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `canonical_exercise_name` - The canonical name.
/// # Returns
/// An `Ok(Some(i64))` with max reps, `Ok(None)` if no entries.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_max_reps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, Error> {
    conn.query_row(
        "SELECT MAX(reps) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND reps IS NOT NULL",
        params![canonical_exercise_name], |row| row.get(0),
    ).optional().map_err(Error::QueryFailed).map(Option::flatten)
}

/// Gets the maximum duration for a specific exercise.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `canonical_exercise_name` - The canonical name.
/// # Returns
/// An `Ok(Some(i64))` with max duration, `Ok(None)` if no entries.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_max_duration_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, Error> {
    conn.query_row(
        "SELECT MAX(duration_minutes) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND duration_minutes IS NOT NULL",
        params![canonical_exercise_name], |row| row.get(0),
    ).optional().map_err(Error::QueryFailed).map(Option::flatten)
}

/// Gets the maximum distance (in km) for a specific exercise.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `canonical_exercise_name` - The canonical name.
/// # Returns
/// An `Ok(Some(f64))` with max distance (km), `Ok(None)` if no entries.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_max_distance_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, Error> {
    conn.query_row(
        "SELECT MAX(distance) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND distance IS NOT NULL",
        params![canonical_exercise_name], |row| row.get(0),
    ).optional().map_err(Error::QueryFailed).map(Option::flatten)
}

/// Retrieves all workout timestamps for a specific exercise, ordered chronologically.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `canonical_exercise_name` - The canonical name.
/// # Returns
/// A `Result` containing a vector of `DateTime<Utc>` timestamps.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
/// - `Error::Conversion`: If timestamp parsing fails.
pub fn get_workout_timestamps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Vec<DateTime<Utc>>, Error> {
    let mut stmt = conn.prepare(
        "SELECT timestamp FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE ORDER BY timestamp ASC",
    )?;
    let timestamp_iter = stmt.query_map(params![canonical_exercise_name], |row| {
        let timestamp_str: String = row.get(0)?;
        DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(Error::Conversion(format!(
                        "Invalid timestamp '{timestamp_str}': {e}"
                    ))) as Box<dyn StdError + Send + Sync>,
                )
            })
    })?;
    timestamp_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_collect_error)
}

/// Adds a new bodyweight entry.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `timestamp` - The timestamp of the measurement.
/// * `weight` - The bodyweight value.
/// # Returns
/// The `rowid` of the newly inserted entry.
/// # Errors
/// - `Error::InsertFailed`: If SQL INSERT fails (other than uniqueness).
/// - `Error::BodyweightEntryExists`: If an entry with the same timestamp exists.
pub fn add_bodyweight(
    conn: &Connection,
    timestamp: DateTime<Utc>,
    weight: f64,
) -> Result<i64, Error> {
    let timestamp_str = timestamp.to_rfc3339();
    conn.execute(
        "INSERT INTO bodyweights (timestamp, weight) VALUES (?1, ?2)",
        params![timestamp_str, weight],
    )
    .map_err(|e| {
        if let rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ConstraintViolation,
                extended_code: 2067,
                ..
            },
            _,
        ) = e
        {
            Error::BodyweightEntryExists(timestamp_str)
        } else {
            Error::InsertFailed(e)
        }
    })?;
    Ok(conn.last_insert_rowid())
}

/// Retrieves the most recent bodyweight entry.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// # Returns
/// An `Ok(Some(f64))` with the latest weight, `Ok(None)` otherwise.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn get_latest_bodyweight(conn: &Connection) -> Result<Option<f64>, Error> {
    conn.query_row(
        "SELECT weight FROM bodyweights ORDER BY timestamp DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(Error::QueryFailed)
}

/// Retrieves bodyweight entries, ordered by timestamp descending, up to a limit.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `limit` - The maximum number of entries.
/// # Returns
/// A `Result` containing a vector of `(i64, DateTime<Utc>, f64)`.
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
/// - `Error::Conversion`: If timestamp parsing fails.
pub fn list_bodyweights(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<(i64, DateTime<Utc>, f64)>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, weight FROM bodyweights ORDER BY timestamp DESC LIMIT ?1",
    )?;
    let iter = stmt.query_map(params![limit], |row| {
        let id: i64 = row.get(0)?;
        let timestamp_str: String = row.get(1)?;
        let weight: f64 = row.get(2)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(Error::Conversion(format!(
                        "Invalid timestamp '{timestamp_str}': {e}"
                    ))) as Box<dyn StdError + Send + Sync>,
                )
            })?;
        Ok((id, timestamp, weight))
    })?;
    iter.collect::<Result<Vec<_>, _>>()
        .map_err(map_collect_error)
}

/// Deletes a bodyweight entry by its ID.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// * `id` - The ID of the entry.
/// # Returns
/// The number of rows affected (should be 1).
/// # Errors
/// - `Error::DeleteFailed`: If SQL DELETE fails.
/// - `Error::BodyWeightEntryNotFound`: If the ID doesn't exist.
pub fn delete_bodyweight(conn: &Connection, id: i64) -> Result<usize, Error> {
    let rows_affected = conn
        .execute("DELETE FROM bodyweights WHERE id = ?", params![id])
        .map_err(Error::DeleteFailed)?;
    if rows_affected == 0 {
        Err(Error::BodyWeightEntryNotFound(id))
    } else {
        Ok(rows_affected)
    }
}

/// Retrieves a distinct list of dates on which any workout was recorded.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// # Returns
/// A `Result` containing a vector of unique `NaiveDate`s, sorted chronologically.
/// # Errors
/// - `Error::QueryFailed`: If the query or date parsing fails.
pub fn get_all_dates_with_exercise(conn: &Connection) -> Result<Vec<NaiveDate>, Error> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT DATE(timestamp) FROM workouts ORDER BY DATE(timestamp) ASC")
        .map_err(Error::QueryFailed)?;
    let date_iter = stmt
        .query_map([], |row| {
            let date_str: String = row.get(0)?;
            date_str.parse::<NaiveDate>().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .map_err(Error::QueryFailed)?;
    date_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::QueryFailed)
}

/// Retrieves a sorted list of unique muscle names defined across all exercises.
/// # Arguments
/// * `conn` - A reference to the database connection.
/// # Returns
/// A `Result` containing a vector of unique, sorted muscle names (lowercase).
/// # Errors
/// - `Error::QueryFailed`: If the query fails.
pub fn list_all_muscles(conn: &Connection) -> Result<Vec<String>, Error> {
    let mut stmt = conn
        .prepare("SELECT muscles FROM exercises WHERE muscles IS NOT NULL AND muscles != ''")
        .map_err(Error::QueryFailed)?;
    let muscle_csv_iter = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(Error::QueryFailed)?;

    let mut unique_muscles: HashSet<String> = HashSet::new();
    for muscle_csv_result in muscle_csv_iter {
        match muscle_csv_result {
            Ok(csv) => {
                for part in csv.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        unique_muscles.insert(trimmed.to_lowercase());
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Skipping row due to muscle fetch error: {e}");
            }
        }
    }
    let mut sorted_muscles: Vec<String> = unique_muscles.into_iter().collect();
    sorted_muscles.sort_unstable();
    Ok(sorted_muscles)
}

/// Adds a boolean log flag column to the exercises table if it doesn't exist.
fn add_log_flag_column_if_not_exists(
    conn: &Connection,
    column_name: &str,
    default_value: i64,
) -> Result<(), Error> {
    // Check if column exists using PRAGMA
    let mut stmt = conn.prepare("PRAGMA table_info(exercises)")?;
    let column_exists = stmt
        .query_map([], |row| row.get::<_, String>(1))? // Column names are in the second column (index 1)
        .any(|col_res| col_res.map_or(false, |col| col == column_name));

    // Add column if it doesn't exist
    if !column_exists {
        // SQLite uses 0 for FALSE and 1 for TRUE for BOOLEAN columns
        // We add NOT NULL and a DEFAULT to ensure existing rows get a value
        let sql = format!(
            "ALTER TABLE exercises ADD COLUMN {} BOOLEAN NOT NULL DEFAULT {}",
            column_name, default_value
        );
        conn.execute(&sql, [])?;
    }
    Ok(())
}

pub fn get_workout_dates_for_month_db(
    conn: &Connection,
    year: i32,
    month: u32,
) -> Result<Vec<String>, Error> {
    // Ensure month is valid (1-12) - good to keep this check close to the query
    if !(1..=12).contains(&month) {
        // Using anyhow::bail here if you prefer it for internal validation in db.rs
        // or return a specific db::Error variant. For consistency with db::Error:
        return Err(Error::InvalidParameterCount(month as usize, 12)); // Or a more specific error
    }

    // SQL query for RFC3339 timestamps stored in the 'timestamp' column
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT strftime('%Y-%m-%d', timestamp) AS workout_day \
         FROM workouts \
         WHERE CAST(strftime('%Y', timestamp) AS INTEGER) = ?1 \
           AND CAST(strftime('%m', timestamp) AS INTEGER) = ?2 \
         ORDER BY workout_day;",
        )
        .map_err(Error::QueryFailed)?;

    let date_iter = stmt
        .query_map(
            params![year, month],
            |row| row.get(0), // Get the 'workout_day' string
        )
        .map_err(Error::QueryFailed)?;

    let mut dates = Vec::new();
    for date_result in date_iter {
        dates.push(date_result.map_err(Error::QueryFailed)?); // Ensure rusqlite::Error is mapped
    }

    Ok(dates)
}
