// src/main.rs
mod cli;
mod db;

use chrono::{Utc, Duration};
use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use db::{Workout, ExerciseDefinition }; 

fn main() -> Result<()> {
    // Parse command-line arguments
    let cli_args = cli::parse_args();

    // Get database path and open connection
    let db_path = db::get_db_path().context("Failed to determine database path")?;
    let conn = db::open_db(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    // Initialize database (create table if needed)
    db::init_db(&conn).context("Failed to initialize database schema")?;

    // Execute commands
    match cli_args.command {
        cli::Commands::CreateExercise { name, type_, muscles } => {
            // Convert CLI enum to DB enum
            let db_type = match type_ {
                cli::ExerciseTypeCli::Resistance => db::ExerciseType::Resistance,
                cli::ExerciseTypeCli::Cardio => db::ExerciseType::Cardio,
            };

            match db::create_exercise(&conn, &name, &db_type, muscles.as_deref()) {
                Ok(id) => println!("Successfully defined exercise: '{}' (Type: {:?}, Muscles: {}) (ID: {})",
                                   name, db_type, muscles.unwrap_or_else(|| "None".to_string()), id),
                Err(e) => {
                    // Provide more specific feedback if it already exists
                    eprintln!("Error creating exercise: {}", e);
                }
            }
        }
        cli::Commands::EditExercise {
            identifier,
            name,
            type_,
            muscles,
        } => {
            // Convert CLI enum to DB enum if provided
            let result;
            let db_type = match type_ {
                Some(type_) => {
                    result = match type_ {
                        cli::ExerciseTypeCli::Resistance => db::ExerciseType::Resistance,
                        cli::ExerciseTypeCli::Cardio => db::ExerciseType::Cardio,
                    };
                    Some(&result)
                }
                None => None
            };

            let rows_affected = db::update_exercise(
                &conn,
                &identifier,
                name,
                db_type,
                muscles.as_deref(),
            )
            .context("Failed to update exercise")?;

            println!("Successfully updated exercise ({} rows affected)", rows_affected);
        }
        cli::Commands::DeleteExercise { identifier } => {
            let rows_affected = db::delete_exercise(&conn, &identifier)
                .context("Failed to delete exercise")?;
            println!("Successfully deleted exercise ({} rows affected)", rows_affected);
        }
        cli::Commands::Add {
            exercise,
            sets,
            reps,
            weight,
            duration,
            exercise_type,
            muscles,
            notes,
        } => {
            // Implicitly create exercise definition if type/muscles are provided and it doesn't exist
            if let (Some(cli_type), Some(muscle_list)) = (&exercise_type, &muscles) {
                // Check if it exists first
                if db::get_exercise_by_name(&conn, &exercise)?.is_none() {
                    println!("Exercise '{}' not found, defining it implicitly.", exercise);
                    let db_type = match cli_type {
                         cli::ExerciseTypeCli::Resistance => db::ExerciseType::Resistance,
                         cli::ExerciseTypeCli::Cardio => db::ExerciseType::Cardio,
                    };
                    match db::create_exercise(&conn, &exercise, &db_type, Some(muscle_list)) {
                         Ok(id) => println!("Implicitly defined exercise: '{}' (ID: {})", exercise, id),
                         Err(e) => eprintln!("Warning: Failed to implicitly define exercise '{}': {}", exercise, e), // Warn but proceed with adding workout
                    }
                }
                 // If it exists, we don't update it here, just proceed to add the workout log.
            }

            // Add the workout entry regardless of whether the definition existed or was just create
            let inserted_id = db::add_workout(&conn, &exercise, sets, reps, weight, duration, notes)
                .context("Failed to add workout")?;
            println!(
                "Successfully added workout: '{}' (ID: {})",
                exercise, inserted_id
            );
        }
        cli::Commands::List {
            limit,
            today,
            yesterday,
            exercise,
            nth_last_day,
        } => {
            let workouts = if today {
                 let today_date = Utc::now().date_naive();
                 db::list_workouts_for_date(&conn, today_date, exercise.as_deref())
                     .context("Failed to retrieve today's workouts")?
            } else if yesterday {
                 let yesterday_date = (Utc::now() - Duration::days(1)).date_naive();
                 db::list_workouts_for_date(&conn, yesterday_date, exercise.as_deref())
                     .context("Failed to retrieve yesterday's workouts")?
            } else if let Some(n) = nth_last_day {
                 // 'requires' in clap ensures exercise is Some
                 let ex_name = exercise.as_deref().unwrap();
                 db::list_workouts_for_exercise_on_nth_last_day(&conn, ex_name, n)
                     .with_context(|| format!("Failed to retrieve workouts for exercise '{}' on the {}{} last day", ex_name, n, day_suffix(n)))?
            } else {
                 // Default case: use limit (no other filters applied)
                 db::list_workouts(&conn, limit).context("Failed to retrieve workouts")?
            };
            if workouts.is_empty() {
                println!("No workouts found.");
            } else {
                print_workout_table(workouts);
            }
        }
        cli::Commands::ListExercises { type_, muscle } => {
            // Convert CLI enum filter to DB enum filter
            let db_type_filter = type_.map(|t| match t {
                cli::ExerciseTypeCli::Resistance => db::ExerciseType::Resistance,
                cli::ExerciseTypeCli::Cardio => db::ExerciseType::Cardio,
            });

            let exercises = db::list_exercises(&conn, db_type_filter, muscle.as_deref())
                .context("Failed to list exercise definitions")?;
            if exercises.is_empty() {
                println!("No exercise definitions found matching the criteria.");
            } else {
                print_exercise_definition_table(exercises);
            }
        }
        cli::Commands::EditWorkout {
            identifier,
            exercise,
            sets,
            reps,
            weight,
            duration,
            notes,
        } => {
            let rows_affected = db::update_workout(
                &conn,
                &identifier,
                exercise.as_deref(),
                sets,
                reps,
                weight,
                duration,
                notes.as_deref(),
            )
            .context("Failed to update workout")?;
            println!("Successfully updated workout ({} rows affected)", rows_affected);
        }
        cli::Commands::DeleteWorkout { id } => {
            let rows_affected = db::delete_workout(&conn, id)
                .context("Failed to delete workout")?;
            println!("Successfully deleted workout ({} rows affected)", rows_affected);
        }
        cli::Commands::DbPath => {
            println!("Database file is located at: {:?}", db_path);
        }
    }

    Ok(())
}

// Helper function to print workouts in a table
fn print_workout_table(workouts: Vec<Workout>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(Color::Green),
            Cell::new("Timestamp (UTC)").fg(Color::Green),
            Cell::new("Exercise").fg(Color::Green),
            Cell::new("Sets").fg(Color::Green),
            Cell::new("Reps").fg(Color::Green),
            Cell::new("Weight").fg(Color::Green),
            Cell::new("Duration (min)").fg(Color::Green),
            Cell::new("Notes").fg(Color::Green),
        ]);

    for workout in workouts {
        table.add_row(vec![
            Cell::new(workout.id.to_string()),
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()), // Format timestamp
            Cell::new(workout.exercise_name),
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.weight.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.duration_minutes.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.notes.map_or("-".to_string(), |v| v)),
        ]);
    }

    println!("{table}");
}

fn print_exercise_definition_table(exercises: Vec<ExerciseDefinition>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(Color::Cyan),
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Type").fg(Color::Cyan),
            Cell::new("Muscles").fg(Color::Cyan),
        ]);

    for exercise in exercises {
        table.add_row(vec![
            Cell::new(exercise.id.to_string()),
            Cell::new(exercise.name),
            Cell::new(exercise.type_.to_string()), // Use Display impl
            Cell::new(exercise.muscles.map_or("-".to_string(), |v| v)),
        ]);
    }

    println!("{table}");
}

fn day_suffix(n: u32) -> &'static str {
    if n % 100 >= 11 && n % 100 <= 13 {
        "th"
    } else {
        match n % 10 {
            1 => "st", 2 => "nd", 3 => "rd", _ => "th",
        }
    }
}
