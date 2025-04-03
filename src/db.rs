// src/db.rs
use anyhow::{Context, Result};
use core::fmt;
use chrono::{DateTime, NaiveDate, Utc };
use rusqlite::{params, Connection, Row, ToSql};
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
}

// Represents the definition of an exercise type
#[derive(Debug)]
pub struct ExerciseDefinition {
    pub id: i64,
    pub name: String,
    pub type_: ExerciseType, // Use enum for type safety
    pub muscles: Option<String>,
}

// Enum for Exercise Type in the database/logic layer
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExerciseType {
    Resistance,
    Cardio,
}

// Convert string from DB to ExerciseType
impl TryFrom<&str> for ExerciseType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "resistance" => Ok(ExerciseType::Resistance),
            "cardio" => Ok(ExerciseType::Cardio),
            _ => anyhow::bail!("Invalid exercise type string: {}", value),
        }
    }
}

// Convert ExerciseType to string for DB storage
impl fmt::Display for ExerciseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExerciseType::Resistance => write!(f, "resistance"),
            ExerciseType::Cardio => write!(f, "cardio"),
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
    #[error("I/O error")]
    Io(#[from] std::io::Error),
}

const DB_FILE_NAME: &str = "workouts.sqlite";

/// Gets the path to the SQLite database file.
/// Creates the directory if it doesn't exist.
pub fn get_db_path() -> Result<PathBuf, DbError> {
    let data_dir = dirs::data_dir().ok_or(DbError::DataDir)?;
    let app_dir = data_dir.join("workout-tracker-cli");
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }
    Ok(app_dir.join(DB_FILE_NAME))
}

/// Opens a connection to the SQLite database.
pub fn open_db<P: AsRef<Path>>(path: P) -> Result<Connection, DbError> {
    Connection::open(path).map_err(DbError::Connection)
}

/// Initializes the database by creating the 'workouts' table if it doesn't exist.
pub fn init_db(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS workouts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,           -- Store as ISO 8601 string
            exercise_name TEXT NOT NULL,
            sets INTEGER,
            reps INTEGER,
            weight REAL,                       -- Use REAL for f64
            duration_minutes INTEGER,
            notes TEXT
        )",
        [], // No parameters
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio')), -- Enforce allowed types
            muscles TEXT -- Comma-separated list or similar
        )",
        [],
    )?; // Create exercises table
    Ok(())
}

/// Adds a new workout entry to the database.
pub fn add_workout(
    conn: &Connection,
    exercise: &str,
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    notes: Option<String>,
) -> Result<i64> { // Return the ID of the inserted row
    let timestamp = Utc::now().to_rfc3339(); // Store timestamp as string

    let rows_affected = conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            timestamp,
            exercise,
            sets,
            reps,
            weight,
            duration,
            notes
        ],
    ).context("Failed to insert workout into database")?;

    if rows_affected == 0 {
         anyhow::bail!("Failed to insert workout, no rows affected.");
    }

    Ok(conn.last_insert_rowid())
}


/// Updates an existing workout entry in the database.
/// Returns the number of rows affected (should be 1 if successful).
pub fn update_workout(
    conn: &Connection,
    identifier: &str,
    exercise: Option<&str>,
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    notes: Option<&str>,
) -> Result<u64> {
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut updates = Vec::new();
    let id = if let Ok(id) = identifier.parse::<i64>() {
    id
    } else {
        // Look up by name
        if let Some(ex) = get_exercise_by_name(conn, identifier)? {
            ex.id
        } else {
            anyhow::bail!("Exercise '{}' not found", identifier);
        }
    };


    if let Some(ex) = exercise {
        updates.push("exercise_name = ?");
        params.push(Box::new(ex));
    }

    if let Some(s) = sets {
        updates.push("sets = ?");
        params.push(Box::new(s));
    }

    if let Some(r) = reps {
        updates.push("reps = ?");
        params.push(Box::new(r));
    }

    if let Some(w) = weight {
        updates.push("weight = ?");
        params.push(Box::new(w));
    }

    if let Some(d) = duration {
        updates.push("duration_minutes = ?");
        params.push(Box::new(d));
    }

    if let Some(n) = notes {
        updates.push("notes = ?");
        params.push(Box::new(n));
    }

    if updates.is_empty() {
        anyhow::bail!("No fields to update");
    }

    let mut sql = format!("UPDATE workouts SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(id));

    let params_slice: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows_affected = conn.execute(&sql, params_slice.as_slice())?;

    if rows_affected == 0 {
        anyhow::bail!("Workout not found or no changes made");
    }

    Ok(rows_affected as u64)
}

/// Deletes a workout entry from the database.
/// Returns the number of rows affected (should be 1 if successful).
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64> {
    let rows_affected = conn.execute("DELETE FROM workouts WHERE id = ?", params![id])?;
    
    if rows_affected == 0 {
        anyhow::bail!("Workout not found");
    }

    Ok(rows_affected as u64)
}

// Helper function to map a database row to a Workout struct
fn map_row_to_workout(row: &Row) -> Result<Workout, rusqlite::Error> {
    let timestamp_str: String = row.get(1)?;
    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(e),
        ))?;
    Ok(Workout {
        id: row.get(0)?,
        timestamp,
        exercise_name: row.get(2)?,
        sets: row.get(3)?,
        reps: row.get(4)?,
        weight: row.get(5)?,
        duration_minutes: row.get(6)?,
        notes: row.get(7)?,
    })
}


/// Lists workout entries from the database, ordered by timestamp descending.
pub fn list_workouts(conn: &Connection, limit: u32) -> Result<Vec<Workout>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, exercise_name, sets, reps, weight, duration_minutes, notes
         FROM workouts
         ORDER BY timestamp DESC
         LIMIT ?1",
    )?;

    let workout_iter = stmt.query_map(params![limit], map_row_to_workout)?;

    let mut workouts = Vec::new();
    for workout_result in workout_iter {
        workouts.push(workout_result.context("Failed to map row to Workout struct")?);
    }


    Ok(workouts)
}


/// Lists workouts performed on a specific date, optionally filtered by exercise.
pub fn list_workouts_for_date(
    conn: &Connection,
    date: NaiveDate,
    exercise_filter: Option<&str>,
) -> Result<Vec<Workout>> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let mut params_vec: Vec<&dyn rusqlite::types::ToSql> = Vec::new();
    params_vec.push(&date_str);
    let exercise_sql;

    let mut sql = "SELECT id, timestamp, exercise_name, sets, reps, weight, duration_minutes, notes
                   FROM workouts
                   WHERE date(timestamp) = date(?1)"
                   .to_string();

    if let Some(exercise) = exercise_filter {
        sql.push_str(" AND exercise_name = ?2");
        exercise_sql = exercise.to_sql().unwrap();
        params_vec.push(&exercise_sql); // Add exercise to params
    }

    sql.push_str(" ORDER BY timestamp ASC"); // Order chronologically within the day

    // Convert Vec<&dyn ToSql> to &[&dyn ToSql] for query_map
    let params_slice = params_vec.as_slice();

    let mut stmt = conn.prepare(&sql)?;
    let workout_iter = stmt.query_map(params_slice, map_row_to_workout)?;

    let mut workouts = Vec::new();
    for workout_result in workout_iter {
        workouts.push(workout_result.context("Failed to map row to Workout struct")?);
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
        anyhow::bail!("Nth last day must be 1 or greater.");
    }
    let offset = n - 1; // SQL OFFSET is 0-based

    // CTE to find the Nth most recent distinct date for the exercise
    let sql = "
        WITH RankedDays AS (
            SELECT DISTINCT date(timestamp) as workout_date
            FROM workouts
            WHERE exercise_name = ?1
            ORDER BY workout_date DESC
            LIMIT 1 OFFSET ?2
        )
        SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.notes
        FROM workouts w
        JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
        WHERE w.exercise_name = ?1
        ORDER BY w.timestamp ASC; -- Order chronologically within that day
    ";

    let mut stmt = conn.prepare(sql)?;
    let workout_iter = stmt.query_map(params![exercise_name, offset], map_row_to_workout)?;

    let mut workouts = Vec::new();
    for workout_result in workout_iter {
        workouts.push(workout_result.context("Failed to map row to Workout struct")?);
    }
     Ok(workouts)
 }

// ---- Exercise Definition Functions ----

/// Creates a new exercise definition in the database.
/// Returns the ID of the newly created exercise.
/// If an exercise with the same name already exists, it returns an error.
pub fn create_exercise(
    conn: &Connection,
    name: &str,
    type_: &ExerciseType,
    muscles: Option<&str>,
) -> Result<i64> {
    let type_str = type_.to_string();
    let rows_affected = conn.execute(
        "INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO NOTHING", // Prevent duplicates silently
        params![name, type_str, muscles],
    )?;

    if rows_affected == 0 {
        // Could be a conflict or another issue, check if it exists
        if get_exercise_by_name(conn, name)?.is_some() {
             anyhow::bail!("Exercise '{}' already exists.", name);
        } else {
            // If it doesn't exist after INSERT failed, something else went wrong
            anyhow::bail!("Failed to insert exercise '{}', unknown error.", name);
        }
    }

    Ok(conn.last_insert_rowid())
}

/// Updates an existing exercise definition in the database.
/// Can update name, type, and/or muscles.
/// Returns the number of rows affected (should be 1 if successful).
pub fn update_exercise(
    conn: &Connection,
    identifier: &str, // Can be name or ID
    new_name: Option<String>,
    new_type: Option<&ExerciseType>,
    new_muscles: Option<&str>,
) -> Result<u64> {
    // First try to parse as ID, if not assume it's a name
    let id = if let Ok(id) = identifier.parse::<i64>() {
        id
    } else {
        // Look up by name
        if let Some(ex) = get_exercise_by_name(conn, identifier)? {
            ex.id
        } else {
            anyhow::bail!("Exercise '{}' not found", identifier);
        }
    };

    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name {
        updates.push("name = ?");
        params.push(Box::new(name));
    }

    if let Some(t) = new_type {
        updates.push("type = ?");
        params.push(Box::new(t.to_string()));
    }

    if let Some(m) = new_muscles {
        updates.push("muscles = ?");
        params.push(Box::new(m));
    }

    if updates.is_empty() {
        anyhow::bail!("No fields to update");
    }

    let sql = format!("UPDATE exercises SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(id));

    let params_slice: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows_affected = conn.execute(&sql, params_slice.as_slice())?;

    if rows_affected == 0 {
        anyhow::bail!("Exercise not found or no changes made");
    }

    Ok(rows_affected as u64)
}

/// Deletes an exercise definition from the database.
/// Returns the number of rows affected (should be 1 if successful).
pub fn delete_exercise(conn: &Connection, identifier: &str) -> Result<u64> {
    // First try to parse as ID, if not assume it's a name
    let id = if let Ok(id) = identifier.parse::<i64>() {
        id
    } else {
        // Look up by name
        if let Some(ex) = get_exercise_by_name(conn, identifier)? {
            ex.id
        } else {
            anyhow::bail!("Exercise '{}' not found", identifier);
        }
    };

    let rows_affected = conn.execute("DELETE FROM exercises WHERE id = ?", params![id])?;
    
    if rows_affected == 0 {
        anyhow::bail!("Exercise not found");
    }

    Ok(rows_affected as u64)
}

/// Retrieves an exercise definition by its name.
pub fn get_exercise_by_name(conn: &Connection, name: &str) -> Result<Option<ExerciseDefinition>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, muscles FROM exercises WHERE name = ?1",
    )?;
    let mut rows = stmt.query_map(params![name], |row| {
        let type_str: String = row.get(2)?;
        let ex_type = ExerciseType::try_from(type_str.as_str())
            .map_err(|e| {
                // Convert anyhow::Error to string and box it as dyn Error
                let err_msg = e.to_string();
                rusqlite::Error::FromSqlConversionFailure(
                    2, // Column index
                    rusqlite::types::Type::Text,
                    Box::<dyn std::error::Error + Send + Sync>::from(err_msg), // Box the string error
                )
            })?;
        Ok(ExerciseDefinition {
            id: row.get(0)?,
            name: row.get(1)?,
            type_: ex_type,
            muscles: row.get(3)?,
        })
    })?;

    match rows.next() {
        Some(Ok(exercise)) => Ok(Some(exercise)),
        Some(Err(e)) => Err(e.into()), // Propagate rusqlite or conversion errors
        None => Ok(None),
    }
}

/// Lists defined exercises, optionally filtering by type and/or muscle.
pub fn list_exercises(
    conn: &Connection,
    type_filter: Option<ExerciseType>,
    muscle_filter: Option<&str>,
) -> Result<Vec<ExerciseDefinition>> {
    // Base query
    let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new(); // Use Box<dyn ToSql>

    if let Some(t) = type_filter {
        sql.push_str(" AND type = ?");
        params_vec.push(Box::new(t.to_string()));
    }
    if let Some(m) = muscle_filter {
        sql.push_str(" AND muscles LIKE ?"); // Basic substring search for muscle
        params_vec.push(Box::new(format!("%{}%", m))); // Wrap muscle in % for LIKE
    }
    sql.push_str(" ORDER BY name ASC");

    // Convert Vec<Box<dyn ToSql>> to Vec<&dyn ToSql> for query_map
    let params_slice: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let exercise_iter = stmt.query_map(params_slice.as_slice(), |row| {
        let type_str: String = row.get(2)?;
        let ex_type = ExerciseType::try_from(type_str.as_str())
            .map_err(|e| {
                let err_msg = e.to_string();
                rusqlite::Error::FromSqlConversionFailure(
                    2, // Column index
                    rusqlite::types::Type::Text,
                    Box::<dyn std::error::Error + Send + Sync>::from(err_msg), // Box the string error
                )
            })?;
        Ok(ExerciseDefinition {
            id: row.get(0)?,
            name: row.get(1)?,
            type_: ex_type,
            muscles: row.get(3)?,
        })
    })?;

    let mut exercises = Vec::new();
    for exercise_result in exercise_iter {
        exercises.push(exercise_result.context("Failed to map row to ExerciseDefinition")?);
    }
    Ok(exercises)
}

