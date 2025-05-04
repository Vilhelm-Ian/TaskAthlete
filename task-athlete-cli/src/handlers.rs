//! This module contains handler functions for each CLI subcommand.

use crate::{cli, output}; // Use local modules
use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use comfy_table::Color;
use std::io::{stdin, stdout, Write};
use task_athlete_lib::{
    AddWorkoutParams, AppService, ConfigError, DbError, EditWorkoutParams, ExerciseType, Units,
    VolumeFilters, WorkoutFilters,
};

// --- Helper Functions ---

/// Converts CLI ExerciseType enum to DB ExerciseType enum (from lib)
fn cli_type_to_db_type(cli_type: cli::ExerciseTypeCli) -> ExerciseType {
    match cli_type {
        cli::ExerciseTypeCli::Resistance => ExerciseType::Resistance,
        cli::ExerciseTypeCli::Cardio => ExerciseType::Cardio,
        cli::ExerciseTypeCli::BodyWeight => ExerciseType::BodyWeight,
    }
}

/// Converts CLI Units enum to DB Units enum (from lib)
fn cli_units_to_db_units(cli_units: cli::UnitsCli) -> Units {
    match cli_units {
        cli::UnitsCli::Metric => Units::Metric,
        cli::UnitsCli::Imperial => Units::Imperial,
    }
}

/// Gets the appropriate header color from config or uses a default.
fn get_header_color(service: &AppService, default: Color) -> Color {
    task_athlete_lib::parse_color(&service.config.theme.header_color)
        .map(Color::from)
        .unwrap_or(default)
}

/// Prompts user for current bodyweight if config allows.
/// Logs it via the service if entered.
/// Returns Ok(Some(weight)) if logged, Ok(None) if cancelled or 'N' entered, Err on failure.
/// Needs mutable service to potentially disable prompt config.
fn prompt_and_log_bodyweight_cli(service: &mut AppService) -> Result<Option<f64>, ConfigError> {
    println!("\nBodyweight is required for this exercise type, but none is logged yet.");
    println!("We can use your latest logged weight if available, or you can enter it now.");
    println!(
        "Please enter your current bodyweight (in {:?}).",
        service.config.units
    );
    print!("Enter weight, 'N' to disable this prompt, or press Enter to skip: ");
    stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.is_empty() {
        println!("Skipping bodyweight entry for this workout. Using 0 base weight.");
        Ok(None)
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!(
            "Using 0 base weight for this workout. Use 'log-bodyweight' to add entries later."
        );
        service.disable_bodyweight_prompt()?;
        Ok(None)
    } else {
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!(
                    "Logging bodyweight: {:.2} {:?}",
                    weight, service.config.units
                );
                match service.add_bodyweight_entry(Utc::now(), weight) {
                    Ok(_) => Ok(Some(weight)),
                    Err(e) => {
                        eprintln!("Error logging bodyweight to database: {}", e);
                        Err(ConfigError::InvalidBodyweightInput(format!(
                            "Failed to save bodyweight: {}",
                            e
                        )))
                    }
                }
            }
            Ok(_) => Err(ConfigError::InvalidBodyweightInput(
                "Weight must be a positive number.".to_string(),
            )),
            Err(e) => Err(ConfigError::InvalidBodyweightInput(format!(
                "Could not parse '{}': {}",
                trimmed_input, e
            ))),
        }
    }
}

/// Handles PB notification logic, including prompting if config not set.
/// Needs mutable service to potentially update config via prompt.
fn handle_pb_notification(
    service: &mut AppService,
    pb_info: &task_athlete_lib::PBInfo,
) -> Result<()> {
    let config = &service.config; // Immutable borrow first
    let relevant_pb_achieved = (pb_info.weight.achieved && config.pb_notifications.notify_weight)
        || (pb_info.reps.achieved && config.pb_notifications.notify_reps)
        || (pb_info.duration.achieved && config.pb_notifications.notify_duration)
        || (pb_info.distance.achieved && config.pb_notifications.notify_distance);

    if !relevant_pb_achieved {
        return Ok(());
    }

    // Now check global setting, may need mutable borrow if prompt needed
    let global_notifications_enabled = match service.check_pb_notification_config() {
        Ok(enabled) => enabled,
        Err(ConfigError::PbNotificationNotSet) => {
            // Prompt needs mutable access
            prompt_and_set_pb_notification_cli(service)?
        }
        Err(e) => return Err(e.into()),
    };

    if global_notifications_enabled {
        // Pass immutable config borrow to output function
        output::print_pb_message_details(pb_info, service.config.units, &service.config);
    }
    Ok(())
}

/// Interactive prompt for PB notification setting. Updates config via service.
fn prompt_and_set_pb_notification_cli(service: &mut AppService) -> Result<bool, ConfigError> {
    println!("You achieved a Personal Best!");
    print!("Do you want to be notified about PBs in the future? (Y/N): ");
    stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.eq_ignore_ascii_case("y") {
        println!("Okay, enabling future PB notifications.");
        service.set_pb_notification_enabled(true)?;
        Ok(true)
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future PB notifications.");
        service.set_pb_notification_enabled(false)?;
        Ok(false)
    } else {
        println!("Invalid input. PB notifications remain unset for now.");
        Err(ConfigError::PbNotificationPromptCancelled)
    }
}

// --- Command Handlers ---

pub fn handle_create_exercise(
    service: &mut AppService,
    name: String,
    type_: cli::ExerciseTypeCli,
    muscles: Option<String>,
    log_flags: Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)>,
) -> Result<()> {
    let db_type = cli_type_to_db_type(type_);
    match service.create_exercise(&name, db_type, log_flags, muscles.as_deref()) {
        Ok(id) => println!(
            "Successfully defined exercise: '{}' (Type: {}, Muscles: {}) ID: {}",
            name.trim(),
            db_type,
            muscles.as_deref().unwrap_or("None"),
            id
        ),
        Err(e) => bail!("Error creating exercise: {}", e),
    }
    Ok(())
}

pub fn handle_edit_exercise(
    service: &mut AppService,
    identifier: String,
    name: Option<String>,
    type_: Option<cli::ExerciseTypeCli>,
    muscles: Option<String>,
    log_flags: Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)>,
) -> Result<()> {
    let db_type = type_.map(cli_type_to_db_type);
    let muscles_update = match muscles {
        Some(ref s) if s.trim().is_empty() => Some(None),
        Some(ref s) => Some(Some(s.trim())),
        None => None,
    };

    match service.edit_exercise(
        &identifier,
        name.as_deref(),
        db_type,
        log_flags,
        muscles_update,
    ) {
        Ok(rows) => {
            println!(
                "Successfully updated exercise definition '{}' ({} row(s) affected).",
                identifier, rows
            );
            if name.is_some() {
                println!("Note: If the name was changed, corresponding workout entries and aliases were also updated.");
            }
        }
        Err(e) => bail!("Error editing exercise '{}': {}", identifier, e),
    }
    Ok(())
}

pub fn handle_delete_exercise(service: &mut AppService, identifiers: Vec<String>) -> Result<()> {
    match service.delete_exercise(&identifiers) {
        Ok(rows) => println!("Successfully deleted exercise definition '{:?}' ({} row(s) affected). Associated aliases were also deleted.", identifiers, rows),
        Err(e) => bail!("Error deleting exercise: {}", e),
    }
    Ok(())
}

pub fn handle_add_workout(
    service: &mut AppService,
    exercise: String,
    date_arg: NaiveDate,
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    distance: Option<f64>,
    notes: Option<String>,
    implicit_type: Option<cli::ExerciseTypeCli>,
    implicit_muscles: Option<String>,
) -> Result<()> {
    let identifier_trimmed = exercise.trim();
    if identifier_trimmed.is_empty() {
        bail!("Exercise identifier cannot be empty for adding a workout.");
    }

    let mut bodyweight_to_use: Option<f64> = None;
    let mut needs_bw_check = false;

    let exercise_def_peek = service.get_exercise_by_identifier_service(identifier_trimmed)?;
    if let Some(ref def) = exercise_def_peek {
        if def.type_ == ExerciseType::BodyWeight {
            needs_bw_check = true;
        }
    } else if let Some(cli::ExerciseTypeCli::BodyWeight) = implicit_type {
        needs_bw_check = true;
    }

    if needs_bw_check {
        match service.get_latest_bodyweight() {
            Ok(Some(bw)) => {
                bodyweight_to_use = Some(bw);
                println!(
                    "Using latest logged bodyweight: {:.2} {:?} (+ {} additional)",
                    bw,
                    service.config.units,
                    weight.unwrap_or(0.0)
                );
            }
            Ok(None) if service.config.prompt_for_bodyweight => {
                match prompt_and_log_bodyweight_cli(service) {
                    Ok(maybe_logged_bw) => bodyweight_to_use = maybe_logged_bw.or(Some(0.0)), // Use logged or 0 if skipped
                    Err(e) => bail!("Cannot add bodyweight exercise: {}", e),
                }
            }
            Ok(None) => {
                println!("Bodyweight prompting disabled or skipped. Using 0 base weight for this exercise.");
                bodyweight_to_use = Some(0.0);
            }
            Err(e) => bail!("Error checking bodyweight configuration: {}", e),
        }
    }

    // Adjust date to include current time if 'today', otherwise use noon UTC
    let timestamp = if Utc::now().date_naive() == date_arg {
        Utc::now()
    } else {
        date_arg
            .and_hms_opt(12, 0, 0)
            .map(|naive| Utc.from_utc_datetime(&naive))
            .context("Internal error creating timestamp from date")?
    };

    let db_implicit_type = implicit_type.map(cli_type_to_db_type);
    let units = service.config.units; // Capture units before potential mutable borrow

    let workout_params = AddWorkoutParams {
        exercise_identifier: identifier_trimmed,
        date: timestamp, // Use the adjusted timestamp
        sets,
        reps,
        weight,
        distance,
        duration,
        notes,
        bodyweight_to_use,
        implicit_type: db_implicit_type,
        implicit_muscles,
    };

    match service.add_workout(workout_params) {
        Ok((id, pb_info_opt)) => {
            let final_exercise_name = service
                .get_exercise_by_identifier_service(identifier_trimmed)?
                .map(|def| def.name)
                .unwrap_or_else(|| identifier_trimmed.to_string());
            println!(
                "Successfully added workout for '{}' on {} ID: {}",
                final_exercise_name,
                timestamp.format("%Y-%m-%d"), // Format the timestamp date part
                id
            );

            if let Some(pb_info) = pb_info_opt {
                // Needs mutable service reference for potential prompt
                handle_pb_notification(service, &pb_info)?;
            }
        }
        Err(e) => bail!("Error adding workout: {}", e),
    }
    Ok(())
}

pub fn handle_edit_workout(
    service: &mut AppService,
    id: i64,
    exercise: Option<String>,
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    distance: Option<f64>,
    notes: Option<String>,
    date: Option<NaiveDate>,
    body_weight: Option<f64>,
) -> Result<()> {
    match service.edit_workout(EditWorkoutParams {
        id,
        new_exercise_identifier: exercise,
        new_sets: sets,
        new_reps: reps,
        new_weight: weight,
        new_duration: duration,
        new_distance_arg: distance,
        new_notes: notes,
        new_date: date,
        new_bodyweight: body_weight,
    }) {
        Ok(rows) => println!(
            "Successfully updated workout ID {} ({} row(s) affected).",
            id, rows
        ),
        Err(e) => bail!("Error editing workout ID {}: {}", id, e),
    }
    Ok(())
}

pub fn handle_delete_workout(service: &mut AppService, ids: Vec<i64>) -> Result<()> {
    match service.delete_workouts(&ids) {
        Ok(deleted_ids) => println!(
            "Successfully deleted workout ID(s) {:?} ({} row(s) affected).",
            deleted_ids,
            deleted_ids.len()
        ),
        Err(e) => bail!("Error deleting workout(s): {}", e),
    }
    Ok(())
}

pub fn handle_list_workouts(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
    limit: u32,
    today_flag: bool,
    yesterday_flag: bool,
    date: Option<NaiveDate>,
    exercise: Option<String>,
    type_: Option<cli::ExerciseTypeCli>,
    muscle: Option<String>,
    nth_last_day_exercise: Option<String>,
    nth_last_day_n: Option<u32>,
) -> Result<()> {
    let effective_date = if today_flag {
        Some(Utc::now().date_naive())
    } else if yesterday_flag {
        Some((Utc::now() - Duration::days(1)).date_naive())
    } else {
        date
    };

    let workouts_result = if let Some(ex_ident) = nth_last_day_exercise {
        let n = nth_last_day_n.context("Missing N value for --nth-last-day")?;
        service.list_workouts_for_exercise_on_nth_last_day(&ex_ident, n)
    } else {
        let db_type_filter = type_.map(cli_type_to_db_type);
        let effective_limit = if effective_date.is_none() && nth_last_day_n.is_none() {
            Some(limit)
        } else {
            None
        };

        let filters = WorkoutFilters {
            exercise_name: exercise.as_deref(),
            date: effective_date,
            exercise_type: db_type_filter,
            muscle: muscle.as_deref(),
            limit: effective_limit,
        };
        service.list_workouts(&filters)
    };

    match workouts_result {
        Ok(workouts) if workouts.is_empty() => {
            println!("No workouts found matching the criteria.");
        }
        Ok(workouts) => {
            if export_csv {
                output::print_workout_csv(workouts, service.config.units)?;
            } else {
                let header_color = get_header_color(service, Color::Green);
                output::print_workout_table(workouts, header_color, service.config.units);
            }
        }
        Err(e) => {
            if let Some(DbError::ExerciseNotFound(ident)) = e.downcast_ref::<DbError>() {
                println!(
                    "Exercise identifier '{}' not found. No workouts listed.",
                    ident
                );
                return Ok(()); // Graceful exit if exercise not found for filter
            }
            bail!("Error listing workouts: {}", e);
        }
    }
    Ok(())
}

pub fn handle_stats(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
    exercise: String,
) -> Result<()> {
    match service.get_exercise_stats(&exercise) {
        Ok(stats) => {
            if export_csv {
                output::print_stats_csv(&stats, service.config.units)?;
            } else {
                // Pass immutable config borrow to output function
                output::print_exercise_stats(&stats, service.config.units);
            }
        }
        Err(e) => {
            if let Some(db_err) = e.downcast_ref::<DbError>() {
                match db_err {
                    DbError::ExerciseNotFound(ident) => {
                        println!("Error: Exercise '{ident}' not found.");
                        return Ok(());
                    }
                    DbError::NoWorkoutDataFound(name) => {
                        println!(
                            "No workout data found for exercise '{name}'. Cannot calculate stats."
                        );
                        return Ok(());
                    }
                    _ => {} // Fall through for other DbErrors
                }
            }
            bail!("Error getting exercise stats for '{}': {}", exercise, e);
        }
    }
    Ok(())
}

pub fn handle_volume(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
    exercise: Option<String>,
    date: Option<NaiveDate>,
    type_: Option<cli::ExerciseTypeCli>,
    muscle: Option<String>,
    limit_days: u32,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<()> {
    let (eff_start_date, eff_end_date) = if let Some(d) = date {
        (Some(d), Some(d))
    } else {
        (start_date, end_date)
    };

    let db_type_filter = type_.map(cli_type_to_db_type);
    let effective_limit = if eff_start_date.is_none() && eff_end_date.is_none() {
        Some(limit_days)
    } else {
        None
    };

    let filters = VolumeFilters {
        exercise_name: exercise.as_deref(),
        start_date: eff_start_date,
        end_date: eff_end_date,
        exercise_type: db_type_filter,
        muscle: muscle.as_deref(),
        limit_days: effective_limit,
    };

    match service.calculate_daily_volume(&filters) {
        Ok(volume_data) if volume_data.is_empty() => {
            println!("No volume data found matching the criteria.");
            // Still print header if CSV requested
            if export_csv {
                output::print_volume_csv(volume_data, service.config.units)?;
            }
        }
        Ok(volume_data) => {
            if export_csv {
                output::print_volume_csv(volume_data, service.config.units)?;
            } else {
                let header_color = get_header_color(service, Color::Yellow);
                output::print_volume_table(volume_data, service.config.units, header_color);
            }
        }
        Err(e) => bail!("Error calculating workout volume: {}", e),
    }
    Ok(())
}

pub fn handle_list_exercises(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
    type_: Option<cli::ExerciseTypeCli>,
    muscle: Option<String>,
) -> Result<()> {
    let db_type_filter = type_.map(cli_type_to_db_type);
    match service.list_exercises(db_type_filter, muscle.as_deref()) {
        Ok(exercises) if exercises.is_empty() => {
            println!("No exercise definitions found matching the criteria.");
            if export_csv {
                output::print_exercise_definition_csv(exercises)?; // Print header only
            }
        }
        Ok(exercises) => {
            if export_csv {
                output::print_exercise_definition_csv(exercises)?;
            } else {
                let header_color = get_header_color(service, Color::Cyan);
                output::print_exercise_definition_table(exercises, header_color);
            }
        }
        Err(e) => bail!("Error listing exercises: {}", e),
    }
    Ok(())
}

pub fn handle_alias(
    service: &mut AppService,
    alias_name: String,
    exercise_identifier: String,
) -> Result<()> {
    match service.create_alias(&alias_name, &exercise_identifier) {
        Ok(()) => println!(
            "Successfully created alias '{}' for exercise '{}'.",
            alias_name, exercise_identifier
        ),
        Err(e) => bail!("Error creating alias: {}", e),
    }
    Ok(())
}

pub fn handle_unalias(service: &mut AppService, alias_name: String) -> Result<()> {
    match service.delete_alias(&alias_name) {
        Ok(rows) => println!(
            "Successfully deleted alias '{}' ({} row(s) affected).",
            alias_name, rows
        ),
        Err(e) => bail!("Error deleting alias '{}': {}", alias_name, e),
    }
    Ok(())
}

pub fn handle_list_aliases(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
) -> Result<()> {
    match service.list_aliases() {
        Ok(aliases) if aliases.is_empty() => {
            if export_csv {
                output::print_alias_csv(aliases)?; // Print header only
            } else {
                println!("No aliases defined.");
            }
        }
        Ok(aliases) => {
            if export_csv {
                output::print_alias_csv(aliases)?;
            } else {
                let header_color = get_header_color(service, Color::Magenta);
                output::print_alias_table(aliases, header_color);
            }
        }
        Err(e) => bail!("Error listing aliases: {}", e),
    }
    Ok(())
}

pub fn handle_set_units(service: &mut AppService, units: cli::UnitsCli) -> Result<()> {
    let db_units = cli_units_to_db_units(units);
    match service.set_units(db_units) {
        Ok(()) => {
            println!("Successfully set default units to: {:?}", db_units);
            println!("Config file updated: {:?}", service.get_config_path());
        }
        Err(e) => bail!("Error setting units: {}", e),
    }
    Ok(())
}

pub fn handle_log_bodyweight(service: &mut AppService, weight: f64, date: NaiveDate) -> Result<()> {
    let timestamp = date
        .and_hms_opt(12, 0, 0)
        .map(|naive_dt| Utc.from_utc_datetime(&naive_dt))
        .context("Internal error creating timestamp from date")?;

    match service.add_bodyweight_entry(timestamp, weight) {
        Ok(id) => println!(
            "Successfully logged bodyweight {} {:?} on {} (ID: {})",
            weight,
            service.config.units,
            date.format("%Y-%m-%d"),
            id
        ),
        Err(e) => bail!("Error logging bodyweight: {}", e),
    }
    Ok(())
}

pub fn handle_list_bodyweights(
    service: &AppService, // Immutable borrow sufficient
    export_csv: bool,
    limit: u32,
) -> Result<()> {
    match service.list_bodyweights(limit) {
        Ok(entries) if entries.is_empty() => {
            println!("No bodyweight entries found.");
            if export_csv {
                output::print_bodyweight_csv(entries, service.config.units)?; // Print header only
            }
        }
        Ok(entries) => {
            if export_csv {
                output::print_bodyweight_csv(entries, service.config.units)?;
            } else {
                let header_color = get_header_color(service, Color::Blue);
                output::print_bodyweight_table(&entries, service.config.units, header_color);
            }
        }
        Err(e) => bail!("Error listing bodyweights: {}", e),
    }
    Ok(())
}

pub fn handle_delete_bodyweight(service: &mut AppService, id: i64) -> Result<()> {
    match service.delete_bodyweight(id) {
        Ok(deleted_id) => println!("Successfully deleted body weight entry {deleted_id}"),
        Err(e) => bail!("Error deleting body weight entry: {}", e),
    }
    Ok(())
}

pub fn handle_set_target_weight(service: &mut AppService, weight: f64) -> Result<()> {
    match service.set_target_bodyweight(Some(weight)) {
        Ok(()) => println!(
            "Successfully set target bodyweight to {} {:?}. Config updated.",
            weight, service.config.units
        ),
        Err(e) => bail!("Error setting target bodyweight: {}", e),
    }
    Ok(())
}

pub fn handle_clear_target_weight(service: &mut AppService) -> Result<()> {
    match service.set_target_bodyweight(None) {
        Ok(()) => println!("Target bodyweight cleared. Config updated."),
        Err(e) => bail!("Error clearing target bodyweight: {}", e),
    }
    Ok(())
}

pub fn handle_set_pb_notification(service: &mut AppService, enabled: bool) -> Result<()> {
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
    Ok(())
}

pub fn handle_set_pb_notify_metric(
    service: &mut AppService,
    metric: &str,
    enabled: bool,
    setter: impl FnOnce(&mut AppService, bool) -> Result<(), ConfigError>,
) -> Result<()> {
    match setter(service, enabled) {
        Ok(()) => println!(
            "Set {} PB notification to: {}. Config updated.",
            metric, enabled
        ),
        Err(e) => bail!("Error setting {} PB notification: {}", metric, e),
    }
    Ok(())
}

pub fn handle_set_streak_interval(service: &mut AppService, days: u32) -> Result<()> {
    match service.set_streak_interval(days) {
        Ok(()) => println!("Set streak interval to {} day(s). Config updated.", days),
        Err(e) => bail!("Error setting streak interval: {}", e),
    }
    Ok(())
}
