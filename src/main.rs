// src/main.rs
mod cli; // Keep cli module for parsing args

use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, Utc};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use std::io::{stdin, Write}; // For potential bodyweight prompt

// Use types from the library
use workout_tracker_lib::{
    AppService, ConfigError, ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters,
};

// --- Main Function ---
fn main() -> Result<()> {
    // Initialize the application service (loads config, connects to DB)
    let mut service = AppService::initialize().context("Failed to initialize application service")?;

    // Parse command-line arguments using the cli module
    let cli_args = cli::parse_args();

    // --- Execute Commands using AppService ---
    match cli_args.command {
        // --- Exercise Definition Commands ---
        cli::Commands::CreateExercise { name, type_, muscles } => {
            let db_type = cli_type_to_db_type(type_);
            match service.create_exercise(&name, db_type, muscles.as_deref()) {
                Ok(id) => println!(
                    "Successfully defined exercise: '{}' (Type: {}, Muscles: {}) ID: {}",
                    name.trim(),
                    db_type, // Use Display impl from lib::ExerciseType
                    muscles.as_deref().unwrap_or("None"),
                    id
                ),
                Err(e) => bail!("Error creating exercise: {}", e),
            }
        }
        cli::Commands::EditExercise { identifier, name, type_, muscles, } => {
            let db_type = type_.map(cli_type_to_db_type);
            let muscles_update = match muscles {
                Some(ref s) if s.trim().is_empty() => Some(None), // Clear
                Some(ref s) => Some(Some(s.trim())),             // Set
                None => None,                                    // Don't change
            };

            match service.edit_exercise(&identifier, name.as_deref(), db_type, muscles_update) {
                 Ok(0) => println!("Exercise '{}' not found or no changes specified.", identifier),
                 Ok(rows) => {
                     println!("Successfully updated exercise definition '{}' ({} row(s) affected in exercises table).", identifier, rows);
                    if name.is_some() {
                        println!("Note: If the name was changed, corresponding workout entries were also updated (if any existed).");
                    }
                 }
                 Err(e) => bail!("Error editing exercise '{}': {}", identifier, e),
            }
        }
        cli::Commands::DeleteExercise { identifier } => {
             match service.delete_exercise(&identifier) {
                Ok(0) => println!("Exercise definition '{}' not found.", identifier), // Should ideally be Err(DbError::NotFound)
                Ok(rows) => println!("Successfully deleted exercise definition '{}' ({} row(s) affected).", identifier, rows),
                Err(e) => bail!("Error deleting exercise '{}': {}", identifier, e),
            }
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise, sets, reps, weight, duration, notes,
            implicit_type, implicit_muscles,
        } => {
            let identifier_trimmed = exercise.trim();
             if identifier_trimmed.is_empty() {
                 bail!("Exercise identifier cannot be empty for adding a workout.");
             }

             // Determine if bodyweight might be needed *before* calling add_workout
             let mut bodyweight_to_use: Option<f64> = None;
             let mut needs_bw_check = false;

             // Peek at exercise type if it exists, or if implicitly creating BodyWeight type
             let exercise_def_peek = service.get_exercise_by_identifier(identifier_trimmed)?;
             if let Some(ref def) = exercise_def_peek {
                 if def.type_ == ExerciseType::BodyWeight { needs_bw_check = true; }
             } else if let Some(cli::ExerciseTypeCli::BodyWeight) = implicit_type {
                 needs_bw_check = true;
             }


             // If bodyweight exercise, check config and potentially prompt
             if needs_bw_check {
                  match service.get_required_bodyweight() {
                     Ok(bw) => {
                         // Bodyweight is set in config
                         bodyweight_to_use = Some(bw);
                         println!("Using configured bodyweight: {} {:?} (+ {} additional)",
                             bw, service.config.units, weight.unwrap_or(0.0));
                     }
                     Err(ConfigError::BodyweightNotSet(_)) => {
                         // Bodyweight needed but not set
                         if service.config.prompt_for_bodyweight {
                             // Call the interactive prompt function (now in main.rs)
                             match prompt_and_set_bodyweight_cli(&mut service) {
                                 Ok(bw_from_prompt) => {
                                     bodyweight_to_use = Some(bw_from_prompt);
                                      println!("Using newly set bodyweight: {} {:?} (+ {} additional)",
                                        bw_from_prompt, service.config.units, weight.unwrap_or(0.0));
                                 }
                                 Err(ConfigError::BodyweightPromptCancelled) => {
                                     // User cancelled, service config was updated to not prompt again. Bail out.
                                     bail!("Bodyweight not set. Cannot add bodyweight exercise entry. Prompt disabled.");
                                 }
                                 Err(e) => {
                                     // Other error during prompt (IO, parse)
                                     bail!("Failed to get bodyweight via prompt: {}", e);
                                 }
                             }
                         } else {
                             // Prompting disabled, and bodyweight not set. Bail out.
                              bail!(ConfigError::BodyweightNotSet(service.get_config_path().to_path_buf()));
                         }
                     }
                     Err(e) => { // Other config error
                         bail!("Error checking bodyweight configuration: {}", e);
                     }
                  }
             }


            // Now call the service add_workout method
            let db_implicit_type = implicit_type.map(cli_type_to_db_type);

            match service.add_workout(
                identifier_trimmed, sets, reps, weight, duration, notes,
                db_implicit_type, implicit_muscles, // Pass implicit creation details
                bodyweight_to_use, // Pass the resolved bodyweight (if applicable)
            ) {
                 Ok(id) => {
                     // Use the potentially *canonical* name if implicit creation happened
                     let final_exercise_name = service.get_exercise_by_identifier(identifier_trimmed)?
                                                     .map(|def| def.name)
                                                     .unwrap_or_else(|| identifier_trimmed.to_string()); // Fallback if refetch fails
                     println!(
                         "Successfully added workout for '{}' ID: {}",
                         final_exercise_name, id
                     );
                 }
                 Err(e) => bail!("Error adding workout: {}", e),
             }
        }

        cli::Commands::EditWorkout { id, exercise, sets, reps, weight, duration, notes, } => {
            match service.edit_workout(id, exercise, sets, reps, weight, duration, notes) {
                Ok(0) => println!("Workout ID {} not found or no changes specified.", id),
                Ok(rows) => println!("Successfully updated workout ID {} ({} row(s) affected).", id, rows),
                Err(e) => bail!("Error editing workout ID {}: {}", id, e),
            }
        }
        cli::Commands::DeleteWorkout { id } => {
            match service.delete_workout(id) {
                Ok(0) => println!("Workout ID {} not found.", id),
                Ok(rows) => println!("Successfully deleted workout ID {} ({} row(s) affected).", id, rows),
                Err(e) => bail!("Error deleting workout ID {}: {}", id, e),
            }
        }

        cli::Commands::List {
            limit, today_flag, yesterday_flag, date, exercise, type_, muscle,
            nth_last_day_exercise, nth_last_day_n,
        } => {
             let workouts_result = if let Some(ex_name) = nth_last_day_exercise {
                  let n = nth_last_day_n.context("Missing N value for --nth-last-day")?;
                  service.list_workouts_for_exercise_on_nth_last_day(&ex_name, n)
             } else {
                  let effective_date = if today_flag { Some(Utc::now().date_naive()) }
                                   else if yesterday_flag { Some((Utc::now() - chrono::Duration::days(1)).date_naive()) }
                                   else { date };
                  let db_type_filter = type_.map(cli_type_to_db_type);
                  let effective_limit = if effective_date.is_none() && nth_last_day_n.is_none() { Some(limit) } else { None };

                  let filters = WorkoutFilters {
                      exercise_name: exercise.as_deref(),
                      date: effective_date,
                      exercise_type: db_type_filter,
                      muscle: muscle.as_deref(),
                      limit: effective_limit,
                  };
                  service.list_workouts(filters)
             };

             match workouts_result {
                Ok(workouts) if workouts.is_empty() => {
                    println!("No workouts found matching the criteria.");
                }
                Ok(workouts) => {
                    let header_color = workout_tracker_lib::parse_color(&service.config.theme.header_color)
                        .map(Color::from)
                        .unwrap_or(Color::Green); // Fallback
                    print_workout_table(workouts, header_color, service.config.units);
                }
                Err(e) => bail!("Error listing workouts: {}", e),
             }
        }
        cli::Commands::ListExercises { type_, muscle } => {
            let db_type_filter = type_.map(cli_type_to_db_type);
            match service.list_exercises(db_type_filter, muscle.as_deref()) {
                Ok(exercises) if exercises.is_empty() => {
                    println!("No exercise definitions found matching the criteria.");
                }
                Ok(exercises) => {
                     let header_color = workout_tracker_lib::parse_color(&service.config.theme.header_color)
                         .map(Color::from)
                         .unwrap_or(Color::Cyan); // Fallback
                     print_exercise_definition_table(exercises, header_color);
                }
                Err(e) => bail!("Error listing exercises: {}", e),
            }
        }
        cli::Commands::DbPath => {
            println!("Database file is located at: {:?}", service.get_db_path());
        }
        cli::Commands::ConfigPath => {
            println!("Config file is located at: {:?}", service.get_config_path());
        }
        cli::Commands::SetBodyweight { weight } => {
            match service.set_bodyweight(weight) {
                 Ok(()) => {
                     println!( "Successfully set bodyweight to: {} {:?}", weight, service.config.units );
                     println!("Config file updated: {:?}", service.get_config_path());
                 }
                 Err(e) => bail!("Error setting bodyweight: {}", e),
            }
        }
    }

    Ok(())
}

// --- CLI Specific Helper Functions ---

/// Converts CLI ExerciseType enum to DB ExerciseType enum (from lib)
fn cli_type_to_db_type(cli_type: cli::ExerciseTypeCli) -> ExerciseType {
    match cli_type {
        cli::ExerciseTypeCli::Resistance => ExerciseType::Resistance,
        cli::ExerciseTypeCli::Cardio => ExerciseType::Cardio,
        cli::ExerciseTypeCli::BodyWeight => ExerciseType::BodyWeight,
    }
}

/// Interactive prompt for bodyweight, specific to the CLI.
/// Updates the service's config and saves it.
fn prompt_and_set_bodyweight_cli(service: &mut AppService) -> Result<f64, ConfigError> {
    // Prompt is needed (caller should ensure service.config.prompt_for_bodyweight is true)
    println!("Bodyweight is required for this exercise type but is not set.");
    println!("Please enter your current bodyweight (in {:?}).", service.config.units);
    print!("Enter weight, or 'N' to not be asked again (use 'set-bodyweight' later): ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?; // Use ConfigError::Io
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!("Please use the 'set-bodyweight <weight>' command to set it manually.");
        // Update config via service method to handle saving
        service.disable_bodyweight_prompt()?;
        Err(ConfigError::BodyweightPromptCancelled)
    } else {
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!("Setting bodyweight to {} {:?}", weight, service.config.units);
                // Update config via service method
                service.set_bodyweight(weight)?; // This also saves the config
                Ok(weight)
            }
            Ok(_) => Err(ConfigError::InvalidBodyweightInput("Weight must be a positive number.".to_string())),
            Err(e) => Err(ConfigError::InvalidBodyweightInput(format!("Could not parse '{}': {}", trimmed_input, e))),
        }
    }
}


// --- Table Printing Functions (Remain in CLI) ---

/// Prints workout entries in a formatted table.
fn print_workout_table(workouts: Vec<Workout>, header_color: Color, units: Units) {
    let mut table = Table::new();
    let weight_unit_str = match units {
        Units::Metric => "(kg)",
        Units::Imperial => "(lbs)",
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
            Cell::new(format!("Weight {}", weight_unit_str)).fg(header_color),
            Cell::new("Duration (min)").fg(header_color),
            Cell::new("Notes").fg(header_color),
        ]);

    for workout in workouts {
        table.add_row(vec![
            Cell::new(workout.id.to_string()),
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()),
            Cell::new(workout.exercise_name),
            Cell::new(workout.exercise_type.map_or("-".to_string(), |t| t.to_string())),
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.weight.map_or("-".to_string(), |v| format!("{:.2}", v))),
            Cell::new(workout.duration_minutes.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.notes.as_deref().unwrap_or("-")),
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
            Cell::new(exercise.type_.to_string()), // Uses Display impl from lib
            Cell::new(exercise.muscles.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
}

/// Generates the correct suffix for ordinal numbers (needed for CLI output)
fn day_suffix(n: u32) -> &'static str {
    if n % 100 >= 11 && n % 100 <= 13 { "th" }
    else { match n % 10 { 1 => "st", 2 => "nd", 3 => "rd", _ => "th" } }
}

// --- Keep src/cli.rs as it is ---
// No changes needed in src/cli.rs itself.
