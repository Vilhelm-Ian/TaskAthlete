//src/db.rs
use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, ToSql}; // Import named_params
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

#[derive(Default, Debug)]
pub struct VolumeFilters<'a> {
    pub exercise_name: Option<&'a str>, // Canonical name expected
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit_days: Option<u32>, // Limit number of distinct days returned
}

pub fn calculate_daily_volume_filtered(
    conn: &Connection,
    filters: VolumeFilters,
) -> Result<Vec<(NaiveDate, String, f64)>, DbError> {
    // Base query calculates volume per workout *entry*
    let mut sql = "
        SELECT
            date(w.timestamp) as workout_date,
            w.exercise_name, -- Select the exercise name
            SUM(CASE
                    WHEN e.type IN ('resistance', 'body-weight')
                    THEN COALESCE(w.sets, 1) * COALESCE(w.reps, 0) * COALESCE(w.weight, 0)
                    ELSE 0 -- Define volume as 0 for Cardio or other types
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
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }

    // Group by date AND exercise name to sum volume correctly per exercise per day
    sql.push_str(
        " GROUP BY workout_date, w.exercise_name ORDER BY workout_date DESC, w.exercise_name ASC",
    );

    // Limit the *number of rows* returned (each row is one exercise on one day)
    // Note: This isn't limiting distinct days directly if one day has multiple exercises.
    // Limiting distinct days would require a subquery, which adds complexity.
    // Let's keep limiting rows for simplicity for now.
    if filters.start_date.is_none() && filters.end_date.is_none() {
        // Apply limit only if no date range specified
        if let Some(limit) = filters.limit_days {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let volume_iter = stmt
        .query_map(params_for_query.as_slice(), |row| {
            let date_str: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let exercise_name: String = row.get(1)?; // Get exercise name
            let volume: f64 = row.get(2)?; // Volume is now the 3rd column (index 2)
            Ok((date, exercise_name, volume))
        })
        .map_err(DbError::QueryFailed)?;

    volume_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

#[derive(Debug, Clone)]
pub struct Workout {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String, // Always the canonical name
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub distance: Option<f64>, // Added distance
    pub notes: Option<String>,
    pub exercise_type: Option<ExerciseType>, // Populated by JOIN
}

#[derive(Debug, Clone, PartialEq)] // Add Clone
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
    #[error("Alias not found: {0}")] // Feature 1
    AliasNotFound(String),
    #[error("Alias already exists: {0}")] // Feature 1
    AliasAlreadyExists(String),
    #[error("Exercise name must be unique (case-insensitive): '{0}' already exists.")] // Feature 2
    ExerciseNameNotUnique(String),
    #[error("No workout data found for exercise '{0}'")]
    NoWorkoutDataFound(String),
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
            sets INTEGER,
            reps INTEGER,
            weight REAL,
            duration_minutes INTEGER,
            distance REAL, -- Added distance column
            notes TEXT
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
        [],).map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bodyweights (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL UNIQUE, -- Store as RFC3339 string, unique timestamp
            weight REAL NOT NULL -- Store weight (assumed in config units, but DB doesn't enforce)
        )",
        [],
    )
    .map_err(DbError::Connection)?;

    // Add indexes for common lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workouts_timestamp ON workouts(timestamp)",
        [],
    )
    .map_err(DbError::Connection)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workouts_exercise_name ON workouts(exercise_name)",
        [],
    )
    .map_err(DbError::Connection)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_aliases_exercise_name ON aliases(exercise_name)",
        [],
    )
    .map_err(DbError::Connection)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_bodyweights_timestamp ON bodyweights(timestamp)",
        [],
    )
    .map_err(DbError::Connection)?;

    // Add distance column if it doesn't exist (for upgrading existing databases)
    add_distance_column_if_not_exists(conn)?;

    Ok(())
}

/// Adds the distance column to the workouts table if it doesn't exist.
/// This is useful for users upgrading from a previous version.
fn add_distance_column_if_not_exists(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(workouts)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut distance_exists = false;
    for column_result in columns {
        if let Ok(column_name) = column_result {
            if column_name == "distance" {
                distance_exists = true;
                break;
            }
        }
    }

    if !distance_exists {
        println!("Adding 'distance' column to workouts table..."); // Inform user
        conn.execute("ALTER TABLE workouts ADD COLUMN distance REAL", [])?;
    }

    Ok(())
}

/// Adds a new workout entry to the database.
pub fn add_workout(
    conn: &Connection,
    exercise_name: &str,      // Should be the canonical name
    timestamp: DateTime<Utc>, // Feature 3: Accept specific timestamp
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    distance: Option<f64>,
    notes: Option<String>,
) -> Result<i64, DbError> {
    let timestamp_str = timestamp.to_rfc3339();
    // Use default value 1 for sets only if it's None and the exercise type needs it (e.g., resistance, bodyweight)
    // For simplicity, let's keep the original behavior where sets default to 1 if None.
    // A more robust approach might check exercise type.
    let sets_val = sets.unwrap_or(1);

    conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, distance, notes)
         VALUES (:ts, :ex_name, :sets, :reps, :weight, :duration, :distance, :notes)",
        named_params! {
            ":ts": timestamp_str,
            ":ex_name": exercise_name,
            ":sets": sets_val,
            ":reps": reps,
            ":weight": weight,
            ":duration": duration,
            ":distance": distance, // Add distance
            ":notes": notes,
        },
    ).map_err(DbError::InsertFailed)?;
    Ok(conn.last_insert_rowid())
}

/// Updates an existing workout entry in the database by its ID.
pub fn update_workout(
    conn: &Connection,
    id: i64,
    new_exercise_name: Option<&str>,
    new_sets: Option<i64>,
    new_reps: Option<i64>,
    new_weight: Option<f64>,
    new_duration: Option<i64>,
    new_distance: Option<f64>,
    new_notes: Option<&str>,
    new_timestamp: Option<DateTime<Utc>>, // Feature 3: Allow editing timestamp
) -> Result<u64, DbError> {
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(ex) = new_exercise_name {
        updates.push("exercise_name = :ex_name");
        params_map.insert(":ex_name".into(), Box::new(ex.to_string()));
    }
    if let Some(s) = new_sets {
        updates.push("sets = :sets");
        params_map.insert(":sets".into(), Box::new(s));
    }
    if let Some(r) = new_reps {
        updates.push("reps = :reps");
        params_map.insert(":reps".into(), Box::new(r));
    }
    // Use is_some() to allow setting weight/duration/distance to NULL explicitly if needed, though CLI usually wouldn't do this.
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
    } // Add distance
    if new_notes.is_some() {
        updates.push("notes = :notes");
        params_map.insert(":notes".into(), Box::new(new_notes));
    }
    if let Some(ts) = new_timestamp {
        updates.push("timestamp = :ts");
        params_map.insert(":ts".into(), Box::new(ts.to_rfc3339()));
    }

    let sql = format!("UPDATE workouts SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));

    // Convert HashMap<String, Box<dyn ToSql>> to Vec<(&str, &dyn ToSql)> for execute_named
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = conn
        .execute(&sql, params_for_exec.as_slice())
        .map_err(DbError::UpdateFailed)?;

    if rows_affected == 0 {
        Err(DbError::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes a workout entry from the database by its ID.
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64, DbError> {
    // Return DbError
    let rows_affected = conn
        .execute("DELETE FROM workouts WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 {
        Err(DbError::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
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
    let distance: Option<f64> = row.get(7)?; // Added distance
    let notes: Option<String> = row.get(8)?;
    let type_str_opt: Option<String> = row.get(9)?; // From JOIN with exercises

    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let exercise_type = match type_str_opt {
        Some(type_str) => match ExerciseType::try_from(type_str.as_str()) {
            Ok(et) => Some(et),
            Err(_) => None, // Silently ignore invalid type from DB in lib layer
        },
        None => None,
    };

    Ok(Workout {
        id,
        timestamp,
        exercise_name,
        sets,
        reps,
        weight,
        duration_minutes,
        distance,
        notes,
        exercise_type,
    })
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
pub fn list_workouts_filtered(
    conn: &Connection,
    filters: WorkoutFilters,
) -> Result<Vec<Workout>, DbError> {
    // Return DbError
    // Note: Column indices change due to adding `distance`
    let mut sql = "SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.notes, e.type
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
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }

    // Order by timestamp: ASC if date filter is used (show earliest first for that day), DESC otherwise (show latest overall)
    if filters.date.is_some() {
        sql.push_str(" ORDER BY w.timestamp ASC");
    } else {
        sql.push_str(" ORDER BY w.timestamp DESC");
    }

    // Apply limit only if date is not specified (limit applies to overall latest, not within a date)
    if filters.date.is_none() {
        if let Some(limit) = filters.limit {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    // Convert HashMap to slice for query_map_named
    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_workout)
        .map_err(DbError::QueryFailed)?;

    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed) // Collect results
}

/// Lists workouts for a specific exercise (canonical name) performed on the Nth most recent day it was done.
pub fn list_workouts_for_exercise_on_nth_last_day(
    conn: &Connection,
    exercise_name: &str, // Canonical name expected
    n: u32,
) -> Result<Vec<Workout>, DbError> {
    // Return DbError
    if n == 0 {
        return Err(DbError::QueryFailed(
            rusqlite::Error::InvalidParameterCount(n as usize, 2),
        ));
    } // Indicate bad N via error
    let offset = n - 1;
    // Note: Column indices change due to adding `distance`
    let sql = "WITH RankedDays AS (SELECT DISTINCT date(timestamp) as workout_date FROM workouts WHERE exercise_name = :ex_name COLLATE NOCASE ORDER BY workout_date DESC LIMIT 1 OFFSET :offset)
                SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.notes, e.type
                FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
                WHERE w.exercise_name = :ex_name COLLATE NOCASE ORDER BY w.timestamp ASC;";

    let mut stmt = conn.prepare(sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(
            named_params! { ":ex_name": exercise_name, ":offset": offset },
            map_row_to_workout,
        )
        .map_err(DbError::QueryFailed)?;

    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

// ---- Exercise Definition Functions ----

/// Creates a new exercise definition. Returns ID. Handles UNIQUE constraint.
pub fn create_exercise(
    conn: &Connection,
    name: &str,
    type_: &ExerciseType,
    muscles: Option<&str>,
) -> Result<i64, DbError> {
    // Return DbError
    let type_str = type_.to_string();
    match conn.execute(
        "INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)",
        params![name, type_str, muscles],
    ) {
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
    conn: &mut Connection,          // Use mutable connection for transaction
    canonical_name_to_update: &str, // Use the resolved canonical name
    new_name: Option<&str>,
    new_type: Option<&ExerciseType>,
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

    if updates.is_empty() {
        return Ok(0);
    } // No fields to update

    // Use a transaction
    let tx = conn.transaction().map_err(DbError::Connection)?;

    // 1. Update exercises table
    let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

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
        tx.execute("UPDATE workouts SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE", // Ensure case-insensitive match on old name
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(DbError::UpdateFailed)?;

        // Update aliases table (Feature 1)
        tx.execute("UPDATE aliases SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE", // Ensure case-insensitive match on old name
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
    tx.execute(
        "DELETE FROM aliases WHERE exercise_name = ? COLLATE NOCASE",
        params![name_to_delete],
    ) // Ensure case-insensitive match
    .map_err(DbError::DeleteFailed)?;

    // 2. Delete the exercise definition
    let rows_affected = tx
        .execute("DELETE FROM exercises WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;

    tx.commit().map_err(DbError::Connection)?;

    if rows_affected == 0 {
        // Should not happen if get_exercise_by_name succeeded
        Err(DbError::ExerciseNotFound(name_to_delete))
    } else {
        Ok(rows_affected as u64)
    }
}

fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let type_str: String = row.get(2)?;
    let muscles: Option<String> = row.get(3)?;
    let ex_type = ExerciseType::try_from(type_str.as_str()).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
        )
    })?;
    Ok(ExerciseDefinition {
        id,
        name,
        type_: ex_type,
        muscles,
    })
}

/// Retrieves an exercise definition by its name (case-insensitive).
pub fn get_exercise_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE name = ?1 COLLATE NOCASE")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![name], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Retrieves an exercise definition by its ID.
pub fn get_exercise_by_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE id = ?1")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![id], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

// --- Alias Functions (Feature 1) ---

/// Creates a new alias for a given canonical exercise name.
pub fn create_alias(
    conn: &Connection,
    alias_name: &str,
    canonical_exercise_name: &str,
) -> Result<(), DbError> {
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
    let rows_affected = conn
        .execute(
            "DELETE FROM aliases WHERE alias_name = ?1 COLLATE NOCASE",
            params![alias_name],
        ) // Ensure case-insensitive match
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 {
        Err(DbError::AliasNotFound(alias_name.to_string()))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Retrieves the canonical exercise name associated with an alias (case-insensitive).
pub fn get_canonical_name_for_alias(
    conn: &Connection,
    alias_name: &str,
) -> Result<Option<String>, DbError> {
    let mut stmt = conn
        .prepare("SELECT exercise_name FROM aliases WHERE alias_name = ?1 COLLATE NOCASE")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![alias_name], |row| row.get(0))
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Lists all defined aliases and their corresponding canonical exercise names.
pub fn list_aliases(conn: &Connection) -> Result<HashMap<String, String>, DbError> {
    let mut stmt = conn
        .prepare("SELECT alias_name, exercise_name FROM aliases ORDER BY alias_name ASC")
        .map_err(DbError::QueryFailed)?;
    let alias_iter = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(DbError::QueryFailed)?;

    alias_iter
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(DbError::QueryFailed)
}

// --- Combined Identifier Resolution ---

/// Retrieves an exercise definition by trying ID first, then alias, then name.
/// Returns Option<(Definition, ResolvedByType)>.
#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedByType {
    Id,
    Alias,
    Name,
}

pub fn get_exercise_by_identifier(
    conn: &Connection,
    identifier: &str,
) -> Result<Option<(ExerciseDefinition, ResolvedByType)>, DbError> {
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
                eprintln!(
                    "Warning: Alias '{}' points to non-existent exercise '{}'.",
                    identifier, canonical_name
                );
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
    conn: &Connection,
    type_filter: Option<ExerciseType>,
    muscle_filter: Option<&str>,
) -> Result<Vec<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(t) = type_filter {
        sql.push_str(" AND type = :type");
        params_map.insert(":type".into(), Box::new(t.to_string()));
    }
    if let Some(m) = muscle_filter {
        sql.push_str(" AND muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }
    sql.push_str(" ORDER BY name ASC");

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let exercise_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_exercise_definition)
        .map_err(DbError::QueryFailed)?;

    exercise_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed) // Collect results
}

// --- Personal Best Query Functions (Feature 4) ---

/// Gets the maximum weight lifted for a specific exercise (canonical name).
pub fn get_max_weight_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT MAX(weight) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND weight IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<f64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum reps performed in a single set for a specific exercise (canonical name).
pub fn get_max_reps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, DbError> {
    conn.query_row(
        // Note: This assumes reps are per set. If reps column means total reps for the entry, the interpretation changes.
        // Assuming reps is 'reps per set'.
        "SELECT MAX(reps) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND reps IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<i64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum duration in minutes for a specific exercise (canonical name).
pub fn get_max_duration_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, DbError> {
    conn.query_row(
        "SELECT MAX(duration_minutes) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND duration_minutes IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum distance for a specific exercise (canonical name). Assumes distance stored in km.
pub fn get_max_distance_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT MAX(distance) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND distance IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    .map(|opt_opt| opt_opt.flatten())
}

// --- Statistics Query Functions ---

/// Retrieves all workout timestamps for a specific exercise, ordered chronologically.
pub fn get_workout_timestamps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Vec<DateTime<Utc>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT timestamp FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE ORDER BY timestamp ASC", // Add COLLATE NOCASE
    )?;
    let timestamp_iter = stmt.query_map(params![canonical_exercise_name], |row| {
        let timestamp_str: String = row.get(0)?;
        DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
    })?;

    timestamp_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

// --- Bodyweight Functions ---

/// Adds a new bodyweight entry.
pub fn add_bodyweight(
    conn: &Connection,
    timestamp: DateTime<Utc>,
    weight: f64,
) -> Result<i64, DbError> {
    let timestamp_str = timestamp.to_rfc3339();
    conn.execute(
        "INSERT INTO bodyweights (timestamp, weight) VALUES (?1, ?2)",
        params![timestamp_str, weight],
    )
    .map_err(|e| {
        // Handle potential UNIQUE constraint violation on timestamp nicely
        if let rusqlite::Error::SqliteFailure(ref err, _) = e {
            if err.code == rusqlite::ErrorCode::ConstraintViolation && err.extended_code == 2067 {
                // SQLITE_CONSTRAINT_UNIQUE (extended)
                return DbError::BodyweightEntryExists(timestamp_str);
            }
        }
        DbError::InsertFailed(e)
    })?;
    Ok(conn.last_insert_rowid())
}

/// Retrieves the most recent bodyweight entry.
pub fn get_latest_bodyweight(conn: &Connection) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT weight FROM bodyweights ORDER BY timestamp DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
}

/// Retrieves all bodyweight entries, ordered by timestamp descending.
pub fn list_bodyweights(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<(DateTime<Utc>, f64)>, DbError> {
    let mut stmt =
        conn.prepare("SELECT timestamp, weight FROM bodyweights ORDER BY timestamp DESC LIMIT ?1")?;
    let iter = stmt.query_map(params![limit], |row| {
        let timestamp_str: String = row.get(0)?;
        let weight: f64 = row.get(1)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok((timestamp, weight))
    })?;
    iter.collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}
