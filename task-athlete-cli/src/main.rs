//! Main executable for the Task Athlete CLI.
//! Parses arguments, initializes services, and delegates command handling.

mod cli;
mod handlers; // NEW: Include handlers module
mod output; // NEW: Include output module

use anyhow::{Context, Result};
use std::io::stdout;
use task_athlete_lib::AppService;

fn main() -> Result<()> {
    // --- Parse Args & Handle Completion ---
    let cli_args = cli::parse_args();
    let export_csv = cli_args.export_csv; // Extract global flag early

    // Handle completion generation request *before* initializing service
    if let cli::Commands::GenerateCompletion { shell } = cli_args.command {
        let mut cmd = cli::build_cli_command();
        let bin_name = cmd.get_name().to_string();
        eprintln!("Generating completion script for {}...", shell);
        clap_complete::generate(shell, &mut cmd, bin_name, &mut stdout());
        return Ok(());
    }

    // --- Initialize Service ---
    // Needs to be mutable as many handlers modify state or config
    let mut service =
        AppService::initialize().context("Failed to initialize application service")?;

    // --- Delegate Command Handling ---
    match cli_args.command {
        // --- Exercise Definition Commands ---
        cli::Commands::CreateExercise {
            name,
            type_,
            muscles,
            duration,
            distance,
            weight,
            reps,
        } => {
            let flags = convert_flags(weight, reps, duration, distance);
            handlers::handle_create_exercise(&mut service, name, type_, muscles, flags)?
        }
        cli::Commands::EditExercise {
            identifier,
            name,
            type_,
            muscles,
            duration,
            distance,
            weight,
            reps,
        } => {
            let flags = convert_flags(duration, distance, weight, reps);
            handlers::handle_edit_exercise(&mut service, identifier, name, type_, muscles, flags)?
        }
        cli::Commands::DeleteExercise { identifiers } => {
            handlers::handle_delete_exercise(&mut service, identifiers)?
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise,
            date,
            sets,
            reps,
            weight,
            duration,
            distance,
            notes,
            implicit_type,
            implicit_muscles,
        } => handlers::handle_add_workout(
            &mut service,
            exercise,
            date,
            sets,
            reps,
            weight,
            duration,
            distance,
            notes,
            implicit_type,
            implicit_muscles,
        )?,
        cli::Commands::EditWorkout {
            id,
            exercise,
            sets,
            reps,
            weight,
            duration,
            distance,
            notes,
            date,
            bodyweight,
        } => handlers::handle_edit_workout(
            &mut service,
            id,
            exercise,
            sets,
            reps,
            weight,
            duration,
            distance,
            notes,
            date,
            bodyweight,
        )?,
        cli::Commands::DeleteWorkout { ids } => handlers::handle_delete_workout(&mut service, ids)?,

        // --- Listing and Stats Commands ---
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
        } => handlers::handle_list_workouts(
            &service,   // Immutable borrow is fine here
            export_csv, // Pass the flag
            limit,
            today_flag,
            yesterday_flag,
            date,
            exercise,
            type_,
            muscle,
            nth_last_day_exercise,
            nth_last_day_n,
        )?,
        cli::Commands::Stats { exercise } => {
            handlers::handle_stats(
                &service,   // Immutable borrow is fine here
                export_csv, // Pass the flag
                exercise,
            )?
        }
        cli::Commands::Volume {
            exercise,
            date,
            type_,
            muscle,
            limit_days,
            start_date,
            end_date,
        } => handlers::handle_volume(
            &service,   // Immutable borrow is fine here
            export_csv, // Pass the flag
            exercise, date, type_, muscle, limit_days, start_date, end_date,
        )?,
        cli::Commands::ListExercises { type_, muscle } => {
            handlers::handle_list_exercises(
                &service,   // Immutable borrow is fine here
                export_csv, // Pass the flag
                type_, muscle,
            )?
        }

        // --- Alias Commands ---
        cli::Commands::Alias {
            alias_name,
            exercise_identifier,
        } => handlers::handle_alias(&mut service, alias_name, exercise_identifier)?,
        cli::Commands::Unalias { alias_name } => {
            handlers::handle_unalias(&mut service, alias_name)?
        }
        cli::Commands::ListAliases => {
            handlers::handle_list_aliases(
                &service,   // Immutable borrow is fine here
                export_csv, // Pass the flag
            )?
        }

        // --- Config/Path Commands ---
        cli::Commands::DbPath => {
            println!("Database file is located at: {:?}", service.get_db_path());
        }
        cli::Commands::ConfigPath => {
            println!("Config file is located at: {:?}", service.get_config_path());
        }
        cli::Commands::SetUnits { units } => handlers::handle_set_units(&mut service, units)?,

        // --- Bodyweight Commands ---
        cli::Commands::LogBodyweight { weight, date } => {
            handlers::handle_log_bodyweight(&mut service, weight, date)?
        }
        cli::Commands::ListBodyweights { limit } => {
            handlers::handle_list_bodyweights(
                &service,   // Immutable borrow is fine here
                export_csv, // Pass the flag
                limit,
            )?
        }
        cli::Commands::DeleteBodyweight { id } => {
            handlers::handle_delete_bodyweight(&mut service, id)?
        }
        cli::Commands::SetTargetWeight { weight } => {
            handlers::handle_set_target_weight(&mut service, weight)?
        }
        cli::Commands::ClearTargetWeight => handlers::handle_clear_target_weight(&mut service)?,

        // --- PB Notification Settings ---
        cli::Commands::SetPbNotification { enabled } => {
            handlers::handle_set_pb_notification(&mut service, enabled)?
        }
        cli::Commands::SetPbNotifyWeight { enabled } => handlers::handle_set_pb_notify_metric(
            &mut service,
            "Weight",
            enabled,
            AppService::set_pb_notify_weight,
        )?,
        cli::Commands::SetPbNotifyReps { enabled } => handlers::handle_set_pb_notify_metric(
            &mut service,
            "Reps",
            enabled,
            AppService::set_pb_notify_reps,
        )?,
        cli::Commands::SetPbNotifyDuration { enabled } => handlers::handle_set_pb_notify_metric(
            &mut service,
            "Duration",
            enabled,
            AppService::set_pb_notify_duration,
        )?,
        cli::Commands::SetPbNotifyDistance { enabled } => handlers::handle_set_pb_notify_metric(
            &mut service,
            "Distance",
            enabled,
            AppService::set_pb_notify_distance,
        )?,
        cli::Commands::SetStreakInterval { days } => {
            handlers::handle_set_streak_interval(&mut service, days)?
        }

        // --- Completion Generation (already handled, but exhaustive match) ---
        cli::Commands::GenerateCompletion { .. } => {
            unreachable!("Completion generation should have exited earlier");
        }
    }

    Ok(())
}

fn convert_flags(
    weight: bool,
    reps: bool,
    duration: bool,
    distance: bool,
) -> Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)> {
    match (duration, distance, weight, reps) {
        (false, false, false, false) => None,
        _ => Some((Some(duration), Some(distance), Some(weight), Some(reps))),
    }
}
