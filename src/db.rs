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
    let sets = sets.unwrap_or(1);
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
            Err(_) => None, // Silently ignore invalid type from DB in lib layer
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

    // Apply limit only if date is not specified (limit applies to overall latest, not within a date)
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
 ) -> Result<Vec<Workout>, anyhow::Error> { // Keep anyhow::Error here or use DbError? DbError preferred
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
            if let rusqlite::Error::SqliteFailure(ref err, Some(ref msg)) = e {
                 // Check for UNIQUE constraint violation specifically on 'exercises.name'
                 if err.code == rusqlite::ErrorCode::ConstraintViolation && msg.contains("UNIQUE constraint failed: exercises.name") {
                    // Verify case-insensitivity wasn't the issue before returning specific error
                    if get_exercise_by_name(conn, name).map_err(|db_err| anyhow::Error::new(db_err))?.is_some() {
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
        // No fields to update, return Ok(0)
        return Ok(0);
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
        tx.execute("UPDATE workouts SET exercise_name = ?1 WHERE exercise_name = ?2", params![target_new_name, original_name])
          .map_err(DbError::UpdateFailed)?;
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
            None => {
                // It parsed as ID but wasn't found. Should we try as name?
                // Let's keep the original logic: if it looks like an ID, only check ID.
                Ok(None)
            },
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
