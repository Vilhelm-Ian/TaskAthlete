// src/db.rs
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, ToSql};
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
/// Creates the directory if it doesn't exist.
pub fn get_db_path() -> Result<PathBuf, DbError> {
    let data_dir = dirs::data_dir().ok_or(DbError::DataDir)?;
    let app_dir = data_dir.join("workout-tracker-cli"); // Same dir name as config for consistency
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
            timestamp TEXT NOT NULL,           -- Store as ISO 8601 string (RFC3339)
            exercise_name TEXT NOT NULL COLLATE NOCASE, -- Store canonical name, match case-insensitively
            sets INTEGER,
            reps INTEGER,
            weight REAL,                       -- Use REAL for f64
            duration_minutes INTEGER,
            notes TEXT
            -- Removed FOREIGN KEY for simplicity on exercise name changes, rely on JOIN
        )",
        [],
    )
    .map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE, -- Ensure name uniqueness (case-insensitive)
            type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio', 'body-weight')),
            muscles TEXT -- Comma-separated list or similar
        )",
        [],
    )
    .map_err(DbError::Connection)?;

    // Removed settings table

    Ok(())
}

/// Adds a new workout entry to the database.
pub fn add_workout(
    conn: &Connection,
    exercise_name: &str, // Use the canonical name from ExerciseDefinition
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>, // This is the *final* weight (potentially including bodyweight)
    duration: Option<i64>,
    notes: Option<String>,
) -> Result<i64> {
    let timestamp = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            timestamp,
            exercise_name, // Store the canonical name
            sets,
            reps,
            weight, // Store final calculated weight
            duration,
            notes
        ],
    )
    .map_err(DbError::InsertFailed)?; // Use specific error

    Ok(conn.last_insert_rowid())
}

/// Updates an existing workout entry in the database by its ID.
pub fn update_workout(
    conn: &Connection,
    id: i64, // Identify workout by unique ID
    new_exercise_name: Option<&str>, // Allow changing the linked exercise
    new_sets: Option<i64>,
    new_reps: Option<i64>,
    new_weight: Option<f64>, // Update with the provided absolute value
    new_duration: Option<i64>,
    new_notes: Option<&str>,
) -> Result<u64> {
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut updates = Vec::new();

    // Important: If exercise name changes, we might ideally check the *new* exercise's type.
    // For simplicity here, editing `weight` just sets the value directly.
    // Bodyweight logic is only applied during `add`.
    if let Some(ex) = new_exercise_name {
        updates.push("exercise_name = ?");
        params.push(Box::new(ex.to_string())); // Ensure it's owned
    }
    if let Some(s) = new_sets {
        updates.push("sets = ?");
        params.push(Box::new(s));
    }
    if let Some(r) = new_reps {
        updates.push("reps = ?");
        params.push(Box::new(r));
    }
    if new_weight.is_some() { // Handle Option<f64> correctly
        updates.push("weight = ?");
        params.push(Box::new(new_weight));
    }
    if new_duration.is_some() { // Handle Option<i64> correctly
        updates.push("duration_minutes = ?");
        params.push(Box::new(new_duration));
    }
     if new_notes.is_some() { // Handle Option<&str> by checking is_some
        updates.push("notes = ?");
        params.push(Box::new(new_notes)); // Option<&str> can be passed if Some
    }


    if updates.is_empty() {
        anyhow::bail!("No fields provided to update for workout ID {}", id);
    }

    let sql = format!("UPDATE workouts SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(id)); // Add ID as the last parameter for WHERE clause

    // Convert Vec<Box<dyn ToSql>> to Vec<&dyn ToSql> for execute
    let params_slice: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();

    let rows_affected = conn
        .execute(&sql, params_slice.as_slice())
        .map_err(DbError::UpdateFailed)?; // Use specific error

    if rows_affected == 0 {
        // Workout ID might not exist
        Err(DbError::WorkoutNotFound(id).into())
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes a workout entry from the database by its ID.
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64> {
    let rows_affected = conn
        .execute("DELETE FROM workouts WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?; // Use specific error

    if rows_affected == 0 {
        Err(DbError::WorkoutNotFound(id).into())
    } else {
        Ok(rows_affected as u64)
    }
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
    let type_str_opt: Option<String> = row.get(8)?; // From LEFT JOIN exercises

    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let exercise_type = match type_str_opt {
        Some(type_str) => match ExerciseType::try_from(type_str.as_str()) {
            Ok(et) => Some(et),
            Err(e) => {
                eprintln!(
                    "Warning: Invalid exercise type '{}' in DB for exercise '{}': {}. Mapping as None.",
                    type_str, exercise_name, e
                );
                None // Don't fail the whole query, just mark type as unknown
            }
        },
        None => None, // Exercise definition not found or type was NULL
    };

    Ok(Workout {
        id,
        timestamp,
        exercise_name,
        sets,
        reps,
        weight,
        duration_minutes,
        notes,
        exercise_type,
    })
}

// Struct to hold filter criteria for list_workouts_filtered
#[derive(Default, Debug)]
pub struct WorkoutFilters<'a> {
    pub exercise_name: Option<&'a str>,
    pub date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit: Option<u32>, // Use Option for limit
}


/// Lists workout entries from the database based on various filters.
pub fn list_workouts_filtered(conn: &Connection, filters: WorkoutFilters) -> Result<Vec<Workout>> {
    let mut sql = "
        SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
        FROM workouts w
        LEFT JOIN exercises e ON w.exercise_name = e.name -- Case-insensitive join due to COLLATE NOCASE on exercises.name
        WHERE 1=1" // Start WHERE clause
        .to_string();

    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new(); // Use Box<dyn ToSql> for heterogeneous types

    if let Some(name) = filters.exercise_name {
        sql.push_str(&format!(" AND w.exercise_name = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(name.to_string())); // Store owned string
    }
    if let Some(date) = filters.date {
        sql.push_str(&format!(
            " AND date(w.timestamp) = date(?{})",
            params_vec.len() + 1
        ));
        params_vec.push(Box::new(date.format("%Y-%m-%d").to_string())); // Store formatted date string
    }
     if let Some(ex_type) = filters.exercise_type {
        sql.push_str(&format!(" AND e.type = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(ex_type.to_string())); // Store type string
    }
     if let Some(m) = filters.muscle {
        sql.push_str(&format!(" AND e.muscles LIKE ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(format!("%{}%", m))); // Wrap muscle in % for LIKE
    }

    // Determine ordering: If a date is specified, order by time within the day. Otherwise, order by most recent first.
    if filters.date.is_some() {
        sql.push_str(" ORDER BY w.timestamp ASC");
    } else {
         sql.push_str(" ORDER BY w.timestamp DESC");
    }


    // Apply LIMIT only if no date/day filters are active and limit is provided
    if filters.date.is_none() { // Add check: only apply limit if date is not specified
         if let Some(limit) = filters.limit {
            sql.push_str(&format!(" LIMIT ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(limit));
         }
    }


    // Convert Vec<Box<dyn ToSql>> to Vec<&dyn ToSql> for query_map
    let params_slice: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(params_slice.as_slice(), map_row_to_workout)
        .map_err(DbError::QueryFailed)?;

    let mut workouts = Vec::new();
    for workout_result in workout_iter {
        // Don't use context here, let map_row_to_workout handle its errors, propagate query errors
        workouts.push(workout_result?); // Propagate rusqlite::Error if mapping fails
    }
    Ok(workouts)
}


/// Lists workouts for a specific exercise performed on the Nth most recent day it was done.
pub fn list_workouts_for_exercise_on_nth_last_day(
    conn: &Connection,
    exercise_name: &str,
    n: u32,
) -> Result<Vec<Workout>> {
    if n == 0 {
        anyhow::bail!("Nth last day (N) must be 1 or greater.");
    }
    let offset = n - 1; // SQL OFFSET is 0-based

    // CTE to find the Nth most recent distinct date for the exercise
    // Join with exercises table is still needed to get the type for mapping
    let sql = "
        WITH RankedDays AS (
            SELECT DISTINCT date(timestamp) as workout_date
            FROM workouts
            WHERE exercise_name = ?1 COLLATE NOCASE -- Match case-insensitively
            ORDER BY workout_date DESC
            LIMIT 1 OFFSET ?2
        )
        SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes, e.type
        FROM workouts w
        LEFT JOIN exercises e ON w.exercise_name = e.name -- Case-insensitive join
        JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
        WHERE w.exercise_name = ?1 COLLATE NOCASE -- Filter again on the primary table
        ORDER BY w.timestamp ASC; -- Order chronologically within that day
    ";

    let mut stmt = conn.prepare(sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(params![exercise_name, offset], map_row_to_workout)
        .map_err(DbError::QueryFailed)?;

    let mut workouts = Vec::new();
    for workout_result in workout_iter {
         workouts.push(workout_result.context("Failed mapping row in nth_last_day query")?); // Add context here specifically
    }
    Ok(workouts)
}


// ---- Exercise Definition Functions ----

/// Creates a new exercise definition in the database. Returns the ID.
pub fn create_exercise(
    conn: &Connection,
    name: &str,
    type_: &ExerciseType,
    muscles: Option<&str>,
) -> Result<i64> {
    let type_str = type_.to_string();
     // Try insert first
    let result = conn.execute(
        "INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)",
        params![name, type_str, muscles],
    );

     match result {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(e) => {
            // Check if it's a UNIQUE constraint violation (SQLite error code 19, constraint violation 2067 for UNIQUE)
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                 // SQLITE_CONSTRAINT_UNIQUE = 2067
                 // SQLITE_CONSTRAINT (generic) = 19
                 // Check primary code 19, then extended 2067 if available
                 // Or just check if the name exists now
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                     // Check if it already exists to give a specific error
                    if get_exercise_by_name(conn, name)?.is_some() {
                        return Err(anyhow::anyhow!("Exercise '{}' already exists (case-insensitive).", name));
                    }
                }
            }
             // If it wasn't a unique constraint or some other error occurred
            Err(DbError::InsertFailed(e).into())
        }
    }
}

/// Updates an existing exercise definition (found by ID or name).
pub fn update_exercise(
    conn: &mut Connection,
    identifier: &str,
    new_name: Option<&str>, // Use &str
    new_type: Option<&ExerciseType>,
    new_muscles: Option<Option<&str>>, // Allow explicitly setting muscles to NULL/None
) -> Result<u64> {
     // Find the exercise ID first
    let exercise = get_exercise_by_identifier(conn, identifier)?
        .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
    let id = exercise.id;

    // Store the original name to check if it's being changed
    let original_name = exercise.name;
    let name_being_changed = new_name.is_some() && new_name != Some(original_name.as_str());
    let target_new_name = new_name.unwrap_or(&original_name); // Use new name if provided, else original

    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name {
        updates.push("name = ?");
        params.push(Box::new(name.to_string())); // Store owned string
    }
    if let Some(t) = new_type {
        updates.push("type = ?");
        params.push(Box::new(t.to_string())); // Store type string
    }
    if let Some(m_opt) = new_muscles { // Check the outer Option
        updates.push("muscles = ?");
        params.push(Box::new(m_opt)); // Pass Option<&str> directly
    }

    if updates.is_empty() {
        anyhow::bail!("No fields provided to update for exercise '{}'", identifier);
    }

    // Begin transaction
    let tx = conn.transaction()?;

    // 1. Update the exercises table
    let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(id)); // Add ID for WHERE clause

    let params_slice_update: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows_affected = tx
        .execute(&sql_update_exercise, params_slice_update.as_slice())
        .map_err(DbError::UpdateFailed)?;

    // 2. If the name was changed, update existing workout entries
    if name_being_changed {
         println!("Updating workout entries referencing old name '{}'...", original_name);
        let rows_workouts = tx.execute(
            "UPDATE workouts SET exercise_name = ?1 WHERE exercise_name = ?2",
            params![target_new_name, original_name], // New name, old name
        )?;
         println!("Updated {} workout entries.", rows_workouts);
    }

    // Commit transaction
    tx.commit()?;


    if rows_affected == 0 {
        // Should not happen if get_exercise_by_identifier succeeded, but check anyway
         Err(DbError::ExerciseNotFound(identifier.to_string()).into())
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes an exercise definition (found by ID or name). Also warns if workouts reference it.
pub fn delete_exercise(conn: &Connection, identifier: &str) -> Result<u64> {
    // Find the exercise ID and Name first
    let exercise = get_exercise_by_identifier(conn, identifier)?
        .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
    let id = exercise.id;
    let name = exercise.name; // Get the canonical name

    // Check if any workouts reference this exercise NAME (case-insensitive check needed if possible)
    let workout_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE",
        params![name], // Use the canonical name found
        |row| row.get(0),
    )?;

    if workout_count > 0 {
        eprintln!(
            "Warning: {} workout entries reference the exercise '{}'.",
            workout_count, name
        );
        eprintln!("These entries will remain but will reference a now-deleted exercise definition.");
        // Consider adding a --force flag or prompt? For now, just warn.
    }

    // Proceed with deletion from exercises table
    let rows_affected = conn
        .execute("DELETE FROM exercises WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;

    if rows_affected == 0 {
         Err(DbError::ExerciseNotFound(identifier.to_string()).into()) // Should be caught earlier, but double-check
    } else {
        Ok(rows_affected as u64)
    }
}

// Helper to map row to ExerciseDefinition, handling potential invalid type string
fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
     let id: i64 = row.get(0)?;
     let name: String = row.get(1)?;
     let type_str: String = row.get(2)?;
     let muscles: Option<String> = row.get(3)?;

     let ex_type = ExerciseType::try_from(type_str.as_str())
         .map_err(|e| {
            eprintln!("Warning: Invalid exercise type '{}' in DB for exercise '{}' (ID {}): {}", type_str, name, id, e);
            // Create a rusqlite::Error to propagate
             rusqlite::Error::FromSqlConversionFailure(
                 2, // Column index
                 rusqlite::types::Type::Text,
                 Box::<dyn std::error::Error + Send + Sync>::from(e.to_string())
             )
         })?;

     Ok(ExerciseDefinition { id, name, type_: ex_type, muscles })
}


/// Retrieves an exercise definition by its name (case-insensitive).
pub fn get_exercise_by_name(conn: &Connection, name: &str) -> Result<Option<ExerciseDefinition>> {
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE name = ?1 COLLATE NOCASE")
        .map_err(DbError::QueryFailed)?;
    let result = stmt
        .query_row(params![name], map_row_to_exercise_definition)
        .optional() // Makes it return Option<Result<T>>
        .map_err(DbError::QueryFailed)?; // Handle query errors

    // result is Option<ExerciseDefinition>, return it directly
     Ok(result)
}


/// Retrieves an exercise definition by its ID.
pub fn get_exercise_by_id(conn: &Connection, id: i64) -> Result<Option<ExerciseDefinition>> {
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE id = ?1")
        .map_err(DbError::QueryFailed)?;
    let result = stmt
        .query_row(params![id], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)?;
     Ok(result)
}

/// Retrieves an exercise definition by trying ID first, then name.
pub fn get_exercise_by_identifier(
    conn: &Connection,
    identifier: &str,
) -> Result<Option<ExerciseDefinition>> {
    if let Ok(id) = identifier.parse::<i64>() {
        // Try parsing as ID
        match get_exercise_by_id(conn, id)? {
            Some(exercise) => return Ok(Some(exercise)), // Found by ID
            None => {
                // It parsed as an ID, but no exercise with that ID exists.
                // Should we try it as a name *also*? It's unlikely an exercise
                // would be named purely numerically AND clash with a non-existent ID.
                // Let's assume if it parses as ID, it IS an ID.
                return Ok(None); // ID not found
            }
        }
    } else {
        // Not a valid i64, treat as name (case-insensitive)
        get_exercise_by_name(conn, identifier)
    }
}

/// Lists defined exercises, optionally filtering by type and/or muscle.
pub fn list_exercises(
    conn: &Connection,
    type_filter: Option<ExerciseType>,
    muscle_filter: Option<&str>,
) -> Result<Vec<ExerciseDefinition>> {
    let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(t) = type_filter {
        sql.push_str(&format!(" AND type = ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(t.to_string()));
    }
    if let Some(m) = muscle_filter {
        sql.push_str(&format!(" AND muscles LIKE ?{}", params_vec.len() + 1));
        params_vec.push(Box::new(format!("%{}%", m)));
    }
    sql.push_str(" ORDER BY name ASC");

    let params_slice: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let exercise_iter = stmt
        .query_map(params_slice.as_slice(), map_row_to_exercise_definition)
        .map_err(DbError::QueryFailed)?;

    let mut exercises = Vec::new();
    for exercise_result in exercise_iter {
        exercises.push(exercise_result?); // Propagate mapping errors
    }
    Ok(exercises)
}
