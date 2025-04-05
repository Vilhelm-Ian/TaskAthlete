//src/main.rs
mod cli; // Keep cli module for parsing args

use csv;
use anyhow::{bail, Context, Result};
use chrono::{Utc, Duration, NaiveDate}; // Keep Duration if needed, remove if not
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table, Attribute};
use std::io::{stdin, Write, stdout}; // For prompts
use std::io;

use task_athlete_lib::{
    AppService, ConfigError, ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters,
    PBInfo, VolumeFilters, DbError, ExerciseStats // Import PB types, DbError, Stats types
};

// Constants for display units
const KM_TO_MILES: f64 = 0.621371;

fn main() -> Result<()> {
    // --- Check for completion generation request FIRST ---
    let cli_args = cli::parse_args(); // Parse arguments once
    let export_csv = cli_args.export_csv;

    if let cli::Commands::GenerateCompletion { shell } = cli_args.command {
        let mut cmd = cli::build_cli_command(); // Get the command structure
        let bin_name = cmd.get_name().to_string(); // Get the binary name

        eprintln!("Generating completion script for {}...", shell); // Print to stderr
        clap_complete::generate(shell, &mut cmd, bin_name, &mut stdout()); // Print script to stdout
        return Ok(()); // Exit after generating script
    }

    // Initialize the application service (loads config, connects to DB)
    let mut service = AppService::initialize().context("Failed to initialize application service")?;


    // --- Execute Commands using AppService ---
    match cli_args.command {
        cli::Commands::GenerateCompletion { .. } => {
            // This case is handled above, but keep it exhaustive
             unreachable!("Completion generation should have exited already");
        }
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
                 // Ok(0) case is now handled by Err(DbError::ExerciseNotFound) from the service layer
                 Ok(rows) => {
                     println!("Successfully updated exercise definition '{}' ({} row(s) affected).", identifier, rows);
                    if name.is_some() {
                        println!("Note: If the name was changed, corresponding workout entries and aliases were also updated.");
                    }
                 }
                 Err(e) => bail!("Error editing exercise '{}': {}", identifier, e), // Handles unique name & not found errors from service
            }
        }
        cli::Commands::DeleteExercise { identifier } => {
             match service.delete_exercise(&identifier) {
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted exercise definition '{}' ({} row(s) affected). Associated aliases were also deleted.", identifier, rows),
                Err(e) => bail!("Error deleting exercise '{}': {}", identifier, e),
            }
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise, date, // Feature 3: Get date from args
            sets, reps, weight, duration, distance, // Add distance arg
            notes,
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
                sets, reps, weight, duration, distance, // Pass distance
                notes,
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
                         // Pass service by reference to allow prompting/config updates
                         handle_pb_notification(&mut service, &pb_info, units)?;
                     }
                 }
                 Err(e) => bail!("Error adding workout: {}", e),
             }
        }

        cli::Commands::EditWorkout { id, exercise, sets, reps, weight, duration, distance, notes, date } => { // Add distance, handle date edit
            match service.edit_workout(id, exercise, sets, reps, weight, duration, distance, notes, date) { // Pass distance and date to service
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully updated workout ID {} ({} row(s) affected).", id, rows),
                Err(e) => bail!("Error editing workout ID {}: {}", id, e),
            }
        }
        cli::Commands::DeleteWorkout { ids } => {
            match service.delete_workouts(&ids) {
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted workout ID {:?} ({} row(s) affected).", ids, rows.len()),
                Err(e) => bail!("Error deleting workout: {}", e),
                // Err(e) => bail!("Error deleting workout ID {}: {}", id, e),
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
                    if export_csv {
                        print_workout_csv(workouts, service.config.units)?;
                    } else {
                        let header_color = task_athlete_lib::parse_color(&service.config.theme.header_color)
                        .map(Color::from)
                        .unwrap_or(Color::Green); // Fallback
                        print_workout_table(workouts, header_color, service.config.units);
                    }
                }
                 Err(e) => {
                     // Handle specific case where exercise filter didn't find the exercise
                    if let Some(db_err) = e.downcast_ref::<DbError>() {
                        if let DbError::ExerciseNotFound(ident) = db_err {
                            println!("Exercise identifier '{}' not found. No workouts listed.", ident);
                            return Ok(()) // Exit gracefully
                        }
                    }
                    // Otherwise, bail with the original error
                    bail!("Error listing workouts: {}", e);
                 }
             }
        }
         cli::Commands::Stats { exercise } => {
            match service.get_exercise_stats(&exercise) {
                Ok(stats) => {
                    if export_csv {
                        print_stats_csv(&stats, service.config.units)?;
                    } else {
                        print_exercise_stats(&stats, service.config.units);
                    }
                }
                Err(e) => {
                    // Handle specific "not found" errors gracefully
                    if let Some(db_err) = e.downcast_ref::<DbError>() {
                         match db_err {
                             DbError::ExerciseNotFound(ident) => {
                                 println!("Error: Exercise '{}' not found.", ident);
                                 return Ok(());
                             }
                             DbError::NoWorkoutDataFound(name) => {
                                println!("No workout data found for exercise '{}'. Cannot calculate stats.", name);
                                return Ok(());
                             }
                             _ => {} // Fall through for other DbErrors
                         }
                    }
                    // Bail for other errors
                    bail!("Error getting exercise stats for '{}': {}", exercise, e);
                }
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
                    if export_csv {
                         print_volume_csv(volume_data, service.config.units)?;
                    } else {
                        print_volume_table(volume_data, service.config.units);
                    }
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
                    if export_csv {
                        print_exercise_definition_csv(exercises)?;
                    } else {
                         let header_color = task_athlete_lib::parse_color(&service.config.theme.header_color)
                             .map(Color::from)
                             .unwrap_or(Color::Cyan); // Fallback
                         print_exercise_definition_table(exercises, header_color);
                     }
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
                // Ok(0) handled by Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted alias '{}' ({} row(s) affected).", alias_name, rows),
                Err(e) => bail!("Error deleting alias '{}': {}", alias_name, e),
            }
        }
        cli::Commands::ListAliases => {
            match service.list_aliases() {
                Ok(aliases) if aliases.is_empty() => {
                    if export_csv {
                        print_alias_csv(aliases)?; // Print header only
                    } else {
                        println!("No aliases defined.");
                    }
                }
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
         cli::Commands::SetPbNotification { enabled } => { // Global enable/disable
            match service.set_pb_notification_enabled(enabled) {
                Ok(()) => {
                    println!(
                        "Successfully {} Personal Best notifications globally.",
                        if enabled { "enabled" } else { "disabled" }
                    );
                    println!("Config file updated: {:?}", service.get_config_path());
                }
                Err(e) => bail!("Error updating global PB notification setting: {}", e),
            }
        }
        cli::Commands::SetPbNotifyWeight { enabled } => {
            match service.set_pb_notify_weight(enabled) {
                Ok(()) => println!("Set Weight PB notification to: {}. Config updated.", enabled),
                Err(e) => bail!("Error setting weight PB notification: {}", e),
            }
        }
        cli::Commands::SetPbNotifyReps { enabled } => {
            match service.set_pb_notify_reps(enabled) {
                Ok(()) => println!("Set Reps PB notification to: {}. Config updated.", enabled),
                Err(e) => bail!("Error setting reps PB notification: {}", e),
            }
        }
        cli::Commands::SetPbNotifyDuration { enabled } => {
             match service.set_pb_notify_duration(enabled) {
                 Ok(()) => println!("Set Duration PB notification to: {}. Config updated.", enabled),
                 Err(e) => bail!("Error setting duration PB notification: {}", e),
             }
        }
        cli::Commands::SetPbNotifyDistance { enabled } => {
             match service.set_pb_notify_distance(enabled) {
                 Ok(()) => println!("Set Distance PB notification to: {}. Config updated.", enabled),
                 Err(e) => bail!("Error setting distance PB notification: {}", e),
             }
        }
        cli::Commands::SetStreakInterval { days } => {
            match service.set_streak_interval(days) {
                Ok(()) => println!("Set streak interval to {} day(s). Config updated.", days),
                Err(e) => bail!("Error setting streak interval: {}", e),
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
/// Needs mutable service to potentially update config via prompt.
fn handle_pb_notification(service: &mut AppService, pb_info: &PBInfo, units: Units) -> Result<()> {
    // Check if *any* relevant PB was achieved before checking global enabled status
    let relevant_pb_achieved =
        (pb_info.achieved_weight_pb && service.config.notify_pb_weight) ||
        (pb_info.achieved_reps_pb && service.config.notify_pb_reps) ||
        (pb_info.achieved_duration_pb && service.config.notify_pb_duration) ||
        (pb_info.achieved_distance_pb && service.config.notify_pb_distance);

    if !relevant_pb_achieved {
        return Ok(()); // No relevant PB achieved, nothing to notify
    }

    // Check if global notifications are enabled (prompt if not set)
    let global_notifications_enabled = match service.check_pb_notification_config() {
        Ok(enabled) => enabled, // Config is set, use the value
        Err(ConfigError::PbNotificationNotSet) => {
             // Config not set, prompt the user
             prompt_and_set_pb_notification_cli(service)? // Returns true if user enables, false if disables
        }
        Err(e) => return Err(e.into()), // Other config error
    };

    if global_notifications_enabled {
        // Print only the PBs that were actually achieved *and* have notifications enabled in config
        print_pb_message(pb_info, units, &service.config);
    }
    Ok(())
}

/// Prints the formatted PB message based on achieved PBs and config settings.
fn print_pb_message(pb_info: &PBInfo, units: Units, config: &task_athlete_lib::Config) {
    let mut messages = Vec::new();

    if pb_info.achieved_weight_pb && config.notify_pb_weight {
        let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
        messages.push(format!(
            "New Max Weight: {:.2} {} {}",
            pb_info.new_weight.unwrap_or(0.0),
            weight_unit_str,
            pb_info.previous_weight.map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
        ));
    }
    if pb_info.achieved_reps_pb && config.notify_pb_reps {
         messages.push(format!(
            "New Max Reps: {} {}",
            pb_info.new_reps.unwrap_or(0),
            pb_info.previous_reps.map_or("".to_string(), |p| format!("(Previous: {})", p))
        ));
    }
    if pb_info.achieved_duration_pb && config.notify_pb_duration {
        messages.push(format!(
            "New Max Duration: {} min {}",
            pb_info.new_duration.unwrap_or(0),
            pb_info.previous_duration.map_or("".to_string(), |p| format!("(Previous: {} min)", p))
        ));
    }
    if pb_info.achieved_distance_pb && config.notify_pb_distance {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_info.new_distance.unwrap_or(0.0), "km"),
            Units::Imperial => (pb_info.new_distance.unwrap_or(0.0) * KM_TO_MILES, "miles"),
        };
        let prev_dist_str = match pb_info.previous_distance {
            Some(prev_km) => {
                let (prev_val, prev_unit) = match units {
                     Units::Metric => (prev_km, "km"),
                     Units::Imperial => (prev_km * KM_TO_MILES, "miles"),
                };
                format!("(Previous: {:.2} {})", prev_val, prev_unit)
            },
            None => "".to_string(),
        };
        messages.push(format!(
            "New Max Distance: {:.2} {} {}",
            dist_val, dist_unit, prev_dist_str
        ));
    }

    if !messages.is_empty() {
        println!("*********************************");
        println!("*     🎉 Personal Best! 🎉     *");
        for msg in messages {
            println!("* {}", msg);
        }
        println!("*********************************");
    }
}

/// Interactive prompt for PB notification setting, specific to the CLI (Feature 4)
/// Updates the service's config and saves it. Returns the chosen setting (true/false).
/// Needs mutable service reference.
fn prompt_and_set_pb_notification_cli(service: &mut AppService) -> Result<bool, ConfigError> {
    println!("You achieved a Personal Best!");
    print!("Do you want to be notified about PBs in the future? (Y/N): ");
    std::io::stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("y") {
        println!("Okay, enabling future PB notifications.");
        service.set_pb_notification_enabled(true)?; // Use specific service method
        Ok(true)
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future PB notifications.");
        service.set_pb_notification_enabled(false)?; // Use specific service method
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
    let weight_unit_str = match units { Units::Metric => "(kg)", Units::Imperial => "(lbs)", };
    let distance_unit_str = match units { Units::Metric => "(km)", Units::Imperial => "(miles)", };

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
            Cell::new(format!("Distance {}", distance_unit_str)).fg(header_color), // Added distance header
            Cell::new("Notes").fg(header_color),
        ]);

    for workout in workouts {
        // Convert distance for display if necessary
        let display_distance = workout.distance.map(|km| match units {
            Units::Metric => km,
            Units::Imperial => km * KM_TO_MILES,
        });

        table.add_row(vec![
            Cell::new(workout.id.to_string()),
            Cell::new(workout.timestamp.format("%Y-%m-%d %H:%M").to_string()), // Format for display
            Cell::new(workout.exercise_name), // Canonical name
            Cell::new(workout.exercise_type.map_or("-".to_string(), |t| t.to_string())),
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.weight.map_or("-".to_string(), |v| format!("{:.2}", v))),
            Cell::new(workout.duration_minutes.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(display_distance.map_or("-".to_string(), |v| format!("{:.2}", v))), // Display formatted distance
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

/// Prints workout volume in a table
fn print_volume_table(volume_data: Vec<(NaiveDate, String, f64)>, units: Units) {
    let mut table = Table::new();
    let header_color = task_athlete_lib::parse_color("Yellow") // Use a different color for volume
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
            Cell::new("Exercise").fg(header_color), // Changed header
            Cell::new(format!("Volume (Sets*Reps*Weight {})", weight_unit_str)).fg(header_color),
        ]);

    // Aggregate volume per day/exercise before printing (data from DB is already aggregated)
    for (date, exercise_name, volume) in volume_data { // Destructure tuple
        table.add_row(vec![
            Cell::new(date.format("%Y-%m-%d")),
            Cell::new(exercise_name), // Added exercise name cell
            Cell::new(format!("{:.2}", volume)),
        ]);
    }
    println!("{table}");
}

/// Prints exercise statistics.
fn print_exercise_stats(stats: &ExerciseStats, units: Units) {
    println!("\n--- Statistics for '{}' ---", stats.canonical_name);

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
         .set_content_arrangement(ContentArrangement::Dynamic); // No headers needed for key-value

    table.add_row(vec![Cell::new("Total Workouts").add_attribute(Attribute::Bold), Cell::new(stats.total_workouts)]);
    table.add_row(vec![
        Cell::new("First Workout").add_attribute(Attribute::Bold),
        Cell::new(stats.first_workout_date.map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()))
    ]);
    table.add_row(vec![
        Cell::new("Last Workout").add_attribute(Attribute::Bold),
        Cell::new(stats.last_workout_date.map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()))
    ]);
    table.add_row(vec![
        Cell::new("Avg Workouts / Week").add_attribute(Attribute::Bold),
        Cell::new(stats.avg_workouts_per_week.map_or("N/A".to_string(), |avg| format!("{:.2}", avg)))
    ]);
     table.add_row(vec![
        Cell::new("Longest Gap").add_attribute(Attribute::Bold),
        Cell::new(stats.longest_gap_days.map_or("N/A".to_string(), |gap| format!("{} days", gap)))
    ]);

    let streak_interval_str = match stats.streak_interval_days {
        1 => "(Daily)".to_string(),
        n => format!("({}-day Interval)", n)
    };
    table.add_row(vec![
        Cell::new(format!("Current Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(if stats.current_streak > 0 { stats.current_streak.to_string() } else { "0".to_string() })
    ]);
    table.add_row(vec![
        Cell::new(format!("Longest Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(if stats.longest_streak > 0 { stats.longest_streak.to_string() } else { "0".to_string() })
    ]);

    println!("{}", table);

    // Personal Bests Section
    println!("\n--- Personal Bests ---");
    let mut pb_table = Table::new();
    pb_table.load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);

    let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
    let distance_unit_str = match units { Units::Metric => "km", Units::Imperial => "miles", };

    let mut has_pbs = false;
    if let Some(pb_weight) = stats.personal_bests.max_weight {
        pb_table.add_row(vec![Cell::new("Max Weight").add_attribute(Attribute::Bold), Cell::new(format!("{:.2} {}", pb_weight, weight_unit_str))]);
        has_pbs = true;
    }
    if let Some(pb_reps) = stats.personal_bests.max_reps {
        pb_table.add_row(vec![Cell::new("Max Reps").add_attribute(Attribute::Bold), Cell::new(pb_reps)]);
         has_pbs = true;
    }
    if let Some(pb_duration) = stats.personal_bests.max_duration_minutes {
        pb_table.add_row(vec![Cell::new("Max Duration").add_attribute(Attribute::Bold), Cell::new(format!("{} min", pb_duration))]);
        has_pbs = true;
    }
    if let Some(pb_distance_km) = stats.personal_bests.max_distance_km {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_distance_km, distance_unit_str),
            Units::Imperial => (pb_distance_km * KM_TO_MILES, distance_unit_str),
        };
        pb_table.add_row(vec![Cell::new("Max Distance").add_attribute(Attribute::Bold), Cell::new(format!("{:.2} {}", dist_val, dist_unit))]);
         has_pbs = true;
    }

    if has_pbs {
        println!("{}", pb_table);
    } else {
        println!("No personal bests recorded for this exercise yet.");
    }
    println!(); // Add a blank line at the end
}


fn print_workout_csv(workouts: Vec<Workout>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
    let distance_unit_str = match units { Units::Metric => "km", Units::Imperial => "miles", };

    // Write header
    writer.write_record(&[
        "ID",
        "Timestamp_UTC",
        "Exercise",
        "Type",
        "Sets",
        "Reps",
        &format!("Weight_{}", weight_unit_str),
        "Duration_min",
        &format!("Distance_{}", distance_unit_str),
        "Notes",
    ])?;

    for workout in workouts {
        // Convert distance for display if necessary
        let display_distance = workout.distance.map(|km| match units {
            Units::Metric => km,
            Units::Imperial => km * KM_TO_MILES,
        });

        writer.write_record(&[
            workout.id.to_string(),
            workout.timestamp.to_rfc3339(), // Use ISO 8601/RFC3339 for CSV
            workout.exercise_name,
            workout.exercise_type.map_or("".to_string(), |t| t.to_string()),
            workout.sets.map_or("".to_string(), |v| v.to_string()),
            workout.reps.map_or("".to_string(), |v| v.to_string()),
            workout.weight.map_or("".to_string(), |v| format!("{:.2}", v)),
            workout.duration_minutes.map_or("".to_string(), |v| v.to_string()),
            display_distance.map_or("".to_string(), |v| format!("{:.2}", v)),
            workout.notes.as_deref().unwrap_or("").to_string(), // Convert Option<&str> to String
        ])?;
    }

    writer.flush()?;
    Ok(())
}


fn print_alias_csv(aliases: std::collections::HashMap<String, String>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    // Write header
    writer.write_record(&[
        "Alias",
        "Canonical_Exercise_Name",
    ])?;

    // Sort aliases for consistent output
    let mut sorted_aliases: Vec<_> = aliases.into_iter().collect();
    sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

    for (alias, canonical_name) in sorted_aliases {
        writer.write_record(&[alias, canonical_name])?;
    }
    writer.flush()?;
    Ok(())
}

fn print_volume_csv(volume_data: Vec<(NaiveDate, String, f64)>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    // Write header
    writer.write_record(&[
        "Date",
        "Exercise",
        &format!("Volume_Sets*Reps*Weight_{}", weight_unit_str),
    ])?;

    for (date, exercise_name, volume) in volume_data { // Destructure tuple
        writer.write_record(&[
            date.format("%Y-%m-%d").to_string(),
            exercise_name,
            format!("{:.2}", volume),
        ])?;
    }
    writer.flush()?;
    Ok(())
 }

fn print_stats_csv(stats: &ExerciseStats, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    // Write header
    writer.write_record(&["Statistic", "Value"])?;

    // Write main stats
    writer.write_record(&["Exercise_Name", &stats.canonical_name])?;
    writer.write_record(&["Total_Workouts", &stats.total_workouts.to_string()])?;
    writer.write_record(&["First_Workout", &stats.first_workout_date.map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string())])?;
    writer.write_record(&["Last_Workout", &stats.last_workout_date.map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string())])?;
    writer.write_record(&["Avg_Workouts_Per_Week", &stats.avg_workouts_per_week.map_or("N/A".to_string(), |avg| format!("{:.2}", avg))])?;
    writer.write_record(&["Longest_Gap_Days", &stats.longest_gap_days.map_or("N/A".to_string(), |gap| gap.to_string())])?;
    writer.write_record(&["Streak_Interval_Days", &stats.streak_interval_days.to_string()])?;
    writer.write_record(&["Current_Streak", &stats.current_streak.to_string()])?;
    writer.write_record(&["Longest_Streak", &stats.longest_streak.to_string()])?;

    // Write Personal Bests
    let weight_unit_str = match units { Units::Metric => "kg", Units::Imperial => "lbs", };
    let distance_unit_str = match units { Units::Metric => "km", Units::Imperial => "miles", };

    if let Some(pb_weight) = stats.personal_bests.max_weight {
        writer.write_record(&[&format!("PB_Max_Weight_{}", weight_unit_str), &format!("{:.2}", pb_weight)])?;
    }
    if let Some(pb_reps) = stats.personal_bests.max_reps {
        writer.write_record(&["PB_Max_Reps", &pb_reps.to_string()])?;
    }
    if let Some(pb_duration) = stats.personal_bests.max_duration_minutes {
        writer.write_record(&["PB_Max_Duration_min", &pb_duration.to_string()])?;
    }
    if let Some(pb_distance_km) = stats.personal_bests.max_distance_km {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_distance_km, distance_unit_str),
            Units::Imperial => (pb_distance_km * KM_TO_MILES, distance_unit_str),
        };
        writer.write_record(&[&format!("PB_Max_Distance_{}", dist_unit), &format!("{:.2}", dist_val)])?;
    }

    writer.flush()?;
    Ok(())
}

fn print_exercise_definition_csv(exercises: Vec<ExerciseDefinition>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    // Write header
    writer.write_record(&[
        "ID",
        "Name",
        "Type",
        "Muscles",
    ])?;

    for exercise in exercises {
        writer.write_record(&[
            exercise.id.to_string(),
            exercise.name,
            exercise.type_.to_string(), // Uses Display impl from lib
            exercise.muscles.as_deref().unwrap_or("").to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
