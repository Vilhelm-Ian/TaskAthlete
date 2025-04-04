// src/main.rs
mod cli;
mod db;
mod config; // Add the config module

use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, Utc};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use db::{ExerciseDefinition, Workout, WorkoutFilters}; // Import WorkoutFilters
use std::path::PathBuf; // Import PathBuf

// --- Main Function ---
fn main() -> Result<()> {
    // Load configuration first
    let config_path = config::get_config_path()
        .context("Failed to determine configuration file path")?;
    let mut config = config::load_config()
        .context(format!("Failed to load config from {:?}", config_path))?;

    // Parse command-line arguments
    let cli_args = cli::parse_args();

    // Get database path and open connection
    let db_path = db::get_db_path().context("Failed to determine database path")?;
    let mut conn = db::open_db(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    // Initialize database (create tables if needed)
    db::init_db(&conn).context("Failed to initialize database schema")?;

    // --- Execute Commands ---
    match cli_args.command {
        // --- Exercise Definition Commands ---
        cli::Commands::CreateExercise { name, type_, muscles } => {
            let db_type = cli_type_to_db_type(type_);
            let exercise_name_trimmed = name.trim(); // Trim whitespace
             if exercise_name_trimmed.is_empty() {
                 bail!("Exercise name cannot be empty.");
            }

            match db::create_exercise(&conn, exercise_name_trimmed, &db_type, muscles.as_deref()) {
                Ok(id) => println!(
                    "Successfully defined exercise: '{}' (Type: {}, Muscles: {}) ID: {}",
                    exercise_name_trimmed,
                    db_type, // Use Display impl
                    muscles.unwrap_or_else(|| "None".to_string()),
                    id
                ),
                Err(e) => bail!("Error creating exercise: {}", e), // Use bail for cleaner exit
            }
        }
        cli::Commands::EditExercise {
            identifier,
            name,
            type_,
            muscles,
        } => {
             let identifier_trimmed = identifier.trim();
             if identifier_trimmed.is_empty() {
                 bail!("Exercise identifier cannot be empty.");
             }
            let db_type = type_.map(cli_type_to_db_type); // Map Option<CliType> to Option<DbType>

            // We need Option<Option<&str>> for muscles to allow clearing it
            let muscles_update: Option<Option<&str>> = match muscles {
                 Some(ref s) if s.trim().is_empty() => Some(None), // Explicitly clear if empty string passed
                 Some(ref s) => Some(Some(s.as_str())),          // Set to new value
                 None => None,                                     // Don't update muscles
            };


            let rows_affected = db::update_exercise(
                &mut conn,
                identifier_trimmed,
                name.as_deref(), // Pass Option<&str>
                db_type.as_ref(), // Pass Option<&DbType>
                muscles_update, // Pass Option<Option<&str>>
            )
            .context(format!("Failed to update exercise '{}'", identifier))?;

            println!("Successfully updated exercise definition '{}' ({} row(s) affected in exercises table).", identifier, rows_affected);
            if name.is_some() {
                 println!("Note: If the name was changed, corresponding workout entries were also updated.");
            }
        }
        cli::Commands::DeleteExercise { identifier } => {
            let identifier_trimmed = identifier.trim();
            if identifier_trimmed.is_empty() {
                 bail!("Exercise identifier cannot be empty.");
            }
            let rows_affected = db::delete_exercise(&conn, identifier_trimmed)
                .context(format!("Failed to delete exercise '{}'", identifier))?;
            println!("Successfully deleted exercise definition '{}' ({} row(s) affected).", identifier, rows_affected);
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise,
            sets,
            reps,
            weight,
            duration,
            notes,
            implicit_type,
            implicit_muscles,
        } => {
            let exercise_identifier_trimmed = exercise.trim();
            if exercise_identifier_trimmed.is_empty() {
                bail!("Exercise identifier cannot be empty for adding a workout.");
            }

            // 1. Find or implicitly create the Exercise Definition
            let mut exercise_def = db::get_exercise_by_identifier(&conn, exercise_identifier_trimmed)
                .context("Failed to query exercise definition")?;

            if exercise_def.is_none() {
                if let (Some(cli_type), Some(muscle_list)) = (implicit_type, implicit_muscles) {
                    println!("Exercise '{}' not found, defining it implicitly...", exercise_identifier_trimmed);
                    let db_type = cli_type_to_db_type(cli_type);
                    let muscles_opt = if muscle_list.trim().is_empty() { None } else { Some(muscle_list.as_str())};

                    match db::create_exercise(&conn, exercise_identifier_trimmed, &db_type, muscles_opt) {
                        Ok(id) => {
                             println!("Implicitly defined exercise: '{}' (ID: {})", exercise_identifier_trimmed, id);
                            exercise_def = Some(db::get_exercise_by_id(&conn, id)
                                .context("Failed to fetch newly created exercise definition")?
                                .ok_or_else(|| anyhow::anyhow!("Failed to re-fetch implicitly created exercise ID {}", id))?); // Should exist
                        }
                        Err(e) => bail!("Failed to implicitly define exercise '{}': {}", exercise_identifier_trimmed, e), // Fail hard if implicit creation fails
                    }
                } else {
                    // Not found and no implicit creation info provided
                    bail!(
                        "Exercise '{}' not found. Define it first using 'create-exercise' or provide --type and --implicit-muscles when adding.",
                        exercise_identifier_trimmed
                    );
                }
            }

            // We now have a valid exercise_def (either found or created)
            let current_exercise_def = exercise_def.unwrap(); // Safe to unwrap here

            // 2. Check and potentially prompt for bodyweight
            let mut final_weight = weight; // Start with weight from CLI args (can be None)

            if current_exercise_def.type_ == db::ExerciseType::BodyWeight {
                match config.bodyweight {
                    Some(bw) => {
                         // Bodyweight already set, add optional additional weight from args
                         final_weight = Some(bw + weight.unwrap_or(0.0));
                         println!("Using configured bodyweight: {} {:?} (+ {} additional) = {} total",
                             bw, config.units, weight.unwrap_or(0.0), final_weight.unwrap());
                    }
                    None => {
                        // Bodyweight not set, try prompting (handles 'N' internally)
                        match config::prompt_and_set_bodyweight(&mut config) {
                             Ok(bw_from_prompt) => {
                                 final_weight = Some(bw_from_prompt + weight.unwrap_or(0.0));
                                 println!("Using newly set bodyweight: {} {:?} (+ {} additional) = {} total",
                                    bw_from_prompt, config.units, weight.unwrap_or(0.0), final_weight.unwrap());
                             }
                             Err(config::ConfigError::BodyweightPromptCancelled) => {
                                 // User pressed 'N', config saved, but we can't proceed with this add
                                 bail!("Bodyweight not set. Cannot add bodyweight exercise entry.");
                             }
                             Err(e) => {
                                // Other error during prompt (IO, parse, etc.)
                                bail!("Failed to get bodyweight: {}", e);
                             }
                         }
                    }
                }
            }

            // 3. Add the workout entry using the canonical exercise name and final weight
            let inserted_id = db::add_workout(
                &conn,
                &current_exercise_def.name, // Use canonical name
                sets,
                reps,
                final_weight, // Use calculated weight
                duration,
                notes,
            )
            .context("Failed to add workout to database")?;

            println!(
                "Successfully added workout: '{}' (using name '{}') ID: {}",
                exercise_identifier_trimmed, current_exercise_def.name, inserted_id
            );
        }

        cli::Commands::EditWorkout {
            id,
            exercise,
            sets,
            reps,
            weight,
            duration,
            notes,
        } => {
            // Fetch the *new* exercise definition if the exercise identifier is being changed.
            // This is important if we were to re-apply bodyweight logic on edit (but we aren't currently).
            let new_exercise_def_opt = if let Some(ref ex_ident) = exercise {
                 db::get_exercise_by_identifier(&conn, ex_ident.trim())
                     .context(format!("Failed to find new exercise definition: '{}'", ex_ident))?
                     // Ensure the new exercise actually exists if specified
                     .or_else(|| {
                         eprintln!("Error: New exercise '{}' specified for edit does not exist.", ex_ident);
                         None // Treat as error or handle differently? For now, let update fail later if needed.
                     })
            } else {
                 None // Exercise not being changed
            };

            // Use the canonical name from the fetched definition if exercise was changed and found
            let new_exercise_name: Option<&str> = match (&exercise, &new_exercise_def_opt) {
                (Some(_), Some(def)) => Some(&def.name), // Use canonical name of the *new* exercise
                (Some(ident), None) => { bail!("Cannot edit workout: Specified new exercise '{}' not found.", ident); }, // Explicitly fail if new exercise doesn't exist
                (None, _) => None, // Exercise name not being updated
            };


            // Note: Weight is updated directly. We do NOT re-apply bodyweight logic here.
            // If the user changes a workout *to* a bodyweight type, they need to manually adjust weight if desired.
            let rows_affected = db::update_workout(
                &conn,
                id,
                new_exercise_name, // Pass Option<&str> of canonical name
                sets,
                reps,
                weight, // Pass Option<f64> directly
                duration,
                notes.as_deref(), // Pass Option<&str>
            )
            .context(format!("Failed to update workout ID {}", id))?;

            println!("Successfully updated workout ID {} ({} row(s) affected).", id, rows_affected);
        }
        cli::Commands::DeleteWorkout { id } => {
            let rows_affected = db::delete_workout(&conn, id)
                .context(format!("Failed to delete workout ID {}", id))?;
            println!("Successfully deleted workout ID {} ({} row(s) affected).", id, rows_affected);
        }

        cli::Commands::List {
            limit,
            today_flag,
            yesterday_flag,
            date,
            exercise,
            type_,
            muscle,
            nth_last_day_exercise,
            nth_last_day_n,
        } => {
            let workouts = if let Some(ex_name) = nth_last_day_exercise {
                 let n = nth_last_day_n.context("Missing N value for --nth-last-day")?; // Should be guaranteed by clap 'requires'
                 db::list_workouts_for_exercise_on_nth_last_day(&conn, &ex_name, n)
                     .with_context(|| format!("Failed to retrieve workouts for exercise '{}' on the {}{} last day", ex_name, n, day_suffix(n)))?

             } else {
                 // Use the general filtered list function
                 let effective_date = if today_flag {
                    Some(Utc::now().date_naive())
                 } else if yesterday_flag {
                    Some((Utc::now() - chrono::Duration::days(1)).date_naive())
                 } else {
                    date // Use the parsed date from --date if provided
                 };

                 let db_type_filter = type_.map(cli_type_to_db_type);

                 // Apply limit only if no date or nth_day filter is active
                 let effective_limit = if effective_date.is_none() { Some(limit) } else { None };


                 let filters = WorkoutFilters {
                     exercise_name: exercise.as_deref(),
                     date: effective_date,
                     exercise_type: db_type_filter,
                     muscle: muscle.as_deref(),
                     limit: effective_limit,
                 };

                 db::list_workouts_filtered(&conn, filters)
                     .context("Failed to retrieve workouts with specified filters")?
             };


            if workouts.is_empty() {
                println!("No workouts found matching the criteria.");
            } else {
                let header_color = config::parse_color(&config.theme.header_color)
                     .map_err(|e| eprintln!("Warning: Invalid header color '{}' in config: {}", config.theme.header_color, e))
                     .map(Color::from) // Convert StandardColor to comfy_table::Color
                     .unwrap_or(Color::Green); // Fallback color

                print_workout_table(workouts, header_color, config.units);
            }
        }
        cli::Commands::ListExercises { type_, muscle } => {
            let db_type_filter = type_.map(cli_type_to_db_type);

            let exercises = db::list_exercises(&conn, db_type_filter, muscle.as_deref())
                .context("Failed to list exercise definitions")?;
            if exercises.is_empty() {
                println!("No exercise definitions found matching the criteria.");
            } else {
                 let header_color = config::parse_color(&config.theme.header_color)
                     .map_err(|e| eprintln!("Warning: Invalid header color '{}' in config: {}", config.theme.header_color, e))
                     .map(Color::from)
                     .unwrap_or(Color::Cyan); // Fallback color (Cyan for exercises)

                print_exercise_definition_table(exercises, header_color);
            }
        }
        cli::Commands::DbPath => {
            println!("Database file is located at: {:?}", db_path);
        }
        cli::Commands::ConfigPath => {
            println!("Config file is located at: {:?}", config_path);
        }
        cli::Commands::SetBodyweight { weight } => {
             if weight <= 0.0 {
                bail!("Bodyweight must be a positive number.");
             }
             config.bodyweight = Some(weight);
             config.prompt_for_bodyweight = true; // Re-enable prompt if they set it manually? Or keep it as is? Let's keep it as user set it last.
             config::save_config(&config_path, &config)
                 .context("Failed to save updated bodyweight to config file")?;
             println!(
                "Successfully set bodyweight to: {} {:?}",
                weight, config.units
             );
             println!("Config file updated: {:?}", config_path);
        }
    }

    Ok(())
}

// --- Helper Functions ---

/// Converts CLI ExerciseType enum to DB ExerciseType enum
fn cli_type_to_db_type(cli_type: cli::ExerciseTypeCli) -> db::ExerciseType {
    match cli_type {
        cli::ExerciseTypeCli::Resistance => db::ExerciseType::Resistance,
        cli::ExerciseTypeCli::Cardio => db::ExerciseType::Cardio,
        cli::ExerciseTypeCli::BodyWeight => db::ExerciseType::BodyWeight,
    }
}

/// Prints workout entries in a formatted table.
fn print_workout_table(workouts: Vec<Workout>, header_color: Color, units: config::Units) {
    let mut table = Table::new();
    let weight_unit_str = match units {
        config::Units::Metric => "(kg)",   // Assuming kg for metric
        config::Units::Imperial => "(lbs)", // Assuming lbs for imperial
    };

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(header_color),
            Cell::new("Timestamp (UTC)").fg(header_color),
            Cell::new("Exercise").fg(header_color),
            Cell::new("Type").fg(header_color),
            Cell::new("Sets").fg(header_color),
            Cell::new("Reps").fg(header_color),
            Cell::new(format!("Weight {}", weight_unit_str)).fg(header_color), // Add unit hint
            Cell::new("Duration (min)").fg(header_color),
            Cell::new("Notes").fg(header_color),
        ]);

    for workout in workouts {
        table.add_row(vec![
            Cell::new(workout.id.to_string()),
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()),
            Cell::new(workout.exercise_name),
            Cell::new(workout.exercise_type.map_or("-".to_string(), |t| t.to_string())), // Use Display
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.weight.map_or("-".to_string(), |v| format!("{:.2}", v))), // Format weight
            Cell::new(workout.duration_minutes.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.notes.as_deref().unwrap_or("-")), // Handle Option<String>
        ]);
    }

    println!("{table}");
}

/// Prints exercise definitions in a formatted table.
fn print_exercise_definition_table(exercises: Vec<ExerciseDefinition>, header_color: Color) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("ID").fg(header_color),
            Cell::new("Name").fg(header_color),
            Cell::new("Type").fg(header_color),
            Cell::new("Muscles").fg(header_color),
        ]);

    for exercise in exercises {
        table.add_row(vec![
            Cell::new(exercise.id.to_string()),
            Cell::new(exercise.name),
            Cell::new(exercise.type_.to_string()), // Use Display impl
            Cell::new(exercise.muscles.as_deref().unwrap_or("-")), // Handle Option<String>
        ]);
    }

    println!("{table}");
}

/// Generates the correct suffix for ordinal numbers (1st, 2nd, 3rd, 4th).
fn day_suffix(n: u32) -> &'static str {
    if n % 100 >= 11 && n % 100 <= 13 {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    }
}
