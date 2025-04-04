mod cli; // Keep cli module for parsing args

use anyhow::{bail, Context, Result};
use chrono::{Utc, Duration, NaiveDate}; // Keep Duration if needed, remove if not
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use std::io::{stdin, Write}; // For prompts
use std::collections::HashMap;

use workout_tracker_lib::{
    AppService, ConfigError, ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters,
    PBInfo, PBType, VolumeFilters, PbMetricScope // Import PB types
};

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
                Err(e) => bail!("Error creating exercise: {}", e), // Handles unique name error message from service
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
                     println!("Successfully updated exercise definition '{}' ({} row(s) affected).", identifier, rows);
                    if name.is_some() {
                        println!("Note: If the name was changed, corresponding workout entries and aliases were also updated.");
                    }
                 }
                 Err(e) => bail!("Error editing exercise '{}': {}", identifier, e), // Handles unique name error from service
            }
        }
        cli::Commands::DeleteExercise { identifier } => {
             match service.delete_exercise(&identifier) {
                Ok(0) => println!("Exercise definition '{}' not found.", identifier), // Should ideally be Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted exercise definition '{}' ({} row(s) affected). Associated aliases were also deleted.", identifier, rows),
                Err(e) => bail!("Error deleting exercise '{}': {}", identifier, e),
            }
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise, date, // Feature 3: Get date from args
            sets, reps, weight, duration, notes,
            implicit_type, implicit_muscles,
        } => {
            let identifier_trimmed = exercise.trim();
             if identifier_trimmed.is_empty() {
                 bail!("Exercise identifier cannot be empty for adding a workout.");
             }

             // Determine if bodyweight might be needed *before* calling add_workout
             let mut bodyweight_to_use: Option<f64> = None;
             let mut needs_bw_check = false;

             // Peek at exercise type using the service resolver
             let exercise_def_peek = service.get_exercise_by_identifier_service(identifier_trimmed)?;
             if let Some(ref def) = exercise_def_peek {
                 if def.type_ == ExerciseType::BodyWeight { needs_bw_check = true; }
             } else if let Some(cli::ExerciseTypeCli::BodyWeight) = implicit_type {
                 needs_bw_check = true;
             }


             // If bodyweight exercise, check config and potentially prompt
             if needs_bw_check {
                  match service.get_required_bodyweight() {
                     Ok(bw) => {
                         bodyweight_to_use = Some(bw);
                         println!("Using configured bodyweight: {} {:?} (+ {} additional)",
                             bw, service.config.units, weight.unwrap_or(0.0));
                     }
                     Err(ConfigError::BodyweightNotSet(_)) => {
                         if service.config.prompt_for_bodyweight {
                             match prompt_and_set_bodyweight_cli(&mut service) {
                                 Ok(bw_from_prompt) => {
                                     bodyweight_to_use = Some(bw_from_prompt);
                                      println!("Using newly set bodyweight: {} {:?} (+ {} additional)",
                                        bw_from_prompt, service.config.units, weight.unwrap_or(0.0));
                                 }
                                 Err(ConfigError::BodyweightPromptCancelled) => {
                                     bail!("Bodyweight not set. Cannot add bodyweight exercise entry. Prompt disabled.");
                                 }
                                 Err(e) => bail!("Failed to get bodyweight via prompt: {}", e),
                             }
                         } else {
                              bail!(ConfigError::BodyweightNotSet(service.get_config_path().to_path_buf()));
                         }
                     }
                     Err(e) => bail!("Error checking bodyweight configuration: {}", e),
                  }
             }


            // Call the service add_workout method
            let db_implicit_type = implicit_type.map(cli_type_to_db_type);
            let units = service.config.units;
            match service.add_workout(
                identifier_trimmed, date, // Pass date
                sets, reps, weight, duration, notes,
                db_implicit_type, implicit_muscles, // Pass implicit creation details
                bodyweight_to_use, // Pass the resolved bodyweight (if applicable)
            ) {
                 Ok((id, pb_info_opt)) => { // Feature 4: Get PB info
                     // Use the potentially *canonical* name if implicit creation happened or alias used
                     let final_exercise_name = service.get_exercise_by_identifier_service(identifier_trimmed)?
                                                     .map(|def| def.name)
                                                     .unwrap_or_else(|| identifier_trimmed.to_string()); // Fallback if refetch fails (shouldn't happen)
                     println!(
                         "Successfully added workout for '{}' on {} ID: {}",
                         final_exercise_name, date.format("%Y-%m-%d"), id
                     );

                     // Handle PB notification (Feature 4)
                     if let Some(pb_info) = pb_info_opt {
                         handle_pb_notification(&mut service, &pb_info, units)?;
                     }
                 }
                 Err(e) => bail!("Error adding workout: {}", e),
             }
        }

        cli::Commands::EditWorkout { id, exercise, sets, reps, weight, duration, notes, date } => { // Feature 3: Handle date edit
            match service.edit_workout(id, exercise, sets, reps, weight, duration, notes, date) { // Pass date to service
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
             // Determine date based on flags or explicit date arg
             let effective_date = if today_flag { Some(Utc::now().date_naive()) }
                                else if yesterday_flag { Some((Utc::now() - Duration::days(1)).date_naive()) }
                                else { date };

             let workouts_result = if let Some(ex_ident) = nth_last_day_exercise {
                  let n = nth_last_day_n.context("Missing N value for --nth-last-day")?;
                  // Service method now resolves identifier internally
                  service.list_workouts_for_exercise_on_nth_last_day(&ex_ident, n)
             } else {
                  let db_type_filter = type_.map(cli_type_to_db_type);
                  // Limit applies only if no date filter and not using nth_last_day
                  let effective_limit = if effective_date.is_none() && nth_last_day_n.is_none() { Some(limit) } else { None };

                  // Service method now resolves identifier internally if provided
                  let filters = WorkoutFilters {
                      exercise_name: exercise.as_deref(), // Pass identifier directly
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
        cli::Commands::Volume {
            exercise, date, type_, muscle, limit_days, start_date, end_date,
        } => {
            // Use explicit date if provided, otherwise use range or limit
            let (eff_start_date, eff_end_date) = if let Some(d) = date {
                 (Some(d), Some(d)) // Filter for a single specific day
            } else {
                 (start_date, end_date) // Use provided range or None
            };

            let db_type_filter = type_.map(cli_type_to_db_type);
            let effective_limit = if eff_start_date.is_none() && eff_end_date.is_none() { Some(limit_days) } else { None };

            let filters = VolumeFilters {
                exercise_name: exercise.as_deref(),
                start_date: eff_start_date,
                end_date: eff_end_date,
                exercise_type: db_type_filter,
                muscle: muscle.as_deref(),
                limit_days: effective_limit,
            };

            match service.calculate_daily_volume(filters) {
                Ok(volume_data) if volume_data.is_empty() => {
                    println!("No workout volume found matching the criteria.");
                }
                Ok(volume_data) => {
                     print_volume_table(volume_data, service.config.units);
                }
                Err(e) => bail!("Error calculating workout volume: {}", e),
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
        // --- Alias Commands (Feature 1) ---
        cli::Commands::Alias { alias_name, exercise_identifier } => {
            match service.create_alias(&alias_name, &exercise_identifier) {
                Ok(()) => println!("Successfully created alias '{}' for exercise '{}'.", alias_name, exercise_identifier),
                Err(e) => bail!("Error creating alias: {}", e),
            }
        }
        cli::Commands::Unalias { alias_name } => {
            match service.delete_alias(&alias_name) {
                Ok(0) => println!("Alias '{}' not found.", alias_name), // Should be Err from service
                Ok(rows) => println!("Successfully deleted alias '{}' ({} row(s) affected).", alias_name, rows),
                Err(e) => bail!("Error deleting alias '{}': {}", alias_name, e),
            }
        }
        cli::Commands::ListAliases => {
            match service.list_aliases() {
                Ok(aliases) if aliases.is_empty() => println!("No aliases defined."),
                Ok(aliases) => print_alias_table(aliases),
                Err(e) => bail!("Error listing aliases: {}", e),
            }
        }
        cli::Commands::SetUnits { units } => { // Feature 3
            let db_units = match units {
                cli::UnitsCli::Metric => Units::Metric,
                cli::UnitsCli::Imperial => Units::Imperial,
            };
            match service.set_units(db_units) {
                Ok(()) => {
                    println!("Successfully set default units to: {:?}", db_units);
                    println!("Config file updated: {:?}", service.get_config_path());
                }
                Err(e) => bail!("Error setting units: {}", e),
             }
         }
        // --- Config/Path Commands ---
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
         cli::Commands::SetPbNotification { enabled } => { // Feature 4
            match service.set_pb_notification(enabled) {
                Ok(()) => {
                    println!(
                        "Successfully {} Personal Best notifications.",
                        if enabled { "enabled" } else { "disabled" }
                    );
                    println!("Config file updated: {:?}", service.get_config_path());
                }
                Err(e) => bail!("Error updating PB notification setting: {}", e),
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
    std::io::stdout().flush().map_err(ConfigError::Io)?;

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


/// Handles PB notification logic, including prompting if config not set (Feature 4)
fn handle_pb_notification(service: &mut AppService, pb_info: &PBInfo, units: Units) -> Result<()> {
    let print_notification = match service.check_pb_notification_config() {
        Ok(enabled) => enabled, // Config is set, use the value
        Err(ConfigError::PbNotificationNotSet) => {
             // Config not set, prompt the user
             prompt_and_set_pb_notification_cli(service)? // Returns true if user enables, false if disables
        }
        Err(e) => return Err(e.into()), // Other config error
    };

    if print_notification {
        print_pb_message(pb_info, units);
    }
    Ok(())
}

/// Prints the formatted PB message.
fn print_pb_message(pb_info: &PBInfo, units: Units) {
    let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
    println!("*********************************");
    println!("*     🎉 Personal Best! 🎉     *");
    match pb_info.pb_type {
        PBType::Weight => {
             println!("* New Max Weight: {:.2} {} {}",
                pb_info.new_weight.unwrap_or(0.0),
                weight_unit_str,
                pb_info.previous_weight.map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
            );
        },
        PBType::Reps => {
            println!("* New Max Reps: {} {}",
                pb_info.new_reps.unwrap_or(0),
                pb_info.previous_reps.map_or("".to_string(), |p| format!("(Previous: {})", p))
            );
        },
        PBType::Both => {
            println!("* New Max Weight: {:.2} {} {}",
                pb_info.new_weight.unwrap_or(0.0),
                weight_unit_str,
                pb_info.previous_weight.map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
            );
            println!("* New Max Reps: {} {}",
                pb_info.new_reps.unwrap_or(0),
                pb_info.previous_reps.map_or("".to_string(), |p| format!("(Previous: {})", p))
            );
        },
    }
     println!("*********************************");
}

/// Interactive prompt for PB notification setting, specific to the CLI (Feature 4)
/// Updates the service's config and saves it. Returns the chosen setting (true/false).
fn prompt_and_set_pb_notification_cli(service: &mut AppService) -> Result<bool, ConfigError> {
    println!("You achieved a Personal Best!");
    print!("Do you want to be notified about PBs in the future? (Y/N): ");
    std::io::stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("y") {
        println!("Okay, enabling future PB notifications.");
        service.set_pb_notification(true)?;
        Ok(true)
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future PB notifications.");
        service.set_pb_notification(false)?;
        Ok(false)
    } else {
         // Invalid input, treat as cancellation for this time, don't update config
         println!("Invalid input. PB notifications remain unset for now.");
         Err(ConfigError::PbNotificationPromptCancelled) // Indicate cancellation/invalid input
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
            Cell::new("Timestamp (UTC)").fg(header_color), // Display full timestamp
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
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()), // Format for display
            Cell::new(workout.exercise_name), // Canonical name
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

/// Prints aliases in a formatted table. (Feature 1)
fn print_alias_table(aliases: std::collections::HashMap<String, String>) {
    let mut table = Table::new();
    let header_color = Color::Magenta; // Use a different color for aliases
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Alias").fg(header_color),
            Cell::new("Canonical Exercise Name").fg(header_color),
        ]);

    // Sort aliases for consistent output
    let mut sorted_aliases: Vec<_> = aliases.into_iter().collect();
    sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

    for (alias, canonical_name) in sorted_aliases {
        table.add_row(vec![
            Cell::new(alias),
            Cell::new(canonical_name),
        ]);
    }
    println!("{table}");
}

fn print_volume_table(volume_data: Vec<(NaiveDate, String, f64)>, units: Units) {
    let mut table = Table::new();
    let header_color = workout_tracker_lib::parse_color("Yellow") // Use a different color for volume
        .map(Color::from)
        .unwrap_or(Color::Yellow);

    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Date").fg(header_color),
            Cell::new("Exercises").fg(header_color),
            Cell::new(format!("Volume (Sets*Reps*Weight {})", weight_unit_str)).fg(header_color),
        ]);

    for (date, exercise_name, volume) in volume_data { // Destructure tuple
        table.add_row(vec![
            Cell::new(date.format("%Y-%m-%d")),
            Cell::new(exercise_name), // Added exercise name cell
            Cell::new(format!("{:.2}", volume)),
        ]);
    }
    println!("{table}");
}
