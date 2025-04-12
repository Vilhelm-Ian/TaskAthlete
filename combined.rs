//task-athlete-cli/src/cli.rs
use chrono::{Duration, NaiveDate, Utc};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(author, version, about = "A CLI tool to track workouts", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(long, global = true)]
    pub export_csv: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum ExerciseTypeCli {
    Resistance,
    Cardio,
    BodyWeight,
}

// Custom parser for date strings and shorthands
pub fn parse_date_shorthand(s: &str) -> Result<NaiveDate, String> {
    match s.to_lowercase().as_str() {
        "today" => Ok(Utc::now().date_naive()),
        "yesterday" => Ok((Utc::now() - Duration::days(1)).date_naive()),
        _ => {
            // Try parsing YYYY-MM-DD first
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Ok(date)
            }
            // Try parsing DD.MM.YYYY next
            else if let Ok(date) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
                Ok(date)
            }
            // Try parsing YYYY/MM/DD
            else if let Ok(date) = NaiveDate::parse_from_str(s, "%Y/%m/%d") {
                Ok(date)
            } else {
                Err(format!(
                    "Invalid date format: '{}'. Use 'today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, or YYYY/MM/DD.", // Updated help message
                    s
                ))
            }
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Define a new exercise type
    CreateExercise {
        /// Name of the exercise (e.g., "Bench Press", "Running") - Must be unique (case-insensitive)
        #[arg(short, long)]
        name: String,
        /// Type of exercise
        #[arg(short = 't', long, value_enum)] // Changed short arg
        type_: ExerciseTypeCli,
        /// Comma-separated list of target muscles (e.g., "chest,triceps,shoulders")
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Delete an exercise definition
    DeleteExercise {
        /// ID, Name, or Alias of the exercise to delete
        identifiers: Vec<String>,
    },
    /// Edit an exercise definition
    EditExercise {
        /// ID, Name, or Alias of the exercise to edit
        identifier: String,
        /// New name for the exercise (must be unique)
        #[arg(short, long)]
        name: Option<String>,
        /// New type for the exercise
        #[arg(short = 't', long, value_enum)] // Changed short arg
        type_: Option<ExerciseTypeCli>,
        /// New comma-separated list of target muscles
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Add a new workout entry
    Add {
        /// Name, ID, or Alias of the exercise (will prompt to create if not found and type/muscles given)
        #[arg(short = 'e', long)] // Added short alias
        exercise: String,

        /// Number of sets performed
        #[arg(short, long)]
        sets: Option<i64>,

        /// Number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,

        /// Weight used (e.g., kg, lbs). For Bodyweight exercises, this is *additional* weight.
        #[arg(short, long)]
        weight: Option<f64>,

        /// Duration in minutes (for cardio or timed exercises)
        #[arg(short = 'd', long)] // Added short alias
        duration: Option<i64>,

        /// Distance covered (e.g., km, miles)
        #[arg(short = 'l', long)] // Use 'l' for distance (length)
        distance: Option<f64>,

        /// Additional notes about the workout
        #[arg(short, long)]
        notes: Option<String>,

        /// Date of the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand, default_value = "today")]
        // Feature 3
        date: NaiveDate,

        // Optional fields for implicit exercise creation during 'add' if exercise not found
        #[arg(
            long = "type",
            value_enum,
            requires = "implicit-muscles",
            id = "implicit-exercise-type"
        )]
        implicit_type: Option<ExerciseTypeCli>, // Renamed to avoid clash with filter

        #[arg(long, requires = "implicit-exercise-type", id = "implicit-muscles")]
        implicit_muscles: Option<String>, // Renamed to avoid clash with filter
    },
    /// Edit an existing workout entry
    EditWorkout {
        /// ID of the workout entry to edit
        id: i64, // Use ID for editing specific entries
        /// New exercise Name, ID or Alias for the workout
        #[arg(short = 'e', long)] // Added short alias
        exercise: Option<String>,
        /// New number of sets performed
        #[arg(short, long)]
        sets: Option<i64>,
        /// New number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,
        /// New weight used (absolute value, bodyweight logic NOT reapplied on edit)
        #[arg(short, long)]
        weight: Option<f64>,
        /// New duration in minutes
        #[arg(short = 'd', long)] // Added short alias
        duration: Option<i64>,
        /// New distance covered (e.g., km, miles)
        #[arg(short = 'l', long)] // Use 'l' for distance
        distance: Option<f64>,
        /// New additional notes
        #[arg(short, long)]
        notes: Option<String>,
        /// New date for the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand)] // Feature 3 (for editing date)
        date: Option<NaiveDate>,
    },
    /// Delete a workout entry
    DeleteWorkout {
        /// ID of the workout to delete
        ids: Vec<i64>,
    },
    /// List workout entries with filters
    List {
        /// Filter by exercise Name, ID or Alias
        #[arg(short = 'e', long, conflicts_with = "nth_last_day_exercise")]
        exercise: Option<String>,

        /// Filter by a specific date ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY)
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["today_flag", "yesterday_flag", "nth_last_day_exercise"])]
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>, // Short 'm'

        /// Show only the last N entries (when no date/day filters used)
        #[arg(short = 'n', long, default_value_t = 20, conflicts_with_all = &["today_flag", "yesterday_flag", "date", "nth_last_day_exercise"])]
        limit: u32,

        // Keep flags for backward compatibility or preference, but date is more versatile
        #[arg(long, conflicts_with_all = &["yesterday_flag", "date", "nth_last_day_exercise", "limit"])]
        today_flag: bool,
        #[arg(long, conflicts_with_all = &["today_flag", "date", "nth_last_day_exercise", "limit"])]
        yesterday_flag: bool,

        /// Show workouts for the Nth most recent day a specific exercise (Name, ID, Alias) was performed
        #[arg(long, value_name = "EXERCISE_IDENTIFIER", requires = "nth_last_day_n", conflicts_with_all = &["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_exercise: Option<String>,
        #[arg(long, value_name = "N", requires = "nth_last_day_exercise", conflicts_with_all = &["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_n: Option<u32>,
    },
    /// List defined exercise types
    ListExercises {
        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Filter by a target muscle (matches if the muscle is in the list)
        #[arg(short = 'm', long)] // short 'm'
        muscle: Option<String>,
    },
    /// Show statistics for a specific exercise
    Stats {
        /// Name, ID, or Alias of the exercise to show stats for
        #[arg(short = 'e', long)]
        exercise: String,
    },
    /// Create an alias for an existing exercise
    Alias {
        // Feature 1
        /// The alias name (e.g., "bp") - Must be unique
        alias_name: String,
        /// The ID, Name, or existing Alias of the exercise to alias
        exercise_identifier: String,
    },
    /// Delete an exercise alias
    Unalias {
        // Feature 1
        /// The alias name to delete
        alias_name: String,
    },
    /// List all defined exercise aliases
    ListAliases, // Feature 1
    DbPath,
    /// Log your bodyweight on a specific date
    LogBodyweight {
        /// Your bodyweight
        weight: f64,
        /// Date of measurement ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand, default_value = "today")]
        date: NaiveDate,
    },
    /// List logged bodyweight entries
    ListBodyweights {
        /// Show only the last N entries
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,
    },
    /// Set your target bodyweight in the config file
    SetTargetWeight {
        weight: f64,
    },
    /// Clear your target bodyweight from the config file
    ClearTargetWeight,
    /// Show the path to the database file
    ConfigPath,
    /// Enable or disable Personal Best (PB) notifications globally
    SetPbNotification {
        // Feature 4
        /// Enable PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Enable or disable Personal Best (PB) notifications for Weight
    SetPbNotifyWeight {
        /// Enable weight PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Enable or disable Personal Best (PB) notifications for Reps
    SetPbNotifyReps {
        /// Enable reps PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Enable or disable Personal Best (PB) notifications for Duration
    SetPbNotifyDuration {
        /// Enable duration PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Enable or disable Personal Best (PB) notifications for Distance
    SetPbNotifyDistance {
        /// Enable distance PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Set the interval in days for calculating streaks
    SetStreakInterval {
        /// Number of days allowed between workouts to maintain a streak (e.g., 1 for daily, 2 for every other day)
        #[arg(value_parser = clap::value_parser!(u32).range(1..))] // Ensure at least 1 day
        days: u32,
    },
    /// Show total workout volume (sets*reps*weight) per day
    Volume {
        // Feature 1
        /// Filter by exercise Name, ID or Alias
        #[arg(short = 'e', long)]
        exercise: Option<String>,

        /// Filter by a specific date ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, Weekday Name)
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["start_date", "end_date", "limit_days"])]
        // Corrected conflicts
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>,

        /// Show only the last N days with workouts (when no date/range filters used)
        #[arg(short = 'n', long, default_value_t = 7, conflicts_with_all = &["date", "start_date", "end_date"])]
        // Corrected conflicts
        limit_days: u32,

        // Optional date range
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["date", "limit_days"])]
        // Corrected conflicts
        start_date: Option<NaiveDate>,
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["date", "limit_days"], requires="start_date")]
        // Corrected conflicts and added requires
        end_date: Option<NaiveDate>,
    },
    /// Set default units (Metric/Imperial)
    SetUnits {
        // Feature 3
        #[arg(value_enum)]
        units: UnitsCli,
    },
    GenerateCompletion {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

// Function to parse CLI arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

pub fn build_cli_command() -> clap::Command {
    Cli::command()
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum UnitsCli {
    Metric,
    Imperial,
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (cli)
    use chrono::{Duration, NaiveDate, Utc};

    #[test]
    fn test_date_parsing_today() {
        let result = parse_date_shorthand("today").unwrap();
        let today = Utc::now().date_naive();
        assert_eq!(result, today);
    }

    #[test]
    fn test_date_parsing_yesterday() {
        let result = parse_date_shorthand("yesterday").unwrap();
        let yesterday = Utc::now().date_naive() - Duration::days(1);
        assert_eq!(result, yesterday);
    }

    #[test]
    fn test_date_parsing_yyyy_mm_dd() {
        let result = parse_date_shorthand("2023-10-27").unwrap();
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
    }

    #[test]
    fn test_date_parsing_dd_mm_yyyy() {
        let result = parse_date_shorthand("27.10.2023").unwrap();
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
    }

    #[test]
    fn test_date_parsing_yyyy_slash_mm_dd() {
        let result = parse_date_shorthand("2023/10/27").unwrap();
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 10, 27).unwrap());
    }

    #[test]
    fn test_date_parsing_case_insensitive() {
        let result_today = parse_date_shorthand("ToDaY").unwrap();
        let today = Utc::now().date_naive();
        assert_eq!(result_today, today);

        let result_yesterday = parse_date_shorthand("yEsTeRdAy").unwrap();
        let yesterday = Utc::now().date_naive() - Duration::days(1);
        assert_eq!(result_yesterday, yesterday);
    }

    #[test]
    fn test_date_parsing_invalid_format() {
        let result = parse_date_shorthand("27-10-2023");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));

        let result = parse_date_shorthand("October 27, 2023");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));

        let result = parse_date_shorthand("invalid-date");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));
    }

    #[test]
    fn test_date_parsing_invalid_date() {
        // Valid format, invalid date
        let result = parse_date_shorthand("2023-02-30"); // February 30th doesn't exist
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format")); // Our parser returns this generic message

        let result = parse_date_shorthand("32.10.2023");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));

        let result = parse_date_shorthand("2023/13/01");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));
    }
}

//task-athlete-cli/src/main.rs
//src/main.rs
mod cli; // Keep cli module for parsing args

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc}; // Keep Duration if needed, remove if not
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use csv;
use std::io;
use std::io::{stdin, stdout, Write}; // For prompts

use task_athlete_lib::{
    AppService,
    ConfigError,
    DbError,
    ExerciseDefinition,
    ExerciseStats, // Import PB types, DbError, Stats types
    ExerciseType,
    PBInfo,
    Units,
    VolumeFilters,
    Workout,
    WorkoutFilters,
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
    let mut service =
        AppService::initialize().context("Failed to initialize application service")?;

    // --- Execute Commands using AppService ---
    match cli_args.command {
        cli::Commands::GenerateCompletion { .. } => {
            // This case is handled above, but keep it exhaustive
            unreachable!("Completion generation should have exited already");
        }
        // --- Exercise Definition Commands ---
        cli::Commands::CreateExercise {
            name,
            type_,
            muscles,
        } => {
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
        cli::Commands::EditExercise {
            identifier,
            name,
            type_,
            muscles,
        } => {
            let db_type = type_.map(cli_type_to_db_type);
            let muscles_update = match muscles {
                Some(ref s) if s.trim().is_empty() => Some(None), // Clear
                Some(ref s) => Some(Some(s.trim())),              // Set
                None => None,                                     // Don't change
            };

            match service.edit_exercise(&identifier, name.as_deref(), db_type, muscles_update) {
                // Ok(0) case is now handled by Err(DbError::ExerciseNotFound) from the service layer
                Ok(rows) => {
                    println!(
                        "Successfully updated exercise definition '{}' ({} row(s) affected).",
                        identifier, rows
                    );
                    if name.is_some() {
                        println!("Note: If the name was changed, corresponding workout entries and aliases were also updated.");
                    }
                }
                Err(e) => bail!("Error editing exercise '{}': {}", identifier, e), // Handles unique name & not found errors from service
            }
        }
        cli::Commands::DeleteExercise { identifiers } => {
            match service.delete_exercise(&identifiers) {
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!("Successfully deleted exercise definition '{:?}' ({} row(s) affected). Associated aliases were also deleted.", identifiers, rows),
                Err(e) => bail!("Error deleting exercise: {}", e),
            }
        }

        // --- Workout Entry Commands ---
        cli::Commands::Add {
            exercise,
            date, // Feature 3: Get date from args
            sets,
            reps,
            weight,
            duration,
            distance, // Add distance arg
            notes,
            implicit_type,
            implicit_muscles,
        } => {
            let identifier_trimmed = exercise.trim();
            if identifier_trimmed.is_empty() {
                bail!("Exercise identifier cannot be empty for adding a workout.");
            }

            // Determine if bodyweight might be needed *before* calling add_workout
            let mut bodyweight_to_use: Option<f64> = None;
            let mut needs_bw_check = false;

            // Peek at exercise type using the service resolver
            let exercise_def_peek =
                service.get_exercise_by_identifier_service(identifier_trimmed)?;
            if let Some(ref def) = exercise_def_peek {
                if def.type_ == ExerciseType::BodyWeight {
                    needs_bw_check = true;
                }
            } else if let Some(cli::ExerciseTypeCli::BodyWeight) = implicit_type {
                needs_bw_check = true;
            }

            // If bodyweight exercise, check config and potentially prompt
            if needs_bw_check {
                // Fetch the latest bodyweight from the DB instead of config
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
                    Ok(None) => {
                        // 2. No weight logged. Check if prompting is enabled.
                        if service.config.prompt_for_bodyweight {
                            // 3. Prompting enabled, call the prompt function
                            match prompt_and_log_bodyweight_cli(&mut service) {
                                // Pass mutable service
                                Ok(Some(logged_bw)) => {
                                    // User entered weight, use it
                                    bodyweight_to_use = Some(logged_bw);
                                }
                                Ok(None) => {
                                    // User skipped or disabled prompt, use 0 base weight
                                    bodyweight_to_use = Some(0.0);
                                }
                                Err(e) => bail!("Cannot add bodyweight exercise: {}", e), // Prompt failed
                            }
                        } else {
                            // 4. Prompting is disabled, use 0 base weight
                            println!("Bodyweight prompting disabled. Using 0 base weight for this exercise.");
                            bodyweight_to_use = Some(0.0);
                        }
                    }
                    Err(e) => bail!("Error checking bodyweight configuration: {}", e),
                }
            }

            // Call the service add_workout method
            let db_implicit_type = implicit_type.map(cli_type_to_db_type);
            let units = service.config.units;
            match service.add_workout(
                identifier_trimmed,
                date, // Pass date
                sets,
                reps,
                weight,
                duration,
                distance, // Pass distance
                notes,
                db_implicit_type,
                implicit_muscles,  // Pass implicit creation details
                bodyweight_to_use, // Pass the resolved bodyweight (if applicable)
            ) {
                Ok((id, pb_info_opt)) => {
                    // Feature 4: Get PB info
                    // Use the potentially *canonical* name if implicit creation happened or alias used
                    let final_exercise_name = service
                        .get_exercise_by_identifier_service(identifier_trimmed)?
                        .map(|def| def.name)
                        .unwrap_or_else(|| identifier_trimmed.to_string()); // Fallback if refetch fails (shouldn't happen)
                    println!(
                        "Successfully added workout for '{}' on {} ID: {}",
                        final_exercise_name,
                        date.format("%Y-%m-%d"),
                        id
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
        } => {
            // Add distance, handle date edit
            match service.edit_workout(
                id, exercise, sets, reps, weight, duration, distance, notes, date,
            ) {
                // Pass distance and date to service
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!(
                    "Successfully updated workout ID {} ({} row(s) affected).",
                    id, rows
                ),
                Err(e) => bail!("Error editing workout ID {}: {}", id, e),
            }
        }
        cli::Commands::DeleteWorkout { ids } => {
            match service.delete_workouts(&ids) {
                // Ok(0) case handled by Err(DbError::NotFound) from service
                Ok(rows) => println!(
                    "Successfully deleted workout ID {:?} ({} row(s) affected).",
                    ids,
                    rows.len()
                ),
                Err(e) => bail!("Error deleting workout: {}", e),
                // Err(e) => bail!("Error deleting workout ID {}: {}", id, e),
            }
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
            // Determine date based on flags or explicit date arg
            let effective_date = if today_flag {
                Some(Utc::now().date_naive())
            } else if yesterday_flag {
                Some((Utc::now() - Duration::days(1)).date_naive())
            } else {
                date
            };

            let workouts_result = if let Some(ex_ident) = nth_last_day_exercise {
                let n = nth_last_day_n.context("Missing N value for --nth-last-day")?;
                // Service method now resolves identifier internally
                service.list_workouts_for_exercise_on_nth_last_day(&ex_ident, n)
            } else {
                let db_type_filter = type_.map(cli_type_to_db_type);
                // Limit applies only if no date filter and not using nth_last_day
                let effective_limit = if effective_date.is_none() && nth_last_day_n.is_none() {
                    Some(limit)
                } else {
                    None
                };

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
                        let header_color =
                            task_athlete_lib::parse_color(&service.config.theme.header_color)
                                .map(Color::from)
                                .unwrap_or(Color::Green); // Fallback
                        print_workout_table(workouts, header_color, service.config.units);
                    }
                }
                Err(e) => {
                    // Handle specific case where exercise filter didn't find the exercise
                    if let Some(db_err) = e.downcast_ref::<DbError>() {
                        if let DbError::ExerciseNotFound(ident) = db_err {
                            println!(
                                "Exercise identifier '{}' not found. No workouts listed.",
                                ident
                            );
                            return Ok(()); // Exit gracefully
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
            exercise,
            date,
            type_,
            muscle,
            limit_days,
            start_date,
            end_date,
        } => {
            // Use explicit date if provided, otherwise use range or limit
            let (eff_start_date, eff_end_date) = if let Some(d) = date {
                (Some(d), Some(d)) // Filter for a single specific day
            } else {
                (start_date, end_date) // Use provided range or None
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
                        let header_color =
                            task_athlete_lib::parse_color(&service.config.theme.header_color)
                                .map(Color::from)
                                .unwrap_or(Color::Cyan); // Fallback
                        print_exercise_definition_table(exercises, header_color);
                    }
                }
                Err(e) => bail!("Error listing exercises: {}", e),
            }
        }
        // --- Alias Commands (Feature 1) ---
        cli::Commands::Alias {
            alias_name,
            exercise_identifier,
        } => match service.create_alias(&alias_name, &exercise_identifier) {
            Ok(()) => println!(
                "Successfully created alias '{}' for exercise '{}'.",
                alias_name, exercise_identifier
            ),
            Err(e) => bail!("Error creating alias: {}", e),
        },
        cli::Commands::Unalias { alias_name } => {
            match service.delete_alias(&alias_name) {
                // Ok(0) handled by Err(DbError::NotFound) from service
                Ok(rows) => println!(
                    "Successfully deleted alias '{}' ({} row(s) affected).",
                    alias_name, rows
                ),
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
        cli::Commands::SetUnits { units } => {
            // Feature 3
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
        cli::Commands::LogBodyweight { weight, date } => {
            // Combine date with noon UTC time
            let timestamp = date
                .and_hms_opt(12, 0, 0)
                .map(|naive_dt| naive_dt.and_utc())
                .ok_or_else(|| {
                    anyhow::anyhow!("Internal error creating timestamp from date {}", date)
                })?;

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
        }
        cli::Commands::ListBodyweights { limit } => match service.list_bodyweights(limit) {
            Ok(entries) if entries.is_empty() => {
                println!("No bodyweight entries found.");
            }
            Ok(entries) => {
                if export_csv {
                    print_bodyweight_csv(entries, service.config.units)?;
                } else {
                    print_bodyweight_table(entries, service.config.units);
                }
            }
            Err(e) => bail!("Error listing bodyweights: {}", e),
        },
        cli::Commands::SetTargetWeight { weight } => {
            match service.set_target_bodyweight(Some(weight)) {
                Ok(()) => println!(
                    "Successfully set target bodyweight to {} {:?}. Config updated.",
                    weight, service.config.units
                ),
                Err(e) => bail!("Error setting target bodyweight: {}", e),
            }
        }
        cli::Commands::ClearTargetWeight => match service.set_target_bodyweight(None) {
            Ok(()) => println!("Target bodyweight cleared. Config updated."),
            Err(e) => bail!("Error clearing target bodyweight: {}", e),
        },
        cli::Commands::SetPbNotification { enabled } => {
            // Global enable/disable
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
                Ok(()) => println!(
                    "Set Weight PB notification to: {}. Config updated.",
                    enabled
                ),
                Err(e) => bail!("Error setting weight PB notification: {}", e),
            }
        }
        cli::Commands::SetPbNotifyReps { enabled } => match service.set_pb_notify_reps(enabled) {
            Ok(()) => println!("Set Reps PB notification to: {}. Config updated.", enabled),
            Err(e) => bail!("Error setting reps PB notification: {}", e),
        },
        cli::Commands::SetPbNotifyDuration { enabled } => {
            match service.set_pb_notify_duration(enabled) {
                Ok(()) => println!(
                    "Set Duration PB notification to: {}. Config updated.",
                    enabled
                ),
                Err(e) => bail!("Error setting duration PB notification: {}", e),
            }
        }
        cli::Commands::SetPbNotifyDistance { enabled } => {
            match service.set_pb_notify_distance(enabled) {
                Ok(()) => println!(
                    "Set Distance PB notification to: {}. Config updated.",
                    enabled
                ),
                Err(e) => bail!("Error setting distance PB notification: {}", e),
            }
        }
        cli::Commands::SetStreakInterval { days } => match service.set_streak_interval(days) {
            Ok(()) => println!("Set streak interval to {} day(s). Config updated.", days),
            Err(e) => bail!("Error setting streak interval: {}", e),
        },
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

/// Prompts user for current bodyweight if config allows.
/// Logs it via the service if entered.
/// Returns Ok(Some(weight)) if logged, Ok(None) if cancelled or 'N' entered, Err on failure.
/// Needs mutable service to potentially disable prompt config.
fn prompt_and_log_bodyweight_cli(service: &mut AppService) -> Result<Option<f64>, ConfigError> {
    // Caller should ensure service.config.prompt_for_bodyweight is true before calling

    println!("\nBodyweight is required for this exercise type, but none is logged yet.");
    println!("We can use your latest logged weight if available, or you can enter it now.");
    println!(
        "Please enter your current bodyweight (in {:?}).",
        service.config.units
    );
    print!("Enter weight, 'N' to disable this prompt, or press Enter to skip: "); // Updated prompt
    stdout().flush().map_err(ConfigError::Io)?;

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(ConfigError::Io)?;
    let trimmed_input = input.trim();

    if trimmed_input.is_empty() {
        // User pressed Enter to skip
        println!("Skipping bodyweight entry for this workout. Using 0 base weight.");
        Ok(None) // Signal to use 0 base weight
    } else if trimmed_input.eq_ignore_ascii_case("n") {
        println!("Okay, disabling future bodyweight prompts for 'add' command.");
        println!(
            "Using 0 base weight for this workout. Use 'log-bodyweight' to add entries later."
        );
        // Update config via service method to handle saving
        service.disable_bodyweight_prompt()?; // This saves the config
        Ok(None) // Signal to use 0 base weight
    } else {
        // Try parsing as weight
        match trimmed_input.parse::<f64>() {
            Ok(weight) if weight > 0.0 => {
                println!(
                    "Logging bodyweight: {:.2} {:?}",
                    weight, service.config.units
                );
                // Log the bodyweight using the service
                match service.add_bodyweight_entry(Utc::now(), weight) {
                    Ok(_) => Ok(Some(weight)), // Return the successfully logged weight
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

/// Handles PB notification logic, including prompting if config not set (Feature 4)
/// Needs mutable service to potentially update config via prompt.
fn handle_pb_notification(service: &mut AppService, pb_info: &PBInfo, units: Units) -> Result<()> {
    // Check if *any* relevant PB was achieved before checking global enabled status
    let relevant_pb_achieved = (pb_info.achieved_weight_pb && service.config.notify_pb_weight)
        || (pb_info.achieved_reps_pb && service.config.notify_pb_reps)
        || (pb_info.achieved_duration_pb && service.config.notify_pb_duration)
        || (pb_info.achieved_distance_pb && service.config.notify_pb_distance);

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
        let weight_unit_str = match units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        messages.push(format!(
            "New Max Weight: {:.2} {} {}",
            pb_info.new_weight.unwrap_or(0.0),
            weight_unit_str,
            pb_info
                .previous_weight
                .map_or("".to_string(), |p| format!("(Previous: {:.2})", p))
        ));
    }
    if pb_info.achieved_reps_pb && config.notify_pb_reps {
        messages.push(format!(
            "New Max Reps: {} {}",
            pb_info.new_reps.unwrap_or(0),
            pb_info
                .previous_reps
                .map_or("".to_string(), |p| format!("(Previous: {})", p))
        ));
    }
    if pb_info.achieved_duration_pb && config.notify_pb_duration {
        messages.push(format!(
            "New Max Duration: {} min {}",
            pb_info.new_duration.unwrap_or(0),
            pb_info
                .previous_duration
                .map_or("".to_string(), |p| format!("(Previous: {} min)", p))
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
            }
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

/// Prints logged bodyweights in a table
fn print_bodyweight_table(entries: Vec<(DateTime<Utc>, f64)>, units: Units) {
    let mut table = Table::new();
    let header_color = task_athlete_lib::parse_color("Blue")
        .map(Color::from)
        .unwrap_or(Color::Blue);
    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Timestamp (UTC)").fg(header_color),
            Cell::new(format!("Weight ({})", weight_unit_str)).fg(header_color),
        ]);

    for (timestamp, weight) in entries {
        table.add_row(vec![
            Cell::new(timestamp.format("%Y-%m-%d %H:%M").to_string()),
            Cell::new(format!("{:.2}", weight)),
        ]);
    }
    println!("{table}");
}

fn print_bodyweight_csv(entries: Vec<(DateTime<Utc>, f64)>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    // Write header
    writer.write_record(&["Timestamp_UTC", &format!("Weight_{}", weight_unit_str)])?;

    for (timestamp, weight) in entries {
        writer.write_record(&[timestamp.to_rfc3339(), format!("{:.2}", weight)])?;
    }
    writer.flush()?;
    Ok(())
}

/// Prints workout entries in a formatted table.
fn print_workout_table(workouts: Vec<Workout>, header_color: Color, units: Units) {
    let mut table = Table::new();
    let weight_unit_str = match units {
        Units::Metric => "(kg)",
        Units::Imperial => "(lbs)",
    };
    let distance_unit_str = match units {
        Units::Metric => "(km)",
        Units::Imperial => "(miles)",
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
            Cell::new(workout.exercise_name),                                  // Canonical name
            Cell::new(
                workout
                    .exercise_type
                    .map_or("-".to_string(), |t| t.to_string()),
            ),
            Cell::new(workout.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(workout.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(
                workout
                    .weight
                    .map_or("-".to_string(), |v| format!("{:.2}", v)),
            ),
            Cell::new(
                workout
                    .duration_minutes
                    .map_or("-".to_string(), |v| v.to_string()),
            ),
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
        table.add_row(vec![Cell::new(alias), Cell::new(canonical_name)]);
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
    for (date, exercise_name, volume) in volume_data {
        // Destructure tuple
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
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic); // No headers needed for key-value

    table.add_row(vec![
        Cell::new("Total Workouts").add_attribute(Attribute::Bold),
        Cell::new(stats.total_workouts),
    ]);
    table.add_row(vec![
        Cell::new("First Workout").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .first_workout_date
                .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Last Workout").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .last_workout_date
                .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Avg Workouts / Week").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .avg_workouts_per_week
                .map_or("N/A".to_string(), |avg| format!("{:.2}", avg)),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Longest Gap").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .longest_gap_days
                .map_or("N/A".to_string(), |gap| format!("{} days", gap)),
        ),
    ]);

    let streak_interval_str = match stats.streak_interval_days {
        1 => "(Daily)".to_string(),
        n => format!("({}-day Interval)", n),
    };
    table.add_row(vec![
        Cell::new(format!("Current Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(if stats.current_streak > 0 {
            stats.current_streak.to_string()
        } else {
            "0".to_string()
        }),
    ]);
    table.add_row(vec![
        Cell::new(format!("Longest Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(if stats.longest_streak > 0 {
            stats.longest_streak.to_string()
        } else {
            "0".to_string()
        }),
    ]);

    println!("{}", table);

    // Personal Bests Section
    println!("\n--- Personal Bests ---");
    let mut pb_table = Table::new();
    pb_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let distance_unit_str = match units {
        Units::Metric => "km",
        Units::Imperial => "miles",
    };

    let mut has_pbs = false;
    if let Some(pb_weight) = stats.personal_bests.max_weight {
        pb_table.add_row(vec![
            Cell::new("Max Weight").add_attribute(Attribute::Bold),
            Cell::new(format!("{:.2} {}", pb_weight, weight_unit_str)),
        ]);
        has_pbs = true;
    }
    if let Some(pb_reps) = stats.personal_bests.max_reps {
        pb_table.add_row(vec![
            Cell::new("Max Reps").add_attribute(Attribute::Bold),
            Cell::new(pb_reps),
        ]);
        has_pbs = true;
    }
    if let Some(pb_duration) = stats.personal_bests.max_duration_minutes {
        pb_table.add_row(vec![
            Cell::new("Max Duration").add_attribute(Attribute::Bold),
            Cell::new(format!("{} min", pb_duration)),
        ]);
        has_pbs = true;
    }
    if let Some(pb_distance_km) = stats.personal_bests.max_distance_km {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_distance_km, distance_unit_str),
            Units::Imperial => (pb_distance_km * KM_TO_MILES, distance_unit_str),
        };
        pb_table.add_row(vec![
            Cell::new("Max Distance").add_attribute(Attribute::Bold),
            Cell::new(format!("{:.2} {}", dist_val, dist_unit)),
        ]);
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
    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let distance_unit_str = match units {
        Units::Metric => "km",
        Units::Imperial => "miles",
    };

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
            workout
                .exercise_type
                .map_or("".to_string(), |t| t.to_string()),
            workout.sets.map_or("".to_string(), |v| v.to_string()),
            workout.reps.map_or("".to_string(), |v| v.to_string()),
            workout
                .weight
                .map_or("".to_string(), |v| format!("{:.2}", v)),
            workout
                .duration_minutes
                .map_or("".to_string(), |v| v.to_string()),
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
    writer.write_record(&["Alias", "Canonical_Exercise_Name"])?;

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

    for (date, exercise_name, volume) in volume_data {
        // Destructure tuple
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
    writer.write_record(&[
        "First_Workout",
        &stats
            .first_workout_date
            .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
    ])?;
    writer.write_record(&[
        "Last_Workout",
        &stats
            .last_workout_date
            .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
    ])?;
    writer.write_record(&[
        "Avg_Workouts_Per_Week",
        &stats
            .avg_workouts_per_week
            .map_or("N/A".to_string(), |avg| format!("{:.2}", avg)),
    ])?;
    writer.write_record(&[
        "Longest_Gap_Days",
        &stats
            .longest_gap_days
            .map_or("N/A".to_string(), |gap| gap.to_string()),
    ])?;
    writer.write_record(&[
        "Streak_Interval_Days",
        &stats.streak_interval_days.to_string(),
    ])?;
    writer.write_record(&["Current_Streak", &stats.current_streak.to_string()])?;
    writer.write_record(&["Longest_Streak", &stats.longest_streak.to_string()])?;

    // Write Personal Bests
    let weight_unit_str = match units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let distance_unit_str = match units {
        Units::Metric => "km",
        Units::Imperial => "miles",
    };

    if let Some(pb_weight) = stats.personal_bests.max_weight {
        writer.write_record(&[
            &format!("PB_Max_Weight_{}", weight_unit_str),
            &format!("{:.2}", pb_weight),
        ])?;
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
        writer.write_record(&[
            &format!("PB_Max_Distance_{}", dist_unit),
            &format!("{:.2}", dist_val),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

fn print_exercise_definition_csv(exercises: Vec<ExerciseDefinition>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    // Write header
    writer.write_record(&["ID", "Name", "Type", "Muscles"])?;

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

//task-athlete-lib/src/config.rs
//src/config.rs
use anyhow::Result;
use comfy_table::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{stdin, Write};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use thiserror::Error;

const CONFIG_FILE_NAME: &str = "config.toml";
const APP_CONFIG_DIR: &str = "workout-tracker-cli";
const CONFIG_ENV_VAR: &str = "WORKOUT_CONFIG_DIR"; // Environment variable name

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not determine configuration directory.")]
    CannotDetermineConfigDir,
    #[error("I/O error accessing config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config file (TOML): {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Failed to serialize config data (TOML): {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Invalid color name: {0}")]
    InvalidColor(String),
    #[error("Bodyweight not set in config. Use 'set-bodyweight <weight>' or update {0:?}.")]
    BodyweightNotSet(PathBuf),
    #[error("Bodyweight input cancelled by user.")] // Keep for potential interactive use
    BodyweightPromptCancelled,
    #[error("Invalid bodyweight input: {0}")]
    InvalidBodyweightInput(String),
    #[error("Personal best notification setting not configured. Please enable/disable using 'set-pb-notification true|false'.")]
    // Feature 4
    PbNotificationNotSet,
    #[error("Personal best notification prompt cancelled by user.")] // Feature 4
    PbNotificationPromptCancelled,
    #[error("Invalid input for PB notification prompt: {0}")] // Feature 4
    InvalidPbNotificationInput(String),
}

// Note: PbMetricScope removed as specific booleans are used now.
//       Kept the enum definition commented out in case of future refactoring.
// #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
// #[serde(rename_all = "lowercase")]
// pub enum PbMetricScope {
//     All,     // Check weight and reps
//     Weight,  // Only check weight PBs
//     Reps,    // Only check reps PBs
//     // Note: Disabling notifications entirely is handled by notify_on_pb = false
// }
// impl Default for PbMetricScope { fn default() -> Self { PbMetricScope::All } }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Metric,   // e.g., kg, km
    Imperial, // e.g., lbs, miles
}

// Implement Default for Units
impl Default for Units {
    fn default() -> Self {
        Units::Metric // Default to Metric
    }
}

// Define standard colors using strum for easy iteration/parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum StandardColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    DarkGrey,
    DarkRed,
    DarkGreen,
    DarkYellow,
    DarkBlue,
    DarkMagenta,
    DarkCyan,
    Grey,
}

// Helper to convert our enum to comfy_table::Color
impl From<StandardColor> for Color {
    fn from(value: StandardColor) -> Self {
        match value {
            StandardColor::Black => Color::Black,
            StandardColor::Red => Color::Red,
            StandardColor::Green => Color::Green,
            StandardColor::Yellow => Color::Yellow,
            StandardColor::Blue => Color::Blue,
            StandardColor::Magenta => Color::Magenta,
            StandardColor::Cyan => Color::Cyan,
            StandardColor::White => Color::White,
            StandardColor::DarkGrey => Color::DarkGrey,
            StandardColor::DarkRed => Color::DarkRed,
            StandardColor::DarkGreen => Color::DarkGreen,
            StandardColor::DarkYellow => Color::DarkYellow,
            StandardColor::DarkBlue => Color::DarkBlue,
            StandardColor::DarkMagenta => Color::DarkMagenta,
            StandardColor::DarkCyan => Color::DarkCyan,
            StandardColor::Grey => Color::Grey,
        }
    }
}

// Helper to parse a string into our StandardColor enum
pub fn parse_color(color_str: &str) -> Result<StandardColor, ConfigError> {
    for color in StandardColor::iter() {
        if format!("{:?}", color).eq_ignore_ascii_case(color_str) {
            return Ok(color);
        }
    }
    Err(ConfigError::InvalidColor(color_str.to_string()))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct ThemeConfig {
    pub header_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            header_color: "Green".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Removed Default derive
#[serde(default)] // Ensure defaults are used if fields are missing
pub struct Config {
    pub bodyweight: Option<f64>,
    pub units: Units,
    pub prompt_for_bodyweight: bool, // Default is true
    pub streak_interval_days: u32,   // Default 1

    // PB Notification Settings
    pub notify_pb_enabled: Option<bool>, // None = prompt first time, Some(true/false) = user setting
    pub notify_pb_weight: bool,
    pub notify_pb_reps: bool,
    pub notify_pb_duration: bool,
    pub notify_pb_distance: bool,
    pub target_bodyweight: Option<f64>,

    // Theming
    pub theme: ThemeConfig,
}

// Implement Default for Config manually to set defaults correctly
impl Default for Config {
    fn default() -> Self {
        Config {
            bodyweight: None,
            units: Units::default(),
            prompt_for_bodyweight: true, // Explicitly true by default
            streak_interval_days: 1,     // Default to daily streaks
            notify_pb_enabled: None,     // Default to None, so user is prompted first time
            notify_pb_weight: true,      // Default to true
            notify_pb_reps: true,        // Default to true
            notify_pb_duration: true,    // Default to true
            notify_pb_distance: true,    // Default to true
            target_bodyweight: None,
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    // Helper to create a new instance with defaults
    fn new_default() -> Self {
        Self::default()
    }
}

/// Determines the path to the configuration file.
/// Exposed at crate root as get_config_path_util
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let config_dir_override = std::env::var(CONFIG_ENV_VAR).ok();

    let config_dir_path = match config_dir_override {
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path.is_dir() {
                eprintln!( // Keep warning, as it's about env var setup
                    "Warning: Environment variable {} points to '{}', which is not a directory. Trying to create it.",
                    CONFIG_ENV_VAR,
                    path.display()
                 );
                fs::create_dir_all(&path)?;
            }
            path
        }
        None => {
            let base_config_dir =
                dirs::config_dir().ok_or(ConfigError::CannotDetermineConfigDir)?;
            base_config_dir.join(APP_CONFIG_DIR)
        }
    };

    if !config_dir_path.exists() {
        fs::create_dir_all(&config_dir_path)?;
    }

    Ok(config_dir_path.join(CONFIG_FILE_NAME))
}

/// Loads the configuration from the TOML file at the given path.
/// Exposed at crate root as load_config_util
pub fn load_config(config_path: &Path) -> Result<Config, ConfigError> {
    if !config_path.exists() {
        // Don't print here, let caller decide how to inform user
        let default_config = Config::new_default();
        save_config(&config_path, &default_config)?;
        Ok(default_config)
    } else {
        let config_content = fs::read_to_string(&config_path)?;
        // Use serde(default) to handle missing fields when parsing
        let config: Config = toml::from_str(&config_content).map_err(ConfigError::TomlParse)?;
        // No need to manually fill defaults here if using #[serde(default)] on struct and fields
        Ok(config)
    }
}

/// Saves the configuration to the TOML file.
/// Exposed at crate root as save_config_util
pub fn save_config(config_path: &Path, config: &Config) -> Result<(), ConfigError> {
    if let Some(parent_dir) = config_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    }
    let config_content = toml::to_string_pretty(config).map_err(ConfigError::TomlSerialize)?;
    fs::write(config_path, config_content)?;
    Ok(())
}

//task-athlete-lib/src/db.rs
//src/db.rs
use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension, Row, ToSql}; // Import named_params
use std::collections::HashMap; // For listing aliases
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

#[derive(Default, Debug)]
pub struct VolumeFilters<'a> {
    pub exercise_name: Option<&'a str>, // Canonical name expected
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit_days: Option<u32>, // Limit number of distinct days returned
}

pub fn calculate_daily_volume_filtered(
    conn: &Connection,
    filters: VolumeFilters,
) -> Result<Vec<(NaiveDate, String, f64)>, DbError> {
    // Base query calculates volume per workout *entry*
    let mut sql = "
        SELECT
            date(w.timestamp) as workout_date,
            w.exercise_name, -- Select the exercise name
            SUM(CASE
                    WHEN e.type IN ('resistance', 'body-weight')
                    THEN COALESCE(w.sets, 1) * COALESCE(w.reps, 0) * COALESCE(w.weight, 0)
                    ELSE 0 -- Define volume as 0 for Cardio or other types
                END) as daily_volume
        FROM workouts w
        LEFT JOIN exercises e ON w.exercise_name = e.name
        WHERE 1=1"
        .to_string();

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(name) = filters.exercise_name {
        sql.push_str(" AND w.exercise_name = :ex_name COLLATE NOCASE");
        params_map.insert(":ex_name".into(), Box::new(name.to_string()));
    }
    if let Some(start) = filters.start_date {
        sql.push_str(" AND date(w.timestamp) >= date(:start_date)");
        params_map.insert(
            ":start_date".into(),
            Box::new(start.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(end) = filters.end_date {
        sql.push_str(" AND date(w.timestamp) <= date(:end_date)");
        params_map.insert(
            ":end_date".into(),
            Box::new(end.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(ex_type) = filters.exercise_type {
        sql.push_str(" AND e.type = :ex_type");
        params_map.insert(":ex_type".into(), Box::new(ex_type.to_string()));
    }
    if let Some(m) = filters.muscle {
        sql.push_str(" AND e.muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }

    // Group by date AND exercise name to sum volume correctly per exercise per day
    sql.push_str(
        " GROUP BY workout_date, w.exercise_name ORDER BY workout_date DESC, w.exercise_name ASC",
    );

    // Limit the *number of rows* returned (each row is one exercise on one day)
    // Note: This isn't limiting distinct days directly if one day has multiple exercises.
    // Limiting distinct days would require a subquery, which adds complexity.
    // Let's keep limiting rows for simplicity for now.
    if filters.start_date.is_none() && filters.end_date.is_none() {
        // Apply limit only if no date range specified
        if let Some(limit) = filters.limit_days {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let volume_iter = stmt
        .query_map(params_for_query.as_slice(), |row| {
            let date_str: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let exercise_name: String = row.get(1)?; // Get exercise name
            let volume: f64 = row.get(2)?; // Volume is now the 3rd column (index 2)
            Ok((date, exercise_name, volume))
        })
        .map_err(DbError::QueryFailed)?;

    volume_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

#[derive(Debug)]
pub struct Workout {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String, // Always the canonical name
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub distance: Option<f64>, // Added distance
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
    #[error("Alias not found: {0}")] // Feature 1
    AliasNotFound(String),
    #[error("Alias already exists: {0}")] // Feature 1
    AliasAlreadyExists(String),
    #[error("Exercise name must be unique (case-insensitive): '{0}' already exists.")] // Feature 2
    ExerciseNameNotUnique(String),
    #[error("No workout data found for exercise '{0}'")]
    NoWorkoutDataFound(String),
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
    let conn = Connection::open(path).map_err(DbError::Connection)?;
    // Enable foreign key support if needed later, though not strictly required for aliases
    // conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

/// Initializes the database tables if they don't exist.
pub fn init_db(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE, -- Feature 2: Ensure UNIQUE and case-insensitive
            type TEXT NOT NULL CHECK(type IN ('resistance', 'cardio', 'body-weight')),
            muscles TEXT
        )",
        [],
    ).map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS workouts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL, -- Store as RFC3339 string
            exercise_name TEXT NOT NULL COLLATE NOCASE, -- Store canonical name, case-insensitive for joins
            sets INTEGER,
            reps INTEGER,
            weight REAL,
            duration_minutes INTEGER,
            distance REAL, -- Added distance column
            notes TEXT
            -- Optionally add FOREIGN KEY(exercise_name) REFERENCES exercises(name) ON UPDATE CASCADE ON DELETE SET NULL ?
            -- Requires careful handling of deletion/renaming if implemented. Keeping it simple for now.
        )",
        [],
    ).map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS aliases ( -- Feature 1
            alias_name TEXT PRIMARY KEY NOT NULL COLLATE NOCASE, -- Alias is unique, case-insensitive
            exercise_name TEXT NOT NULL COLLATE NOCASE -- Canonical exercise name it refers to
            -- Optionally add FOREIGN KEY(exercise_name) REFERENCES exercises(name) ON UPDATE CASCADE ON DELETE CASCADE ?
            -- This would auto-update/delete aliases if the exercise name changes or is deleted.
            -- Requires robust transaction handling in exercise edit/delete. Let's manage manually for now.
        )",
        [],).map_err(DbError::Connection)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bodyweights (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL UNIQUE, -- Store as RFC3339 string, unique timestamp
            weight REAL NOT NULL -- Store weight (assumed in config units, but DB doesn't enforce)
        )",
        [],
    )
    .map_err(DbError::Connection)?;

    // Add indexes for common lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workouts_timestamp ON workouts(timestamp)",
        [],
    )
    .map_err(DbError::Connection)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_workouts_exercise_name ON workouts(exercise_name)",
        [],
    )
    .map_err(DbError::Connection)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_aliases_exercise_name ON aliases(exercise_name)",
        [],
    )
    .map_err(DbError::Connection)?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_bodyweights_timestamp ON bodyweights(timestamp)",
        [],
    )
    .map_err(DbError::Connection)?;

    // Add distance column if it doesn't exist (for upgrading existing databases)
    add_distance_column_if_not_exists(conn)?;

    Ok(())
}

/// Adds the distance column to the workouts table if it doesn't exist.
/// This is useful for users upgrading from a previous version.
fn add_distance_column_if_not_exists(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(workouts)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut distance_exists = false;
    for column_result in columns {
        if let Ok(column_name) = column_result {
            if column_name == "distance" {
                distance_exists = true;
                break;
            }
        }
    }

    if !distance_exists {
        println!("Adding 'distance' column to workouts table..."); // Inform user
        conn.execute("ALTER TABLE workouts ADD COLUMN distance REAL", [])?;
    }

    Ok(())
}

/// Adds a new workout entry to the database.
pub fn add_workout(
    conn: &Connection,
    exercise_name: &str,      // Should be the canonical name
    timestamp: DateTime<Utc>, // Feature 3: Accept specific timestamp
    sets: Option<i64>,
    reps: Option<i64>,
    weight: Option<f64>,
    duration: Option<i64>,
    distance: Option<f64>,
    notes: Option<String>,
) -> Result<i64, DbError> {
    let timestamp_str = timestamp.to_rfc3339();
    // Use default value 1 for sets only if it's None and the exercise type needs it (e.g., resistance, bodyweight)
    // For simplicity, let's keep the original behavior where sets default to 1 if None.
    // A more robust approach might check exercise type.
    let sets_val = sets.unwrap_or(1);

    conn.execute(
        "INSERT INTO workouts (timestamp, exercise_name, sets, reps, weight, duration_minutes, distance, notes)
         VALUES (:ts, :ex_name, :sets, :reps, :weight, :duration, :distance, :notes)",
        named_params! {
            ":ts": timestamp_str,
            ":ex_name": exercise_name,
            ":sets": sets_val,
            ":reps": reps,
            ":weight": weight,
            ":duration": duration,
            ":distance": distance, // Add distance
            ":notes": notes,
        },
    ).map_err(DbError::InsertFailed)?;
    Ok(conn.last_insert_rowid())
}

/// Updates an existing workout entry in the database by its ID.
pub fn update_workout(
    conn: &Connection,
    id: i64,
    new_exercise_name: Option<&str>,
    new_sets: Option<i64>,
    new_reps: Option<i64>,
    new_weight: Option<f64>,
    new_duration: Option<i64>,
    new_distance: Option<f64>,
    new_notes: Option<&str>,
    new_timestamp: Option<DateTime<Utc>>, // Feature 3: Allow editing timestamp
) -> Result<u64, DbError> {
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(ex) = new_exercise_name {
        updates.push("exercise_name = :ex_name");
        params_map.insert(":ex_name".into(), Box::new(ex.to_string()));
    }
    if let Some(s) = new_sets {
        updates.push("sets = :sets");
        params_map.insert(":sets".into(), Box::new(s));
    }
    if let Some(r) = new_reps {
        updates.push("reps = :reps");
        params_map.insert(":reps".into(), Box::new(r));
    }
    // Use is_some() to allow setting weight/duration/distance to NULL explicitly if needed, though CLI usually wouldn't do this.
    if new_weight.is_some() {
        updates.push("weight = :weight");
        params_map.insert(":weight".into(), Box::new(new_weight));
    }
    if new_duration.is_some() {
        updates.push("duration_minutes = :duration");
        params_map.insert(":duration".into(), Box::new(new_duration));
    }
    if new_distance.is_some() {
        updates.push("distance = :distance");
        params_map.insert(":distance".into(), Box::new(new_distance));
    } // Add distance
    if new_notes.is_some() {
        updates.push("notes = :notes");
        params_map.insert(":notes".into(), Box::new(new_notes));
    }
    if let Some(ts) = new_timestamp {
        updates.push("timestamp = :ts");
        params_map.insert(":ts".into(), Box::new(ts.to_rfc3339()));
    }

    let sql = format!("UPDATE workouts SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));

    // Convert HashMap<String, Box<dyn ToSql>> to Vec<(&str, &dyn ToSql)> for execute_named
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = conn
        .execute(&sql, params_for_exec.as_slice())
        .map_err(DbError::UpdateFailed)?;

    if rows_affected == 0 {
        Err(DbError::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes a workout entry from the database by its ID.
pub fn delete_workout(conn: &Connection, id: i64) -> Result<u64, DbError> {
    // Return DbError
    let rows_affected = conn
        .execute("DELETE FROM workouts WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 {
        Err(DbError::WorkoutNotFound(id))
    } else {
        Ok(rows_affected as u64)
    }
}

// Helper function to map a database row to a Workout struct
fn map_row_to_workout(row: &Row) -> Result<Workout, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let timestamp_str: String = row.get(1)?;
    let exercise_name: String = row.get(2)?; // Canonical name from DB
    let sets: Option<i64> = row.get(3)?;
    let reps: Option<i64> = row.get(4)?;
    let weight: Option<f64> = row.get(5)?;
    let duration_minutes: Option<i64> = row.get(6)?;
    let distance: Option<f64> = row.get(7)?; // Added distance
    let notes: Option<String> = row.get(8)?;
    let type_str_opt: Option<String> = row.get(9)?; // From JOIN with exercises

    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let exercise_type = match type_str_opt {
        Some(type_str) => match ExerciseType::try_from(type_str.as_str()) {
            Ok(et) => Some(et),
            Err(_) => None, // Silently ignore invalid type from DB in lib layer
        },
        None => None,
    };

    Ok(Workout {
        id,
        timestamp,
        exercise_name,
        sets,
        reps,
        weight,
        duration_minutes,
        distance,
        notes,
        exercise_type,
    })
}

#[derive(Default, Debug)]
pub struct WorkoutFilters<'a> {
    pub exercise_name: Option<&'a str>, // Canonical name expected
    pub date: Option<NaiveDate>,
    pub exercise_type: Option<ExerciseType>,
    pub muscle: Option<&'a str>,
    pub limit: Option<u32>,
}

/// Lists workout entries from the database based on various filters.
pub fn list_workouts_filtered(
    conn: &Connection,
    filters: WorkoutFilters,
) -> Result<Vec<Workout>, DbError> {
    // Return DbError
    // Note: Column indices change due to adding `distance`
    let mut sql = "SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.notes, e.type
                   FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(name) = filters.exercise_name {
        sql.push_str(" AND w.exercise_name = :ex_name COLLATE NOCASE");
        params_map.insert(":ex_name".into(), Box::new(name.to_string()));
    }
    if let Some(date) = filters.date {
        sql.push_str(" AND date(w.timestamp) = date(:date)");
        params_map.insert(
            ":date".into(),
            Box::new(date.format("%Y-%m-%d").to_string()),
        );
    }
    if let Some(ex_type) = filters.exercise_type {
        sql.push_str(" AND e.type = :ex_type");
        params_map.insert(":ex_type".into(), Box::new(ex_type.to_string()));
    }
    if let Some(m) = filters.muscle {
        sql.push_str(" AND e.muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }

    // Order by timestamp: ASC if date filter is used (show earliest first for that day), DESC otherwise (show latest overall)
    if filters.date.is_some() {
        sql.push_str(" ORDER BY w.timestamp ASC");
    } else {
        sql.push_str(" ORDER BY w.timestamp DESC");
    }

    // Apply limit only if date is not specified (limit applies to overall latest, not within a date)
    if filters.date.is_none() {
        if let Some(limit) = filters.limit {
            sql.push_str(" LIMIT :limit");
            params_map.insert(":limit".into(), Box::new(limit));
        }
    }

    // Convert HashMap to slice for query_map_named
    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_workout)
        .map_err(DbError::QueryFailed)?;

    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed) // Collect results
}

/// Lists workouts for a specific exercise (canonical name) performed on the Nth most recent day it was done.
pub fn list_workouts_for_exercise_on_nth_last_day(
    conn: &Connection,
    exercise_name: &str, // Canonical name expected
    n: u32,
) -> Result<Vec<Workout>, DbError> {
    // Return DbError
    if n == 0 {
        return Err(DbError::QueryFailed(
            rusqlite::Error::InvalidParameterCount(n as usize, 2),
        ));
    } // Indicate bad N via error
    let offset = n - 1;
    // Note: Column indices change due to adding `distance`
    let sql = "WITH RankedDays AS (SELECT DISTINCT date(timestamp) as workout_date FROM workouts WHERE exercise_name = :ex_name COLLATE NOCASE ORDER BY workout_date DESC LIMIT 1 OFFSET :offset)
                SELECT w.id, w.timestamp, w.exercise_name, w.sets, w.reps, w.weight, w.duration_minutes, w.distance, w.notes, e.type
                FROM workouts w LEFT JOIN exercises e ON w.exercise_name = e.name JOIN RankedDays rd ON date(w.timestamp) = rd.workout_date
                WHERE w.exercise_name = :ex_name COLLATE NOCASE ORDER BY w.timestamp ASC;";

    let mut stmt = conn.prepare(sql).map_err(DbError::QueryFailed)?;
    let workout_iter = stmt
        .query_map(
            named_params! { ":ex_name": exercise_name, ":offset": offset },
            map_row_to_workout,
        )
        .map_err(DbError::QueryFailed)?;

    workout_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

// ---- Exercise Definition Functions ----

/// Creates a new exercise definition. Returns ID. Handles UNIQUE constraint.
pub fn create_exercise(
    conn: &Connection,
    name: &str,
    type_: &ExerciseType,
    muscles: Option<&str>,
) -> Result<i64, DbError> {
    // Return DbError
    let type_str = type_.to_string();
    match conn.execute(
        "INSERT INTO exercises (name, type, muscles) VALUES (?1, ?2, ?3)",
        params![name, type_str, muscles],
    ) {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                // Check for UNIQUE constraint violation specifically on 'exercises.name'
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    // It's highly likely the name constraint. Return specific error.
                    return Err(DbError::ExerciseNameNotUnique(name.to_string()));
                }
            }
            Err(DbError::InsertFailed(e)) // Wrap other errors
        }
    }
}

/// Updates an existing exercise definition (found by ID or name). Requires mutable conn for transaction.
/// Also handles updating associated aliases and workout entries if the name changes.
pub fn update_exercise(
    conn: &mut Connection,          // Use mutable connection for transaction
    canonical_name_to_update: &str, // Use the resolved canonical name
    new_name: Option<&str>,
    new_type: Option<&ExerciseType>,
    new_muscles: Option<Option<&str>>,
) -> Result<u64, DbError> {
    // Find exercise by canonical name first to get ID and confirm existence
    let exercise = get_exercise_by_name(conn, canonical_name_to_update)?
        .ok_or_else(|| DbError::ExerciseNotFound(canonical_name_to_update.to_string()))?;
    let id = exercise.id;
    let original_name = exercise.name.clone(); // Clone needed for later comparison/updates

    let name_being_changed = new_name.is_some() && new_name != Some(original_name.as_str());
    let target_new_name = new_name.unwrap_or(&original_name);

    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();
    let mut updates = Vec::new();

    if let Some(name) = new_name {
        updates.push("name = :name");
        params_map.insert(":name".into(), Box::new(name.to_string()));
    }
    if let Some(t) = new_type {
        updates.push("type = :type");
        params_map.insert(":type".into(), Box::new(t.to_string()));
    }
    if let Some(m_opt) = new_muscles {
        updates.push("muscles = :muscles");
        params_map.insert(":muscles".into(), Box::new(m_opt));
    }

    if updates.is_empty() {
        return Ok(0);
    } // No fields to update

    // Use a transaction
    let tx = conn.transaction().map_err(DbError::Connection)?;

    // 1. Update exercises table
    let sql_update_exercise = format!("UPDATE exercises SET {} WHERE id = :id", updates.join(", "));
    params_map.insert(":id".into(), Box::new(id));
    let params_for_exec: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let rows_affected = match tx.execute(&sql_update_exercise, params_for_exec.as_slice()) {
        Ok(rows) => rows,
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                // Check for UNIQUE constraint violation specifically on 'exercises.name'
                if err.code == rusqlite::ErrorCode::ConstraintViolation && name_being_changed {
                    // Name change failed due to unique constraint
                    return Err(DbError::ExerciseNameNotUnique(target_new_name.to_string()));
                }
            }
            return Err(DbError::UpdateFailed(e)); // Other update error
        }
    };

    // 2. Update related tables if name changed
    if name_being_changed {
        // Update workouts table
        tx.execute("UPDATE workouts SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE", // Ensure case-insensitive match on old name
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(DbError::UpdateFailed)?;

        // Update aliases table (Feature 1)
        tx.execute("UPDATE aliases SET exercise_name = :new_name WHERE exercise_name = :old_name COLLATE NOCASE", // Ensure case-insensitive match on old name
                   named_params! { ":new_name": target_new_name, ":old_name": original_name })
          .map_err(DbError::UpdateFailed)?;
    }

    tx.commit().map_err(DbError::Connection)?; // Commit transaction

    if rows_affected == 0 {
        // This case should ideally not happen if get_exercise_by_name succeeded, but handle defensively
        Err(DbError::ExerciseNotFound(original_name))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Deletes an exercise definition (found by canonical name).
/// Also deletes associated aliases.
/// Note: Warning about associated workouts is now handled in the AppService layer.
pub fn delete_exercise(conn: &mut Connection, canonical_name: &str) -> Result<u64, DbError> {
    // Find exercise by canonical name first to get ID and confirm existence
    let exercise = get_exercise_by_name(conn, canonical_name)?
        .ok_or_else(|| DbError::ExerciseNotFound(canonical_name.to_string()))?;
    let id = exercise.id;
    let name_to_delete = exercise.name.clone(); // Use the exact name from DB

    // Use a transaction (optional but safer if we add foreign keys later)
    let tx = conn.transaction().map_err(DbError::Connection)?;

    // 1. Delete associated aliases (Feature 1)
    tx.execute(
        "DELETE FROM aliases WHERE exercise_name = ? COLLATE NOCASE",
        params![name_to_delete],
    ) // Ensure case-insensitive match
    .map_err(DbError::DeleteFailed)?;

    // 2. Delete the exercise definition
    let rows_affected = tx
        .execute("DELETE FROM exercises WHERE id = ?", params![id])
        .map_err(DbError::DeleteFailed)?;

    tx.commit().map_err(DbError::Connection)?;

    if rows_affected == 0 {
        // Should not happen if get_exercise_by_name succeeded
        Err(DbError::ExerciseNotFound(name_to_delete))
    } else {
        Ok(rows_affected as u64)
    }
}

fn map_row_to_exercise_definition(row: &Row) -> Result<ExerciseDefinition, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let type_str: String = row.get(2)?;
    let muscles: Option<String> = row.get(3)?;
    let ex_type = ExerciseType::try_from(type_str.as_str()).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()),
        )
    })?;
    Ok(ExerciseDefinition {
        id,
        name,
        type_: ex_type,
        muscles,
    })
}

/// Retrieves an exercise definition by its name (case-insensitive).
pub fn get_exercise_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE name = ?1 COLLATE NOCASE")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![name], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Retrieves an exercise definition by its ID.
pub fn get_exercise_by_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut stmt = conn
        .prepare("SELECT id, name, type, muscles FROM exercises WHERE id = ?1")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![id], map_row_to_exercise_definition)
        .optional()
        .map_err(DbError::QueryFailed)
}

// --- Alias Functions (Feature 1) ---

/// Creates a new alias for a given canonical exercise name.
pub fn create_alias(
    conn: &Connection,
    alias_name: &str,
    canonical_exercise_name: &str,
) -> Result<(), DbError> {
    match conn.execute(
        "INSERT INTO aliases (alias_name, exercise_name) VALUES (?1, ?2)",
        params![alias_name, canonical_exercise_name],
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                // Check for UNIQUE constraint violation specifically on 'aliases.alias_name'
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    return Err(DbError::AliasAlreadyExists(alias_name.to_string()));
                }
            }
            Err(DbError::InsertFailed(e)) // Wrap other errors
        }
    }
}

/// Deletes an alias by its name.
pub fn delete_alias(conn: &Connection, alias_name: &str) -> Result<u64, DbError> {
    let rows_affected = conn
        .execute(
            "DELETE FROM aliases WHERE alias_name = ?1 COLLATE NOCASE",
            params![alias_name],
        ) // Ensure case-insensitive match
        .map_err(DbError::DeleteFailed)?;
    if rows_affected == 0 {
        Err(DbError::AliasNotFound(alias_name.to_string()))
    } else {
        Ok(rows_affected as u64)
    }
}

/// Retrieves the canonical exercise name associated with an alias (case-insensitive).
pub fn get_canonical_name_for_alias(
    conn: &Connection,
    alias_name: &str,
) -> Result<Option<String>, DbError> {
    let mut stmt = conn
        .prepare("SELECT exercise_name FROM aliases WHERE alias_name = ?1 COLLATE NOCASE")
        .map_err(DbError::QueryFailed)?;
    stmt.query_row(params![alias_name], |row| row.get(0))
        .optional()
        .map_err(DbError::QueryFailed)
}

/// Lists all defined aliases and their corresponding canonical exercise names.
pub fn list_aliases(conn: &Connection) -> Result<HashMap<String, String>, DbError> {
    let mut stmt = conn
        .prepare("SELECT alias_name, exercise_name FROM aliases ORDER BY alias_name ASC")
        .map_err(DbError::QueryFailed)?;
    let alias_iter = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(DbError::QueryFailed)?;

    alias_iter
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(DbError::QueryFailed)
}

// --- Combined Identifier Resolution ---

/// Retrieves an exercise definition by trying ID first, then alias, then name.
/// Returns Option<(Definition, ResolvedByType)>.
#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedByType {
    Id,
    Alias,
    Name,
}

pub fn get_exercise_by_identifier(
    conn: &Connection,
    identifier: &str,
) -> Result<Option<(ExerciseDefinition, ResolvedByType)>, DbError> {
    // 1. Try parsing as ID
    if let Ok(id) = identifier.parse::<i64>() {
        if let Some(exercise) = get_exercise_by_id(conn, id)? {
            return Ok(Some((exercise, ResolvedByType::Id)));
        }
        // If it parsed as ID but wasn't found, don't proceed to alias/name check for IDs.
        // This prevents ambiguity if an alias/name happens to be numeric.
        return Ok(None);
    }

    // 2. Try resolving as Alias
    if let Some(canonical_name) = get_canonical_name_for_alias(conn, identifier)? {
        // Found alias, now get the definition using the canonical name
        match get_exercise_by_name(conn, &canonical_name)? {
            Some(exercise) => return Ok(Some((exercise, ResolvedByType::Alias))),
            None => {
                // Alias exists but points to a non-existent exercise (data inconsistency?)
                // Log warning or handle as appropriate. For now, return as not found.
                eprintln!(
                    "Warning: Alias '{}' points to non-existent exercise '{}'.",
                    identifier, canonical_name
                );
                return Ok(None);
            }
        }
    }

    // 3. Try resolving as Name
    match get_exercise_by_name(conn, identifier)? {
        Some(exercise) => Ok(Some((exercise, ResolvedByType::Name))),
        None => Ok(None), // Not found by ID, Alias, or Name
    }
}

/// Lists defined exercises, optionally filtering by type and/or muscle.
pub fn list_exercises(
    conn: &Connection,
    type_filter: Option<ExerciseType>,
    muscle_filter: Option<&str>,
) -> Result<Vec<ExerciseDefinition>, DbError> {
    // Return DbError
    let mut sql = "SELECT id, name, type, muscles FROM exercises WHERE 1=1".to_string();
    let mut params_map: HashMap<String, Box<dyn ToSql>> = HashMap::new();

    if let Some(t) = type_filter {
        sql.push_str(" AND type = :type");
        params_map.insert(":type".into(), Box::new(t.to_string()));
    }
    if let Some(m) = muscle_filter {
        sql.push_str(" AND muscles LIKE :muscle");
        params_map.insert(":muscle".into(), Box::new(format!("%{}%", m)));
    }
    sql.push_str(" ORDER BY name ASC");

    let params_for_query: Vec<(&str, &dyn ToSql)> = params_map
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_ref()))
        .collect();

    let mut stmt = conn.prepare(&sql).map_err(DbError::QueryFailed)?;
    let exercise_iter = stmt
        .query_map(params_for_query.as_slice(), map_row_to_exercise_definition)
        .map_err(DbError::QueryFailed)?;

    exercise_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed) // Collect results
}

// --- Personal Best Query Functions (Feature 4) ---

/// Gets the maximum weight lifted for a specific exercise (canonical name).
pub fn get_max_weight_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT MAX(weight) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND weight IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<f64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum reps performed in a single set for a specific exercise (canonical name).
pub fn get_max_reps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, DbError> {
    conn.query_row(
        // Note: This assumes reps are per set. If reps column means total reps for the entry, the interpretation changes.
        // Assuming reps is 'reps per set'.
        "SELECT MAX(reps) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND reps IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    // The query returns Option<Option<i64>>, flatten it
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum duration in minutes for a specific exercise (canonical name).
pub fn get_max_duration_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<i64>, DbError> {
    conn.query_row(
        "SELECT MAX(duration_minutes) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND duration_minutes IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    .map(|opt_opt| opt_opt.flatten())
}

/// Gets the maximum distance for a specific exercise (canonical name). Assumes distance stored in km.
pub fn get_max_distance_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT MAX(distance) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE AND distance IS NOT NULL", // Add COLLATE NOCASE
        params![canonical_exercise_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
    .map(|opt_opt| opt_opt.flatten())
}

// --- Statistics Query Functions ---

/// Retrieves all workout timestamps for a specific exercise, ordered chronologically.
pub fn get_workout_timestamps_for_exercise(
    conn: &Connection,
    canonical_exercise_name: &str,
) -> Result<Vec<DateTime<Utc>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT timestamp FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE ORDER BY timestamp ASC", // Add COLLATE NOCASE
    )?;
    let timestamp_iter = stmt.query_map(params![canonical_exercise_name], |row| {
        let timestamp_str: String = row.get(0)?;
        DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
    })?;

    timestamp_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

// --- Bodyweight Functions ---

/// Adds a new bodyweight entry.
pub fn add_bodyweight(
    conn: &Connection,
    timestamp: DateTime<Utc>,
    weight: f64,
) -> Result<i64, DbError> {
    let timestamp_str = timestamp.to_rfc3339();
    conn.execute(
        "INSERT INTO bodyweights (timestamp, weight) VALUES (?1, ?2)",
        params![timestamp_str, weight],
    )
    .map_err(|e| {
        // Handle potential UNIQUE constraint violation on timestamp nicely
        if let rusqlite::Error::SqliteFailure(ref err, _) = e {
            if err.code == rusqlite::ErrorCode::ConstraintViolation {
                return DbError::InsertFailed(rusqlite::Error::SqliteFailure(
                    err.clone(),
                    Some(format!(
                        "A bodyweight entry already exists for timestamp '{}'.",
                        timestamp_str
                    )),
                ));
            }
        }
        DbError::InsertFailed(e)
    })?;
    Ok(conn.last_insert_rowid())
}

/// Retrieves the most recent bodyweight entry.
pub fn get_latest_bodyweight(conn: &Connection) -> Result<Option<f64>, DbError> {
    conn.query_row(
        "SELECT weight FROM bodyweights ORDER BY timestamp DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::QueryFailed)
}

/// Retrieves all bodyweight entries, ordered by timestamp descending.
pub fn list_bodyweights(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<(DateTime<Utc>, f64)>, DbError> {
    let mut stmt =
        conn.prepare("SELECT timestamp, weight FROM bodyweights ORDER BY timestamp DESC LIMIT ?1")?;
    let iter = stmt.query_map(params![limit], |row| {
        let timestamp_str: String = row.get(0)?;
        let weight: f64 = row.get(1)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok((timestamp, weight))
    })?;
    iter.collect::<Result<Vec<_>, _>>()
        .map_err(DbError::QueryFailed)
}

//task-athlete-lib/src/lib.rs
use anyhow::{bail, Context, Result};
use chrono::format::Numeric;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc}; // Add Duration, TimeZone
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf}; // For list_aliases return type

// --- Declare modules to load from separate files ---
mod config;
pub mod db;

// --- Expose public types ---
pub use config::{
    get_config_path as get_config_path_util, // Rename utility function
    load_config as load_config_util,         // Rename utility function
    parse_color,                             // PbMetricScope removed
    save_config as save_config_util,         // Rename utility function
    Config,
    ConfigError,
    StandardColor,
    ThemeConfig,
    Units,
};
pub use db::{
    get_db_path as get_db_path_util, // Rename utility function
    DbError,
    ExerciseDefinition,
    ExerciseType,
    ResolvedByType,
    VolumeFilters,
    Workout,
    WorkoutFilters,
};

// --- Personal Best Information (Feature 4) ---
// Replaced PBType with boolean flags within PBInfo
// #[derive(Debug, Clone, PartialEq)]
// pub enum PBType {
//     Weight,
//     Reps,
//     Duration,
//     Distance,
//     // Combinations could be added if needed, but individual flags are simpler
// }

#[derive(Debug, Clone, PartialEq, Default)] // Add Default
pub struct PBInfo {
    pub achieved_weight_pb: bool,
    pub new_weight: Option<f64>,
    pub previous_weight: Option<f64>,
    pub achieved_reps_pb: bool,
    pub new_reps: Option<i64>,
    pub previous_reps: Option<i64>,
    pub achieved_duration_pb: bool,
    pub new_duration: Option<i64>,
    pub previous_duration: Option<i64>,
    pub achieved_distance_pb: bool,
    pub new_distance: Option<f64>,      // Stored as km
    pub previous_distance: Option<f64>, // Stored as km
}

impl PBInfo {
    /// Helper to check if any PB was achieved.
    pub fn any_pb(&self) -> bool {
        self.achieved_weight_pb
            || self.achieved_reps_pb
            || self.achieved_duration_pb
            || self.achieved_distance_pb
    }
}

// --- Statistics Types ---

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PersonalBests {
    pub max_weight: Option<f64>,
    pub max_reps: Option<i64>,
    pub max_duration_minutes: Option<i64>,
    pub max_distance_km: Option<f64>, // Always store in km
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseStats {
    pub canonical_name: String,
    pub total_workouts: usize,
    pub first_workout_date: Option<NaiveDate>,
    pub last_workout_date: Option<NaiveDate>,
    pub avg_workouts_per_week: Option<f64>,
    pub longest_gap_days: Option<u64>,
    pub personal_bests: PersonalBests,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub streak_interval_days: u32, // From config
}

// --- Service Layer ---

/// Main application service holding configuration and database connection.
pub struct AppService {
    pub config: Config,   // Public for reading by UI layers (CLI, TUI)
    pub conn: Connection, // Make mutable ONLY if needed (e.g., transactions directly in service) - currently DB funcs handle transactions
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

impl AppService {
    /// Initializes the application service by loading config and connecting to the DB.
    pub fn initialize() -> Result<Self> {
        let config_path =
            config::get_config_path().context("Failed to determine configuration file path")?;
        // Use the load_config function from the config module
        let config = config::load_config(&config_path)
            .context(format!("Failed to load config from {:?}", config_path))?;

        let db_path = db::get_db_path().context("Failed to determine database path")?;
        let mut conn =
            db::open_db(&db_path) // Use mutable conn for init potentially
                .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        db::init_db(&mut conn).context("Failed to initialize database schema")?; // Pass mutable conn

        Ok(Self {
            config,
            conn,
            db_path,
            config_path,
        })
    }

    // --- Configuration Methods ---

    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn save_config(&self) -> Result<(), ConfigError> {
        config::save_config(&self.config_path, &self.config)
    }

    /// Sets the bodyweight in the configuration and saves it.
    pub fn set_bodyweight(&mut self, weight: f64) -> Result<(), ConfigError> {
        if weight <= 0.0 {
            return Err(ConfigError::InvalidBodyweightInput(
                "Weight must be a positive number.".to_string(),
            ));
        }
        self.config.bodyweight = Some(weight);
        self.save_config()?;
        Ok(())
    }

    /// Checks if bodyweight is needed and returns it, or returns error if needed but not set.
    /// Does NOT prompt.
    pub fn get_required_bodyweight(&self) -> Result<f64, ConfigError> {
        self.config
            .bodyweight
            .ok_or_else(|| ConfigError::BodyweightNotSet(self.config_path.clone()))
    }

    /// Disables the bodyweight prompt in the config and saves it.
    pub fn disable_bodyweight_prompt(&mut self) -> Result<(), ConfigError> {
        self.config.prompt_for_bodyweight = false;
        self.save_config()
    }

    /// Sets the streak interval in the config and saves it.
    pub fn set_streak_interval(&mut self, days: u32) -> Result<(), ConfigError> {
        if days == 0 {
            // Although CLI parser prevents 0, add safeguard here
            return Err(ConfigError::InvalidBodyweightInput(
                "Streak interval must be at least 1 day.".to_string(),
            ));
        }
        self.config.streak_interval_days = days;
        self.save_config()
    }

    // --- PB Notification Config Methods ---

    /// Sets the global PB notification preference in the config and saves it.
    pub fn set_pb_notification_enabled(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_enabled = Some(enabled);
        self.save_config()
    }

    /// Checks the global PB notification config setting. Returns error if not set (needs prompt).
    pub fn check_pb_notification_config(&self) -> Result<bool, ConfigError> {
        self.config
            .notify_pb_enabled
            .ok_or(ConfigError::PbNotificationNotSet)
    }

    pub fn set_pb_notify_weight(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_weight = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_reps(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_reps = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_duration(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_duration = enabled;
        self.save_config()
    }
    pub fn set_pb_notify_distance(&mut self, enabled: bool) -> Result<(), ConfigError> {
        self.config.notify_pb_distance = enabled;
        self.save_config()
    }

    // --- Target Bodyweight Config Methods ---

    /// Sets the target bodyweight in the config and saves it.
    pub fn set_target_bodyweight(&mut self, weight: Option<f64>) -> Result<(), ConfigError> {
        if let Some(w) = weight {
            if w <= 0.0 {
                return Err(ConfigError::InvalidBodyweightInput(
                    "Target weight must be a positive number.".to_string(),
                ));
            }
        }
        self.config.target_bodyweight = weight;
        self.save_config()
    }

    /// Gets the target bodyweight from the config.
    pub fn get_target_bodyweight(&self) -> Option<f64> {
        self.config.target_bodyweight
    }

    // --- Units Config ---
    pub fn set_units(&mut self, units: Units) -> Result<(), ConfigError> {
        self.config.units = units;
        // Potentially add logic here later to convert existing weights/distances if desired,
        // but for now, just change the unit label.
        self.save_config()?;
        Ok(())
    }

    // --- Bodyweight Tracking Methods ---

    /// Adds a new bodyweight entry to the database.
    pub fn add_bodyweight_entry(&self, timestamp: DateTime<Utc>, weight: f64) -> Result<i64> {
        if weight <= 0.0 {
            bail!(ConfigError::InvalidBodyweightInput(
                "Bodyweight must be a positive number.".to_string()
            ));
        }
        db::add_bodyweight(&self.conn, timestamp, weight)
            .context("Failed to add bodyweight entry to database")
    }

    /// Retrieves the most recent bodyweight entry from the database.
    pub fn get_latest_bodyweight(&self) -> Result<Option<f64>> {
        db::get_latest_bodyweight(&self.conn)
            .context("Failed to retrieve latest bodyweight from database")
    }

    /// Lists logged bodyweight entries.
    pub fn list_bodyweights(&self, limit: u32) -> Result<Vec<(DateTime<Utc>, f64)>> {
        db::list_bodyweights(&self.conn, limit).context("Failed to list bodyweights from database")
    }

    // --- Database Path ---
    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }

    // --- Exercise Identifier Resolution (Helper) ---

    /// Resolves an identifier (ID, Alias, Name) to an ExerciseDefinition.
    /// Returns Ok(None) if not found, Err if DB error occurs.
    pub fn resolve_exercise_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            bail!("Exercise identifier cannot be empty.");
        }
        // Call function from db module
        db::get_exercise_by_identifier(&self.conn, trimmed)
            .map(|opt_result| opt_result.map(|(definition, _)| definition)) // Discard ResolvedByType here
            .context(format!(
                "Failed to resolve exercise identifier '{}'",
                identifier
            ))
    }

    /// Resolves an identifier (ID, Alias, Name) to its canonical name.
    /// Returns Ok(None) if not found, Err if DB error occurs.
    fn resolve_identifier_to_canonical_name(&self, identifier: &str) -> Result<Option<String>> {
        self.resolve_exercise_identifier(identifier)
            .map(|opt_def| opt_def.map(|def| def.name))
    }

    // --- Exercise Definition Methods ---

    /// Creates a new exercise definition. (Feature 2: Name uniqueness enforced by DB)
    pub fn create_exercise(
        &self,
        name: &str,
        type_: ExerciseType, // Use db::ExerciseType directly
        muscles: Option<&str>,
    ) -> Result<i64> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            bail!("Exercise name cannot be empty.");
        }
        // Call function from db module
        db::create_exercise(&self.conn, trimmed_name, &type_, muscles).map_err(
            |db_err| match db_err {
                DbError::ExerciseNameNotUnique(_) => anyhow::anyhow!(db_err), // Keep specific error message
                _ => anyhow::Error::new(db_err).context(format!(
                    "Failed to create exercise '{}' in database",
                    trimmed_name
                )),
            },
        )
    }

    /// Edits an existing exercise definition (identified by ID, Alias, or Name).
    /// Handles updates to workouts and aliases if name changes.
    pub fn edit_exercise(
        &mut self, // Takes mut self because db::update_exercise requires it for transaction
        identifier: &str,
        new_name: Option<&str>,
        new_type: Option<ExerciseType>,
        new_muscles: Option<Option<&str>>, // None = don't change, Some(None) = clear, Some(Some("val")) = set
    ) -> Result<u64> {
        // 1. Resolve identifier to get the *current* canonical name
        let current_def = self
            .resolve_exercise_identifier(identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
        let canonical_name_to_update = &current_def.name;

        // Trim new name if provided
        let trimmed_new_name = new_name.map(|n| n.trim()).filter(|n| !n.is_empty());
        if new_name.is_some() && trimmed_new_name.is_none() {
            bail!("New exercise name cannot be empty if provided.");
        }

        // 2. Call DB function with the canonical name
        // Need mutable connection borrow for the transaction inside db::update_exercise
        let mut_conn = &mut self.conn; // Create a mutable reference
        db::update_exercise(
            mut_conn,
            canonical_name_to_update,
            trimmed_new_name,
            new_type.as_ref(), // Pass Option<&DbType>
            new_muscles,
        )
        .map_err(|db_err| match db_err {
            // Make unique constraint violation error more specific
            DbError::ExerciseNameNotUnique(failed_name) => {
                anyhow::anyhow!(
                    "Failed to rename exercise: the name '{}' is already taken.",
                    failed_name
                )
            }
            DbError::ExerciseNotFound(_) => {
                anyhow::anyhow!("Exercise '{}' not found to edit.", identifier)
            } // Make not found error specific
            _ => anyhow::Error::new(db_err).context(format!(
                "Failed to update exercise '{}' in database",
                identifier
            )),
        })
    }

    /// Deletes an exercise definition (identified by ID, Alias, or Name). Returns number of definitions deleted (0 or 1).
    /// Includes warnings about associated workouts and deletes associated aliases.
    pub fn delete_exercise(&mut self, identifiers: &Vec<String>) -> Result<u64> {
        let mut num_deleted = 0;
        for identifier in identifiers {
            // 1. Resolve identifier to get canonical name and check existence
            let exercise_def = self
                .resolve_exercise_identifier(identifier)?
                .ok_or_else(|| DbError::ExerciseNotFound(identifier.to_string()))?;
            let canonical_name = exercise_def.name.clone(); // Clone needed for messages/DB call

            // 2. Check for associated workouts (using canonical name)
            let workout_count: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM workouts WHERE exercise_name = ?1 COLLATE NOCASE",
                    [&canonical_name], // Use canonical name
                    |row| row.get(0),
                )
                .context(format!(
                    "Failed to check for associated workouts for '{}'",
                    canonical_name
                ))?;

            if workout_count > 0 {
                // Print warning here. Ideally UI layer formats this, but simpler here for now.
                eprintln!(
                "Warning: Deleting exercise '{}'. {} associated workout entries will remain but reference a deleted definition.",
                canonical_name, workout_count
             );
            }

            // 3. Call DB function to delete exercise and its aliases (using canonical name)
            // Need mutable connection borrow for the transaction inside db::delete_exercise
            let mut_conn = &mut self.conn; // Create a mutable reference
            db::delete_exercise(mut_conn, &canonical_name).map_err(|e| match e {
                DbError::ExerciseNotFound(_) => {
                    anyhow::anyhow!("Exercise '{}' not found to delete.", identifier)
                } // Should not happen if resolve worked, but handle anyway
                _ => anyhow::Error::new(e).context(format!(
                    "Failed to delete exercise '{}' from database",
                    canonical_name
                )),
            })?;
            num_deleted += 1;
        }
        Ok(num_deleted)
    }

    /// Retrieves an exercise definition by ID, Alias or name.
    pub fn get_exercise_by_identifier_service(
        &self,
        identifier: &str,
    ) -> Result<Option<ExerciseDefinition>> {
        self.resolve_exercise_identifier(identifier)
    }

    /// Lists exercise definitions based on filters.
    pub fn list_exercises(
        &self,
        type_filter: Option<ExerciseType>,
        muscle_filter: Option<&str>,
    ) -> Result<Vec<ExerciseDefinition>> {
        // Call function from db module
        db::list_exercises(&self.conn, type_filter, muscle_filter)
            .context("Failed to list exercise definitions from database")
    }

    // --- Alias Methods (Feature 1) ---

    /// Creates a new alias for an exercise.
    pub fn create_alias(&self, alias_name: &str, exercise_identifier: &str) -> Result<()> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        // Ensure alias doesn't clash with existing exercise IDs or names
        if let Some((_, resolved_type)) = db::get_exercise_by_identifier(&self.conn, trimmed_alias)?
        {
            match resolved_type {
                ResolvedByType::Id => bail!(
                    "Alias '{}' conflicts with an existing exercise ID.",
                    trimmed_alias
                ),
                ResolvedByType::Name => bail!(
                    "Alias '{}' conflicts with an existing exercise name.",
                    trimmed_alias
                ),
                ResolvedByType::Alias => { /* This is handled by the INSERT constraint */ }
            }
        }

        // Resolve the target exercise identifier to its canonical name
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Call DB function
        db::create_alias(&self.conn, trimmed_alias, &canonical_name).map_err(
            |db_err| match db_err {
                DbError::AliasAlreadyExists(_) => anyhow::anyhow!(db_err), // Keep specific error
                _ => anyhow::Error::new(db_err).context(format!(
                    "Failed to create alias '{}' for exercise '{}'",
                    trimmed_alias, canonical_name
                )),
            },
        )
    }

    /// Deletes an exercise alias.
    pub fn delete_alias(&self, alias_name: &str) -> Result<u64> {
        let trimmed_alias = alias_name.trim();
        if trimmed_alias.is_empty() {
            bail!("Alias name cannot be empty.");
        }
        db::delete_alias(&self.conn, trimmed_alias).map_err(|db_err| match db_err {
            DbError::AliasNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
            _ => anyhow::Error::new(db_err)
                .context(format!("Failed to delete alias '{}'", trimmed_alias)),
        })
    }

    /// Lists all defined aliases.
    pub fn list_aliases(&self) -> Result<HashMap<String, String>> {
        db::list_aliases(&self.conn).context("Failed to list aliases from database")
    }

    // --- Workout Entry Methods ---

    /// Adds a workout entry. Handles implicit exercise creation, bodyweight logic, past dates, and PB checking.
    /// Stores distance in km.
    /// Returns Result<(workout_id, Option<PBInfo>)>
    pub fn add_workout(
        &mut self, // Needs mut because bodyweight prompt might update config via caller
        exercise_identifier: &str,
        date: NaiveDate,
        sets: Option<i64>,
        reps: Option<i64>,
        weight_arg: Option<f64>, // Weight from CLI/TUI args
        duration: Option<i64>,
        distance_arg: Option<f64>, // Distance from CLI/TUI args
        notes: Option<String>,
        // For implicit creation:
        implicit_type: Option<ExerciseType>,
        implicit_muscles: Option<String>,
        // Bodyweight handling (determined by caller):
        bodyweight_to_use: Option<f64>, // If type is BodyWeight and caller prompted/knows BW
    ) -> Result<(i64, Option<PBInfo>)> {
        // Feature 4: Return PB info
        // 1. Resolve identifier / Implicitly create Exercise Definition
        let exercise_def = match self.resolve_exercise_identifier(exercise_identifier)? {
            Some(def) => def,
            None => {
                // Try implicit creation
                if let (Some(db_type), Some(muscle_list)) = (implicit_type, implicit_muscles) {
                    println!(
                        // Keep CLI print for now
                        "Exercise '{}' not found, defining it implicitly...",
                        exercise_identifier
                    );
                    let muscles_opt = if muscle_list.trim().is_empty() {
                        None
                    } else {
                        Some(muscle_list.as_str())
                    };
                    match self.create_exercise(exercise_identifier, db_type, muscles_opt) {
                        Ok(id) => {
                            println!(
                                "Implicitly defined exercise: '{}' (ID: {})",
                                exercise_identifier, id
                            );
                            // Refetch the newly created definition
                            self.resolve_exercise_identifier(exercise_identifier)?
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Failed to re-fetch implicitly created exercise '{}'",
                                        exercise_identifier
                                    )
                                })?
                        }
                        Err(e) => {
                            return Err(e).context(format!(
                                "Failed to implicitly define exercise '{}'",
                                exercise_identifier
                            ));
                        }
                    }
                } else {
                    // Not found and no implicit creation info provided
                    bail!(
                          "Exercise '{}' not found. Define it first using 'create-exercise', use an alias, or provide details for implicit creation.",
                          exercise_identifier
                      );
                }
            }
        };

        let canonical_exercise_name = exercise_def.name.clone(); // Use canonical name from here

        // 2. Determine final weight based on type and provided bodyweight
        let final_weight = if exercise_def.type_ == ExerciseType::BodyWeight {
            match bodyweight_to_use {
                Some(bw) => Some(bw + weight_arg.unwrap_or(0.0)),
                None => bail!(
                    "Bodyweight is required for exercise '{}' but was not provided.",
                    canonical_exercise_name
                ),
            }
        } else {
            weight_arg // Use the provided weight directly for non-bodyweight exercises
        };

        // 3. Convert distance to km if necessary and store
        let final_distance_km = match distance_arg {
            Some(dist) => {
                match self.config.units {
                    Units::Metric => Some(dist),             // Assume input is already km
                    Units::Imperial => Some(dist * 1.60934), // Convert miles to km
                }
            }
            None => None,
        };

        // 4. Determine timestamp (Feature 3)
        // Use noon UTC on the given date to represent the day without time specifics
        let date_and_time = date
            .and_hms_opt(12, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Internal error creating NaiveDateTime from date"))?;
        let timestamp = Utc.from_utc_datetime(&date_and_time);

        // 5. Check for PBs *before* adding the new workout (Feature 4)
        let previous_max_weight =
            db::get_max_weight_for_exercise(&self.conn, &canonical_exercise_name)?;
        let previous_max_reps =
            db::get_max_reps_for_exercise(&self.conn, &canonical_exercise_name)?;
        let previous_max_duration =
            db::get_max_duration_for_exercise(&self.conn, &canonical_exercise_name)?;
        let previous_max_distance_km =
            db::get_max_distance_for_exercise(&self.conn, &canonical_exercise_name)?;

        // 6. Add the workout entry using the canonical exercise name, final weight, distance(km), and timestamp
        let inserted_id = db::add_workout(
            &self.conn,
            &canonical_exercise_name, // Use canonical name
            timestamp,
            sets,
            reps,
            final_weight, // Use calculated weight
            duration,
            final_distance_km, // Store distance in km
            notes,
        )
        .context("Failed to add workout to database")?;

        // 7. Determine if a PB was achieved (Feature 4)
        let mut pb_info = PBInfo {
            previous_weight: previous_max_weight,
            previous_reps: previous_max_reps,
            previous_duration: previous_max_duration,
            previous_distance: previous_max_distance_km,
            new_weight: final_weight,
            new_reps: reps,
            new_duration: duration,
            new_distance: final_distance_km,
            ..Default::default() // Initialize achieved flags to false
        };

        // Check weight PB
        if self.config.notify_pb_weight {
            if let Some(current_weight) = final_weight {
                if current_weight > 0.0 && current_weight > previous_max_weight.unwrap_or(0.0) {
                    pb_info.achieved_weight_pb = true;
                }
            }
        }
        // Check reps PB
        if self.config.notify_pb_reps {
            if let Some(current_reps) = reps {
                if current_reps > 0 && current_reps > previous_max_reps.unwrap_or(0) {
                    pb_info.achieved_reps_pb = true;
                }
            }
        }
        // Check duration PB
        if self.config.notify_pb_duration {
            if let Some(current_duration) = duration {
                if current_duration > 0 && current_duration > previous_max_duration.unwrap_or(0) {
                    pb_info.achieved_duration_pb = true;
                }
            }
        }
        // Check distance PB
        if self.config.notify_pb_distance {
            if let Some(current_distance_km) = final_distance_km {
                // Use a small epsilon for float comparison? Might be overkill for distance PBs.
                if current_distance_km > 0.0
                    && current_distance_km > previous_max_distance_km.unwrap_or(0.0)
                {
                    pb_info.achieved_distance_pb = true;
                }
            }
        }

        // Return ID and PB info only if a PB was actually achieved
        let result_pb_info = if pb_info.any_pb() {
            Some(pb_info)
        } else {
            None
        };
        Ok((inserted_id, result_pb_info))
    }

    /// Edits an existing workout entry. Converts distance to km if units are Imperial.
    pub fn edit_workout(
        &self,
        id: i64,
        new_exercise_identifier: Option<String>,
        new_sets: Option<i64>,
        new_reps: Option<i64>,
        new_weight: Option<f64>, // Weight is set directly, no bodyweight logic re-applied
        new_duration: Option<i64>,
        new_distance_arg: Option<f64>, // Distance argument from CLI/TUI
        new_notes: Option<String>,
        new_date: Option<NaiveDate>, // Feature 3: Allow editing date
    ) -> Result<u64> {
        // Resolve the new exercise identifier to its canonical name if provided
        let new_canonical_name: Option<String> = match new_exercise_identifier {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(&ident)?
                    .ok_or_else(|| DbError::ExerciseNotFound(ident.clone()))?,
            ),
            None => None,
        };

        // Convert new_date to new_timestamp if provided
        let new_timestamp: Option<DateTime<Utc>> = match new_date {
            Some(date) => Some(
                date.and_hms_opt(12, 0, 0) // Create NaiveDateTime first
                    .and_then(|naive_dt| Utc.from_local_datetime(&naive_dt).single()) // Convert to DateTime<Utc>
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to create valid timestamp from date {}", date)
                    })?,
            ),
            None => None,
        };

        // Convert distance to km if necessary
        let new_distance_km = match new_distance_arg {
            Some(dist) => {
                match self.config.units {
                    Units::Metric => Some(dist),             // Assume input is already km
                    Units::Imperial => Some(dist * 1.60934), // Convert miles to km
                }
            }
            None => None,
        };

        // Call function from db module
        db::update_workout(
            &self.conn,
            id,
            new_canonical_name.as_deref(), // Pass Option<&str>
            new_sets,
            new_reps,
            new_weight, // Pass Option<f64> directly
            new_duration,
            new_distance_km,      // Pass Option<f64> (km)
            new_notes.as_deref(), // Pass Option<&str>
            new_timestamp,        // Pass Option<DateTime<Utc>>
        )
        .with_context(|| format!("Failed to update workout ID {}", id))
    }

    /// Deletes a workout entry by ID.
    pub fn delete_workouts(&self, ids: &Vec<i64>) -> Result<Vec<u64>> {
        // Call function from db module
        let mut workouts_delete = vec![];
        for id in ids {
            db::delete_workout(&self.conn, *id).map_err(|db_err| match db_err {
                DbError::WorkoutNotFound(_) => anyhow::anyhow!(db_err), // Keep specific error
                _ => anyhow::Error::new(db_err)
                    .context(format!("Failed to delete workout ID {}", id)),
            })?;
            workouts_delete.push(*id as u64);
        }
        Ok(workouts_delete)
    }

    /// Lists workouts based on filters. Resolves exercise identifier if provided.
    pub fn list_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>> {
        // Resolve exercise identifier filter to canonical name if present
        let canonical_exercise_name = match filters.exercise_name {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(ident)?
                    .ok_or_else(|| {
                        // If identifier doesn't resolve, treat as no matching workouts found
                        eprintln!(
                            "Warning: Exercise identifier '{}' not found for filtering.",
                            ident
                        );
                        DbError::ExerciseNotFound(ident.to_string()) // Return specific error
                    })?,
            ),
            None => None,
        };

        // Create new filters struct with resolved name
        let resolved_filters = WorkoutFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            date: filters.date,
            exercise_type: filters.exercise_type,
            muscle: filters.muscle,
            limit: filters.limit,
        };

        // Call function from db module
        db::list_workouts_filtered(&self.conn, resolved_filters)
            .context("Failed to list workouts from database")
    }

    /// Lists workouts for the Nth most recent day a specific exercise (ID, Alias, Name) was performed.
    pub fn list_workouts_for_exercise_on_nth_last_day(
        &self,
        exercise_identifier: &str,
        n: u32,
    ) -> Result<Vec<Workout>> {
        // Resolve identifier to canonical name
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // Call function from db module
        db::list_workouts_for_exercise_on_nth_last_day(&self.conn, &canonical_name, n)
            .map_err(|e| anyhow::Error::new(e)) // Convert DbError to anyhow::Error
            .with_context(|| {
                format!(
                    "Failed to list workouts for exercise '{}' on nth last day {}",
                    canonical_name, n
                )
            })
    }

    // --- Statistics Method ---
    pub fn get_exercise_stats(&self, exercise_identifier: &str) -> Result<ExerciseStats> {
        // 1. Resolve identifier
        let canonical_name = self
            .resolve_identifier_to_canonical_name(exercise_identifier)?
            .ok_or_else(|| DbError::ExerciseNotFound(exercise_identifier.to_string()))?;

        // 2. Get all timestamps for the exercise
        let timestamps = db::get_workout_timestamps_for_exercise(&self.conn, &canonical_name)
            .context(format!(
                "Failed to retrieve workout history for '{}'",
                canonical_name
            ))?;

        if timestamps.is_empty() {
            return Err(DbError::NoWorkoutDataFound(canonical_name).into());
        }

        // 3. Calculate basic stats
        let total_workouts = timestamps.len();
        let first_timestamp = timestamps.first().unwrap(); // Safe due to is_empty check
        let last_timestamp = timestamps.last().unwrap(); // Safe due to is_empty check
        let first_workout_date = Some(first_timestamp.date_naive());
        let last_workout_date = Some(last_timestamp.date_naive());

        // 4. Calculate average workouts per week
        let avg_workouts_per_week = if total_workouts <= 1 {
            None // Cannot calculate average for 0 or 1 workout
        } else {
            let duration_days = (*last_timestamp - *first_timestamp).num_days();
            if duration_days == 0 {
                // Multiple workouts on the same day - technically infinite avg/week, return None or handle differently?
                // Let's consider it as "at least daily" which doesn't fit avg/week well. Return None.
                None
            } else {
                let duration_weeks = (duration_days as f64 / 7.0).max(1.0 / 7.0); // Avoid division by zero, ensure at least 1 day = 1/7 week
                Some(total_workouts as f64 / duration_weeks)
            }
        };

        // 5. Calculate longest gap
        let mut longest_gap_days: Option<u64> = None;
        if total_workouts > 1 {
            let mut max_gap: i64 = 0;
            for i in 1..total_workouts {
                let gap =
                    (timestamps[i].date_naive() - timestamps[i - 1].date_naive()).num_days() - 1;
                if gap > max_gap {
                    max_gap = gap;
                }
            }
            longest_gap_days = Some(max_gap as u64); // Convert to u64
        }

        // 6. Calculate streaks
        let streak_interval = Duration::days(self.config.streak_interval_days as i64);
        let mut current_streak = 0u32;
        let mut longest_streak = 0u32;

        if total_workouts > 0 {
            current_streak = 1; // Start with 1 for the first workout
            longest_streak = 1;
            let mut last_streak_date = timestamps[0].date_naive();

            for i in 1..total_workouts {
                let current_date = timestamps[i].date_naive();
                // Ignore multiple workouts on the same day for streak calculation
                if current_date == last_streak_date {
                    continue;
                }
                // Check if the gap is within the allowed interval
                if current_date - last_streak_date <= streak_interval {
                    current_streak += 1;
                } else {
                    // Streak broken, reset current streak
                    current_streak = 1;
                }
                // Update longest streak if current is longer
                if current_streak > longest_streak {
                    longest_streak = current_streak;
                }
                last_streak_date = current_date; // Update the date for the next comparison
            }

            // Check if the *current* streak is still active based on the last workout date and today
            let today = Utc::now().date_naive();
            if today - last_timestamp.date_naive() > streak_interval {
                current_streak = 0; // Current streak is broken if the last workout is too old
            }
        }

        // 7. Get Personal Bests
        let personal_bests = PersonalBests {
            max_weight: db::get_max_weight_for_exercise(&self.conn, &canonical_name)?,
            max_reps: db::get_max_reps_for_exercise(&self.conn, &canonical_name)?,
            max_duration_minutes: db::get_max_duration_for_exercise(&self.conn, &canonical_name)?,
            max_distance_km: db::get_max_distance_for_exercise(&self.conn, &canonical_name)?,
        };

        Ok(ExerciseStats {
            canonical_name,
            total_workouts,
            first_workout_date,
            last_workout_date,
            avg_workouts_per_week,
            longest_gap_days,
            personal_bests,
            current_streak,
            longest_streak,
            streak_interval_days: self.config.streak_interval_days,
        })
    }

    pub fn calculate_daily_volume(
        &self,
        filters: VolumeFilters,
    ) -> Result<Vec<(NaiveDate, String, f64)>> {
        // Resolve exercise identifier filter to canonical name if present
        let canonical_exercise_name = match filters.exercise_name {
            Some(ident) => Some(
                self.resolve_identifier_to_canonical_name(ident)?
                    .ok_or_else(|| {
                        eprintln!(
                            "Warning: Exercise identifier '{}' not found for filtering volume.",
                            ident
                        );
                        DbError::ExerciseNotFound(ident.to_string())
                    })?,
            ),
            None => None,
        };

        // Create new filters struct with resolved name
        let resolved_filters = VolumeFilters {
            exercise_name: canonical_exercise_name.as_deref(),
            ..filters // Copy other filters (dates, type, muscle, limit)
        };

        db::calculate_daily_volume_filtered(&self.conn, resolved_filters)
            .context("Failed to calculate workout volume from database")
    }
}

//task-athlete-lib/tests/lib_test.rs
use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use rusqlite::Connection;
use std::thread; // For adding delays in PB tests
use std::time::Duration as StdDuration; // For delays
use task_athlete_lib::{
    AppService, Config, ConfigError, DbError, ExerciseType, Units, VolumeFilters, WorkoutFilters,
}; // For mutable connection helper

// Helper function to create a test service with in-memory database
fn create_test_service() -> Result<AppService> {
    // Create an in-memory database for testing
    // Need a mutable connection to pass to init_db
    let mut conn = rusqlite::Connection::open_in_memory()?;
    task_athlete_lib::db::init_db(&mut conn)?; // Pass mutable conn

    // Create a default config for testing
    let config = Config {
        bodyweight: Some(70.0), // Set a default bodyweight for tests
        units: Units::Metric,
        prompt_for_bodyweight: true,
        notify_pb_enabled: Some(true), // Explicitly enable for most tests
        streak_interval_days: 1,       // Default streak interval
        notify_pb_weight: true,
        notify_pb_reps: true,
        notify_pb_duration: true,
        notify_pb_distance: true,
        ..Default::default()
    };

    Ok(AppService {
        config,
        conn, // Store the initialized connection
        db_path: ":memory:".into(),
        config_path: "test_config.toml".into(),
    })
}

// Helper to get a separate mutable connection for tests requiring transactions (like edit_exercise)
// This is a bit of a hack because AppService owns its connection.
fn create_mutable_conn_to_test_db() -> Result<Connection> {
    let mut conn = rusqlite::Connection::open_in_memory()?;
    task_athlete_lib::db::init_db(&mut conn)?; // Ensure schema is initialized
    Ok(conn)
}

#[test]
fn test_create_exercise_unique_name() -> Result<()> {
    let service = create_test_service()?;
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Try creating with same name (case-insensitive)
    let result = service.create_exercise("bench press", ExerciseType::Cardio, None);
    assert!(result.is_err());
    // Check for the specific error type/message if desired
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Exercise name must be unique"));

    // Try creating with different name
    let result = service.create_exercise("Squat", ExerciseType::Resistance, Some("legs"));
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_exercise_aliases() -> Result<()> {
    let mut service = create_test_service()?;

    let ex_id = service.create_exercise(
        "Barbell Bench Press",
        ExerciseType::Resistance,
        Some("chest"),
    )?;
    service.create_exercise("Squat", ExerciseType::Resistance, Some("Legs"))?;

    // 1. Create Alias
    service.create_alias("bp", "Barbell Bench Press")?;

    // 2. List Aliases
    let aliases = service.list_aliases()?;
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases.get("bp").unwrap(), "Barbell Bench Press");

    // 3. Resolve Alias
    let resolved_def = service.resolve_exercise_identifier("bp")?.unwrap();
    assert_eq!(resolved_def.name, "Barbell Bench Press");
    assert_eq!(resolved_def.id, ex_id);

    // 4. Try creating duplicate alias
    let result = service.create_alias("bp", "Squat"); // Different exercise, same alias
    assert!(result.is_err());
    // println!("{:?}",result); // Keep for debugging if needed
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Alias already exists"));

    // 5. Try creating alias conflicting with name/id
    let result = service.create_alias("Barbell Bench Press", "Squat"); // Alias conflicts with name
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("conflicts with an existing exercise name"));

    let result = service.create_alias(&ex_id.to_string(), "Squat"); // Alias conflicts with ID
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("conflicts with an existing exercise ID"));

    // 6. Use Alias in Add Workout
    let today = Utc::now().date_naive();
    let (workout_id, _) = service.add_workout(
        "bp",
        today,
        Some(3),
        Some(5),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("bp"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].exercise_name, "Barbell Bench Press"); // Stored canonical name

    // 7. Delete Alias
    let deleted_count = service.delete_alias("bp")?;
    assert_eq!(deleted_count, 1);
    let aliases_after_delete = service.list_aliases()?;
    assert!(aliases_after_delete.is_empty());

    // 8. Try deleting non-existent alias
    let result = service.delete_alias("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Alias not found"));

    Ok(())
}

// Test for editing exercise name and having aliases/workouts update
#[test]
fn test_edit_exercise_with_alias_and_name_change() -> Result<()> {
    let mut service = create_test_service()?; // Service owns its connection

    service.create_exercise("Old Name", ExerciseType::Resistance, Some("muscle1"))?;
    service.create_alias("on", "Old Name")?;

    // Add a workout using the alias
    let today = Utc::now().date_naive();
    service.add_workout(
        "on",
        today,
        Some(1),
        Some(1),
        Some(1.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Edit using the alias identifier, changing the name and muscles
    service.edit_exercise(
        "on", // Identify by alias
        Some("New Name"),
        None,                          // Keep type
        Some(Some("muscle1,muscle2")), // Change muscles
    )?;

    // --- Verification using the same service instance ---

    // 1. Check old name resolves to None
    assert!(service.resolve_exercise_identifier("Old Name")?.is_none());

    // 2. Check alias now points to the new definition
    let resolved_by_alias = service
        .resolve_exercise_identifier("on")?
        .expect("Alias 'on' should resolve");
    assert_eq!(resolved_by_alias.name, "New Name");
    assert_eq!(
        resolved_by_alias.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 3. Check new name resolves correctly
    let resolved_by_new_name = service
        .resolve_exercise_identifier("New Name")?
        .expect("'New Name' should resolve");
    assert_eq!(resolved_by_new_name.id, resolved_by_alias.id); // Ensure same exercise ID
    assert_eq!(resolved_by_new_name.name, "New Name");
    assert_eq!(
        resolved_by_new_name.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 4. Check the alias list still contains the alias pointing to the NEW name
    let aliases = service.list_aliases()?;
    assert_eq!(
        aliases.get("on").expect("Alias 'on' should exist"),
        "New Name"
    );

    // 5. Check the workout entry associated with the old name was updated
    //    List workouts using the *alias* which should now resolve to "New Name"
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("on"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1, "Should find one workout via alias");
    assert_eq!(
        workouts[0].exercise_name, "New Name",
        "Workout exercise name should be updated"
    );

    //    List workouts using the *new name*
    let workouts_new_name = service.list_workouts(WorkoutFilters {
        exercise_name: Some("New Name"),
        ..Default::default()
    })?;
    assert_eq!(
        workouts_new_name.len(),
        1,
        "Should find one workout via new name"
    );
    assert_eq!(
        workouts_new_name[0].id, workouts[0].id,
        "Workout IDs should match"
    );

    //    List workouts using the *old name* (should find none)
    let workouts_old_name = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Old Name"),
        ..Default::default()
    });
    // Expect ExerciseNotFound error because the filter tries to resolve "Old Name" which no longer exists
    assert!(workouts_old_name.is_err());
    assert!(workouts_old_name
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_delete_exercise_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("To Delete", ExerciseType::Cardio, None)?;
    service.create_alias("td", "To Delete")?;

    // Delete exercise using alias
    let result = service.delete_exercise(&vec!["td".to_string()])?;
    assert_eq!(result, 1);

    // Verify exercise is gone
    assert!(service.resolve_exercise_identifier("To Delete")?.is_none());
    assert!(service.resolve_exercise_identifier("td")?.is_none());

    // Verify alias is gone
    let aliases = service.list_aliases()?;
    assert!(aliases.is_empty());

    Ok(())
}

#[test]
fn test_add_workout_past_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Rowing", ExerciseType::Cardio, None)?;

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let two_days_ago = Utc::now().date_naive() - Duration::days(2);

    service.add_workout(
        "Rowing",
        yesterday,
        None,
        None,
        None,
        Some(30),
        None,
        None,
        None,
        None,
        None,
    )?;
    service.add_workout(
        "Rowing",
        two_days_ago,
        None,
        None,
        None,
        Some(25),
        None,
        None,
        None,
        None,
        None,
    )?;

    // List for yesterday
    let workouts_yesterday = service.list_workouts(WorkoutFilters {
        date: Some(yesterday),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].duration_minutes, Some(30));
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    // List for two days ago
    let workouts_two_days_ago = service.list_workouts(WorkoutFilters {
        date: Some(two_days_ago),
        ..Default::default()
    })?;
    assert_eq!(workouts_two_days_ago.len(), 1);
    assert_eq!(workouts_two_days_ago[0].duration_minutes, Some(25));
    assert_eq!(
        workouts_two_days_ago[0].timestamp.date_naive(),
        two_days_ago
    );

    Ok(())
}

#[test]
fn test_edit_workout_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Push-ups", ExerciseType::BodyWeight, None)?;
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    let (workout_id, _) = service.add_workout(
        "Push-ups",
        today,
        Some(3),
        Some(15),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(70.0),
    )?;

    // Edit the date
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(yesterday),
    )?;

    // Verify date change by listing
    let workouts_today = service.list_workouts(WorkoutFilters {
        date: Some(today),
        ..Default::default()
    })?;
    assert!(workouts_today.is_empty());

    let workouts_yesterday = service.list_workouts(WorkoutFilters {
        date: Some(yesterday),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].id, workout_id);
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    Ok(())
}

// TODO
// Updated PB test to reflect new PBInfo structure and separate config flags
// #[test]
// fn test_pb_detection_and_config() -> Result<()> {
//     let mut service = create_test_service()?; // Uses default config with all PBs enabled
//     service.create_exercise("Deadlift", ExerciseType::Resistance, Some("back,legs"))?;
//     service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
//     let today = Utc::now().date_naive();

//     // --- Test Default Config (All PBs Enabled) ---

//     // Workout 1: Baseline
//     let (_, pb1) = service.add_workout("Deadlift", today, Some(1), Some(5), Some(100.0), None, None, None, None, None, None)?;
//     assert!(pb1.is_none(), "First workout shouldn't be a PB");
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 2: Weight PB
//     let (_, pb2) = service.add_workout("Deadlift", today, Some(1), Some(3), Some(110.0), None, None, None, None, None, None)?;
//     assert!(pb2.is_some(), "Should detect weight PB");
//     let info2 = pb2.unwrap();
//     assert!(info2.achieved_weight_pb);
//     assert!(!info2.achieved_reps_pb);
//     assert_eq!(info2.new_weight, Some(110.0));
//     assert_eq!(info2.previous_weight, Some(100.0)); // Previous was 100
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 3: Reps PB
//     let (_, pb3) = service.add_workout("Deadlift", today, Some(3), Some(6), Some(90.0), None, None, None, None, None, None)?;
//     assert!(pb3.is_some(), "Should detect reps PB");
//     let info3 = pb3.unwrap();
//     assert!(!info3.achieved_weight_pb);
//     assert!(info3.achieved_reps_pb);
//     assert_eq!(info3.new_reps, Some(6));
//     assert_eq!(info3.previous_reps, Some(5)); // Previous was 5
//     thread::sleep(StdDuration::from_millis(10));

//      // Workout 4: Both Weight and Reps PB
//     let (_, pb4) = service.add_workout("Deadlift", today, Some(1), Some(7), Some(120.0), None, None, None, None, None, None)?;
//     assert!(pb4.is_some(), "Should detect both PBs");
//     let info4 = pb4.unwrap();
//     assert!(info4.achieved_weight_pb);
//     assert!(info4.achieved_reps_pb);
//     assert_eq!(info4.new_weight, Some(120.0));
//     assert_eq!(info4.previous_weight, Some(110.0)); // Previous was 110
//     assert_eq!(info4.new_reps, Some(7));
//     assert_eq!(info4.previous_reps, Some(6)); // Previous was 6
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 5: No PB
//     let (_, pb5) = service.add_workout("Deadlift", today, Some(5), Some(5), Some(105.0), None, None, None, None, None, None)?;
//     assert!(pb5.is_none(), "Should not detect PB");
//     thread::sleep(StdDuration::from_millis(10));

//     // --- Test Disabling Specific PBs ---
//     service.set_pb_notify_reps(false)?; // Disable Rep PB notifications

//     // Workout 6: Weight PB (should be detected)
//     let (_, pb6) = service.add_workout("Deadlift", today, Some(1), Some(4), Some(130.0), None, None, None, None, None, None)?;
//     assert!(pb6.is_some(), "Weight PB should still be detected");
//     assert!(pb6.unwrap().achieved_weight_pb);
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 7: Reps PB (should NOT be detected as PB *notification*)
//     let (_, pb7) = service.add_workout("Deadlift", today, Some(1), Some(8), Some(125.0), None, None, None, None, None, None)?;
//     assert!(pb7.is_none(), "Reps PB should NOT trigger notification when disabled");
//     thread::sleep(StdDuration::from_millis(10));

//     // --- Test Duration/Distance PBs ---
//     service.set_pb_notify_reps(true)?; // Re-enable reps
//     service.set_pb_notify_weight(false)?; // Disable weight

//     // Running Workout 1: Baseline
//      let (_, rpb1) = service.add_workout("Running", today, None, None, None, Some(30), Some(5.0), None, None, None, None)?; // 5km in 30min
//      assert!(rpb1.is_none());
//      thread::sleep(StdDuration::from_millis(10));

//      // Running Workout 2: Duration PB (longer duration, same distance)
//      let (_, rpb2) = service.add_workout("Running", today, None, None, None, Some(35), Some(5.0), None, None, None, None)?;
//      assert!(rpb2.is_some());
//      let rinfo2 = rpb2.unwrap();
//      assert!(rinfo2.achieved_duration_pb);
//      assert!(!rinfo2.achieved_distance_pb);
//      assert_eq!(rinfo2.new_duration, Some(35));
//      assert_eq!(rinfo2.previous_duration, Some(30));
//      thread::sleep(StdDuration::from_millis(10));

//       // Running Workout 3: Distance PB (longer distance, irrelevant duration)
//      let (_, rpb3) = service.add_workout("Running", today, None, None, None, Some(25), Some(6.0), None, None, None, None)?;
//      assert!(rpb3.is_some());
//      let rinfo3 = rpb3.unwrap();
//      assert!(!rinfo3.achieved_duration_pb);
//      assert!(rinfo3.achieved_distance_pb);
//      assert_eq!(rinfo3.new_distance, Some(6.0));
//      assert_eq!(rinfo3.previous_distance, Some(5.0));
//      thread::sleep(StdDuration::from_millis(10));

//      // Running Workout 4: Disable distance PB, achieve distance PB -> No notification
//      service.set_pb_notify_distance(false)?;
//      let (_, rpb4) = service.add_workout("Running", today, None, None, None, Some(40), Some(7.0), None, None, None, None)?;
//      assert!(rpb4.is_some(), "Duration PB should still trigger"); // Duration PB is still active
//      let rinfo4 = rpb4.unwrap();
//      assert!(rinfo4.achieved_duration_pb);
//      assert!(!rinfo4.achieved_distance_pb, "Distance PB flag should be false in returned info if notify disabled"); // Flag should reflect config state at time of adding

//     Ok(())
// }

#[test]
fn test_create_and_list_exercises() -> Result<()> {
    let service = create_test_service()?;

    // Create some exercises
    service.create_exercise(
        "Bench Press",
        ExerciseType::Resistance,
        Some("chest,triceps"),
    )?;
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back,biceps"))?;

    // List all exercises
    let exercises = service.list_exercises(None, None)?;
    assert_eq!(exercises.len(), 3);

    // Filter by type
    let resistance_exercises = service.list_exercises(Some(ExerciseType::Resistance), None)?;
    assert_eq!(resistance_exercises.len(), 1);
    assert_eq!(resistance_exercises[0].name, "Bench Press");

    // Filter by muscle
    let leg_exercises = service.list_exercises(None, Some("legs"))?;
    assert_eq!(leg_exercises.len(), 1);
    assert_eq!(leg_exercises[0].name, "Running");

    Ok(())
}

#[test]
fn test_pb_config_interaction() -> Result<()> {
    let mut service = create_test_service()?; // PB notifications default to Some(true) here

    // Check initial state (global enabled)
    assert_eq!(service.check_pb_notification_config()?, true);

    // Disable PB notifications globally
    service.set_pb_notification_enabled(false)?;
    assert_eq!(service.config.notify_pb_enabled, Some(false));
    assert_eq!(service.check_pb_notification_config()?, false);

    // Re-enable PB notifications globally
    service.set_pb_notification_enabled(true)?;
    assert_eq!(service.config.notify_pb_enabled, Some(true));
    assert_eq!(service.check_pb_notification_config()?, true);

    // Test case where config starts as None (simulate first run)
    service.config.notify_pb_enabled = None;
    let result = service.check_pb_notification_config();
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::PbNotificationNotSet => {} // Correct error
        _ => panic!("Expected PbNotificationNotSet error"),
    }

    // Test individual metric flags
    assert!(service.config.notify_pb_weight); // Should be true by default
    service.set_pb_notify_weight(false)?;
    assert!(!service.config.notify_pb_weight);
    service.set_pb_notify_weight(true)?;
    assert!(service.config.notify_pb_weight);

    assert!(service.config.notify_pb_reps);
    service.set_pb_notify_reps(false)?;
    assert!(!service.config.notify_pb_reps);

    assert!(service.config.notify_pb_duration);
    service.set_pb_notify_duration(false)?;
    assert!(!service.config.notify_pb_duration);

    assert!(service.config.notify_pb_distance);
    service.set_pb_notify_distance(false)?;
    assert!(!service.config.notify_pb_distance);

    Ok(())
}

// Test list filtering with aliases
#[test]
fn test_list_filter_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise(
        "Overhead Press",
        ExerciseType::Resistance,
        Some("shoulders"),
    )?;
    service.create_alias("ohp", "Overhead Press")?;
    let today = Utc::now().date_naive();

    service.add_workout(
        "ohp",
        today,
        Some(5),
        Some(5),
        Some(50.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Filter list using alias
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("ohp"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].exercise_name, "Overhead Press");

    // Filter list using canonical name
    let workouts2 = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Overhead Press"),
        ..Default::default()
    })?;
    assert_eq!(workouts2.len(), 1);
    assert_eq!(workouts2[0].exercise_name, "Overhead Press");

    Ok(())
}

#[test]
fn test_add_and_list_workouts_with_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Metric; // Use Metric for easy km verification

    // Create an exercise first
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;

    // Add some workouts
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 2).unwrap();
    service.add_workout(
        "Running",
        date1,
        None,
        None,
        None,
        Some(30),
        Some(5.0), // 5 km
        Some("First run".to_string()),
        None,
        None,
        None,
    )?;

    let date2 = NaiveDate::from_ymd_opt(2015, 6, 3).unwrap();
    service.add_workout(
        "Running",
        date2,
        None,
        None,
        None,
        Some(60),
        Some(10.5), // 10.5 km
        Some("Second run".to_string()),
        None,
        None,
        None,
    )?;

    // List workouts
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Running"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 2);
    // Most recent first
    assert_eq!(workouts[0].timestamp.date_naive(), date2);
    assert_eq!(workouts[0].duration_minutes, Some(60));
    assert_eq!(workouts[0].distance, Some(10.5)); // Check distance stored (km)

    assert_eq!(workouts[1].timestamp.date_naive(), date1);
    assert_eq!(workouts[1].duration_minutes, Some(30));
    assert_eq!(workouts[1].distance, Some(5.0));

    Ok(())
}

#[test]
fn test_add_workout_imperial_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Imperial; // Set units to Imperial

    service.create_exercise("Cycling", ExerciseType::Cardio, None)?;
    let today = Utc::now().date_naive();

    let miles_input = 10.0;
    let expected_km = miles_input * 1.60934;

    service.add_workout(
        "Cycling",
        today,
        None,
        None,
        None,
        Some(45),
        Some(miles_input),
        None,
        None,
        None,
        None,
    )?;

    // List workout and check stored distance (should be km)
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Cycling"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert!(workouts[0].distance.is_some());
    // Compare with tolerance for floating point
    assert!((workouts[0].distance.unwrap() - expected_km).abs() < 0.001);

    Ok(())
}

#[test]
fn test_edit_workout_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Metric;
    service.create_exercise("Walking", ExerciseType::Cardio, None)?;
    let today = Utc::now().date_naive();

    let (workout_id, _) = service.add_workout(
        "Walking",
        today,
        None,
        None,
        None,
        Some(60),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?;

    // Edit distance
    let new_distance = 7.5;
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        Some(new_distance),
        None,
        None,
    )?;

    // Verify
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Walking"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].distance, Some(new_distance));

    // Edit distance with Imperial units input
    service.config.units = Units::Imperial;
    let imperial_input = 2.0; // 2 miles
    let expected_km_edit = imperial_input * 1.60934;
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        Some(imperial_input),
        None,
        None,
    )?;

    // Verify stored km
    let workouts_imperial_edit = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Walking"),
        ..Default::default()
    })?;
    assert_eq!(workouts_imperial_edit.len(), 1);
    assert!(workouts_imperial_edit[0].distance.is_some());
    assert!((workouts_imperial_edit[0].distance.unwrap() - expected_km_edit).abs() < 0.001);

    Ok(())
}

#[test]
fn test_bodyweight_workouts() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.bodyweight = Some(70.0); // Set bodyweight

    // Create a bodyweight exercise
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?;

    // Add workout with additional weight
    service.add_workout(
        "Pull-ups",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        Some(3),
        Some(10),
        Some(5.0), // Additional weight
        None,
        None, // No distance
        None,
        None,
        None,
        Some(70.0), // Pass bodyweight
    )?;

    // Check that weight was calculated correctly
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].weight, Some(75.0)); // 70 + 5

    Ok(())
}

#[test]
fn test_edit_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Edit the exercise
    service.edit_exercise(
        "Bench Press",
        Some("Barbell Bench Press"),
        Some(ExerciseType::Resistance),
        Some(Some("chest,triceps,shoulders")),
    )?;

    // Verify changes
    let exercise = service
        .get_exercise_by_identifier_service("Barbell Bench Press")?
        .unwrap();
    assert_eq!(exercise.name, "Barbell Bench Press");
    assert_eq!(
        exercise.muscles,
        Some("chest,triceps,shoulders".to_string())
    );

    // Try editing non-existent exercise
    let edit_result = service.edit_exercise("NonExistent", Some("WontWork"), None, None);
    assert!(edit_result.is_err());
    assert!(edit_result
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_delete_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Delete it
    let result = service.delete_exercise(&vec!["Bench Press".to_string()])?;
    assert_eq!(result, 1);

    // Verify it's gone
    let exercise = service.get_exercise_by_identifier_service("Bench Press")?;
    assert!(exercise.is_none());

    // Try deleting non-existent exercise
    let delete_result = service.delete_exercise(&vec!["NonExistent".to_string()]);
    assert!(delete_result.is_err());
    assert!(delete_result
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_workout_filters() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Curl", ExerciseType::Resistance, Some("Biceps"))?;

    // Add workouts on different dates
    // Hack: We can't set the timestamp directly, so we'll add with a small delay
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 3).unwrap();
    service.add_workout(
        "Running",
        date1,
        None,
        None,
        None,
        Some(30),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?;
    thread::sleep(StdDuration::from_millis(10)); // Ensure different timestamp

    // Add another workout on same date but later time
    service.add_workout(
        "Curl",
        date1,
        Some(3),
        Some(12),
        Some(15.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Filter by type
    let resistance_workouts = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Resistance),
        ..Default::default()
    })?;
    assert_eq!(resistance_workouts.len(), 1);
    assert_eq!(resistance_workouts[0].exercise_name, "Curl");
    assert_eq!(resistance_workouts[0].reps, Some(12));

    let cardio_workouts = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(cardio_workouts.len(), 1);
    assert_eq!(cardio_workouts[0].exercise_name, "Running");
    assert_eq!(cardio_workouts[0].distance, Some(5.0));

    // Filter by date
    let date1_workouts = service.list_workouts(WorkoutFilters {
        date: Some(date1),
        ..Default::default()
    })?;
    assert_eq!(date1_workouts.len(), 2); // Both workouts were on this date

    Ok(())
}

#[test]
fn test_nth_last_day_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Add workouts on different dates
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 2).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2015, 6, 7).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2015, 6, 9).unwrap(); // Most recent

    // Workout 1 (oldest)
    service.add_workout(
        "Squats",
        date1,
        Some(3),
        Some(10),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    // Workout 2 (middle)
    service.add_workout(
        "Squats",
        date2,
        Some(5),
        Some(5),
        Some(120.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    // Workout 3 (most recent)
    service.add_workout(
        "Squats",
        date3,
        Some(4),
        Some(6),
        Some(125.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Get workouts for the most recent day (n=1)
    let recent_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 1)?;
    assert_eq!(recent_workouts.len(), 1);
    assert_eq!(recent_workouts[0].sets, Some(4));
    assert_eq!(recent_workouts[0].timestamp.date_naive(), date3);

    // Get workouts for the second most recent day (n=2)
    let previous_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 2)?;
    assert_eq!(previous_workouts.len(), 1);
    assert_eq!(previous_workouts[0].sets, Some(5));
    assert_eq!(previous_workouts[0].timestamp.date_naive(), date2);

    // Get workouts for the third most recent day (n=3)
    let oldest_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 3)?;
    assert_eq!(oldest_workouts.len(), 1);
    assert_eq!(oldest_workouts[0].sets, Some(3));
    assert_eq!(oldest_workouts[0].timestamp.date_naive(), date1);

    // Try getting n=4 (should be empty)
    let no_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 4)?;
    assert!(no_workouts.is_empty());

    Ok(())
}

#[test]
fn test_config_operations() -> Result<()> {
    let mut service = create_test_service()?;

    // Test setting bodyweight
    service.set_bodyweight(75.5)?;
    assert_eq!(service.config.bodyweight, Some(75.5));

    // Test getting required bodyweight
    let bw = service.get_required_bodyweight()?;
    assert_eq!(bw, 75.5);

    // Test disabling prompt
    service.disable_bodyweight_prompt()?;
    assert!(!service.config.prompt_for_bodyweight);

    // Test setting streak interval
    assert_eq!(service.config.streak_interval_days, 1);
    service.set_streak_interval(3)?;
    assert_eq!(service.config.streak_interval_days, 3);
    let interval_result = service.set_streak_interval(0); // Test invalid interval
    assert!(interval_result.is_err()); // Should fail

    Ok(())
}

#[test]
fn test_exercise_not_found() -> Result<()> {
    let mut service = create_test_service()?;

    // Try to get non-existent exercise
    let result = service.get_exercise_by_identifier_service("Non-existent");
    assert!(result.is_ok()); // Should return Ok(None)
    assert!(result?.is_none());

    // Try to edit non-existent exercise
    let result = service.edit_exercise("Non-existent", None, None, None);
    assert!(result.is_err());
    match result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::ExerciseNotFound(_)) => (),
        _ => panic!("Expected ExerciseNotFound error"),
    }

    // Try to delete non-existent exercise
    let result = service.delete_exercise(&vec!["Non-existent".to_string()]);
    assert!(result.is_err());
    match result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::ExerciseNotFound(_)) => (),
        _ => panic!("Expected ExerciseNotFound error"),
    }

    // Try to add workout for non-existent exercise without implicit details
    let result = service.add_workout(
        "Non-existent",
        Utc::now().date_naive(),
        Some(1),
        Some(1),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found. Define it first"));

    Ok(())
}

#[test]
fn test_workout_not_found() -> Result<()> {
    let service = create_test_service()?;

    // Try to edit non-existent workout
    let result = service.edit_workout(999, None, None, None, None, None, None, None, None);
    assert!(result.is_err());
    // match result.unwrap_err().downcast_ref::<DbError>() {
    //      Some(DbError::WorkoutNotFound(999)) => (), // Correct error and ID
    //      _ => panic!("Expected WorkoutNotFound error with ID 999"),
    // }

    // Try to delete non-existent workout
    let result = service.delete_workouts(&vec![999]);
    assert!(result.is_err());
    //  match result.unwrap_err().downcast_ref::<DbError>() {
    //     Some(DbError::WorkoutNotFound(999)) => (), // Correct error and ID
    //     _ => panic!("Expected WorkoutNotFound error with ID 999"),
    // }

    Ok(())
}

#[test]
fn test_bodyweight_validation() -> Result<()> {
    let mut service = create_test_service()?;

    // Test invalid bodyweight
    let result = service.set_bodyweight(0.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    let result = service.set_bodyweight(-10.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    Ok(())
}

#[test]
fn test_set_units() -> Result<()> {
    let mut service = create_test_service()?;
    assert_eq!(service.config.units, Units::Metric);

    // Set to Imperial
    service.set_units(Units::Imperial)?;
    assert_eq!(service.config.units, Units::Imperial);

    // Set back to Metric
    service.set_units(Units::Metric)?;
    assert_eq!(service.config.units, Units::Metric);

    Ok(())
}

// Test Workout Volume Calculation (Feature 1)
#[test]
fn test_workout_volume() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();

    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?; // BW with added weight
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Day 1 Workouts
    service.add_workout(
        "Bench Press",
        day1,
        Some(3),
        Some(10),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 3*10*100 = 3000
    service.add_workout(
        "Bench Press",
        day1,
        Some(1),
        Some(8),
        Some(105.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 1*8*105 = 840
    service.add_workout(
        "Pull-ups",
        day1,
        Some(4),
        Some(6),
        Some(10.0),
        None,
        None,
        None,
        None,
        None,
        Some(70.0),
    )?; // Vol = 4*6*(70+10) = 1920
    service.add_workout(
        "Running",
        day1,
        None,
        None,
        None,
        Some(30),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?; // Vol = 0

    // Day 2 Workouts
    service.add_workout(
        "Squats",
        day2,
        Some(5),
        Some(5),
        Some(120.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 5*5*120 = 3000
    service.add_workout(
        "Bench Press",
        day2,
        Some(4),
        Some(6),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 4*6*100 = 2400

    // --- Test Volume Calculation ---

    // Total volume per day/exercise (no filters, default limit)
    let volume_all = service.calculate_daily_volume(VolumeFilters::default())?;
    // Should return rows per exercise per day, most recent day first
    // Expected: (day2, Squats, 3000), (day2, Bench Press, 2400), (day1, Bench Press, 3840), (day1, Pull-ups, 1920), (day1, Running, 0)
    assert_eq!(volume_all.len(), 5);

    // Verify Day 2 data (order within day is by name ASC)
    assert_eq!(volume_all[0].0, day2);
    assert_eq!(volume_all[0].1, "Bench Press"); // BP comes before Squats alphabetically
    assert!((volume_all[0].2 - 2400.0).abs() < 0.01);
    assert_eq!(volume_all[1].0, day2);
    assert_eq!(volume_all[1].1, "Squats");
    assert!((volume_all[1].2 - 3000.0).abs() < 0.01);

    // Verify Day 1 data (order within day is by name ASC)
    assert_eq!(volume_all[2].0, day1);
    assert_eq!(volume_all[2].1, "Bench Press"); // BP
    assert!((volume_all[2].2 - (3000.0 + 840.0)).abs() < 0.01); // 3840
    assert_eq!(volume_all[3].0, day1);
    assert_eq!(volume_all[3].1, "Pull-ups"); // Pull-ups
    assert!((volume_all[3].2 - 1920.0).abs() < 0.01);
    assert_eq!(volume_all[4].0, day1);
    assert_eq!(volume_all[4].1, "Running"); // Running
    assert!((volume_all[4].2 - 0.0).abs() < 0.01);

    // Volume for Day 1 only
    let volume_day1 = service.calculate_daily_volume(VolumeFilters {
        start_date: Some(day1),
        end_date: Some(day1),
        ..Default::default()
    })?;
    assert_eq!(volume_day1.len(), 3); // BP, Pull-ups, Running on day 1
                                      // Could check specific values if needed

    // Volume for "Bench Press" only
    let volume_bp = service.calculate_daily_volume(VolumeFilters {
        exercise_name: Some("Bench Press"),
        ..Default::default()
    })?;
    assert_eq!(volume_bp.len(), 2); // BP on day 1 and day 2
                                    // Day 2 BP
    assert_eq!(volume_bp[0].0, day2);
    assert_eq!(volume_bp[0].1, "Bench Press");
    assert!((volume_bp[0].2 - 2400.0).abs() < 0.01);
    // Day 1 BP
    assert_eq!(volume_bp[1].0, day1);
    assert_eq!(volume_bp[1].1, "Bench Press");
    assert!((volume_bp[1].2 - 3840.0).abs() < 0.01); // Sum of both BP entries

    // Volume for Cardio (should be 0)
    let volume_cardio = service.calculate_daily_volume(VolumeFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(volume_cardio.len(), 1); // Only Running on day 1
    assert_eq!(volume_cardio[0].0, day1);
    assert_eq!(volume_cardio[0].1, "Running");
    assert_eq!(volume_cardio[0].2, 0.0);

    Ok(())
}

// Test Exercise Statistics Calculation
#[test]
fn test_exercise_stats() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 20).unwrap(); // Fri
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 22).unwrap(); // Sun (Gap 1 day -> 2 days total)
    let day3 = NaiveDate::from_ymd_opt(2023, 10, 23).unwrap(); // Mon (Gap 0 days -> 1 day total)
    let day4 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(); // Fri (Gap 3 days -> 4 days total) - Longest Gap 3
    let day5 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap(); // Sat (Gap 0 days -> 1 day total)

    service.create_exercise("Test Stats", ExerciseType::Resistance, None)?;

    // Add workouts
    service.add_workout(
        "Test Stats",
        day1,
        Some(3),
        Some(10),
        Some(50.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // PB: W=50, R=10
    service.add_workout(
        "Test Stats",
        day2,
        Some(3),
        Some(8),
        Some(55.0),
        Some(10),
        None,
        None,
        None,
        None,
        None,
    )?; // PB: W=55, D=10
    service.add_workout(
        "Test Stats",
        day3,
        Some(4),
        Some(6),
        Some(50.0),
        Some(12),
        None,
        None,
        None,
        None,
        None,
    )?; // PB: D=12
    service.add_workout(
        "Test Stats",
        day4,
        Some(2),
        Some(12),
        Some(45.0),
        None,
        Some(5.0),
        None,
        None,
        None,
        None,
    )?; // PB: R=12, Dist=5.0
    service.add_workout(
        "Test Stats",
        day5,
        Some(3),
        Some(10),
        Some(55.0),
        Some(10),
        Some(5.5),
        None,
        None,
        None,
        None,
    )?; // PB: Dist=5.5

    // --- Test with daily streak interval (default) ---
    let stats_daily = service.get_exercise_stats("Test Stats")?;

    assert_eq!(stats_daily.canonical_name, "Test Stats");
    assert_eq!(stats_daily.total_workouts, 5);
    assert_eq!(stats_daily.first_workout_date, Some(day1));
    assert_eq!(stats_daily.last_workout_date, Some(day5));

    // Avg/week: 5 workouts / (8 days / 7 days/week) = 5 / (8/7) = 35/8 = 4.375
    assert!((stats_daily.avg_workouts_per_week.unwrap() - 4.375).abs() < 0.01);

    // Gaps: day2-day1=2 days, day3-day2=1 day, day4-day3=4 days, day5-day4=1 day. Max gap = 4 days. DB stores diff, we want gap between.
    // Longest gap days: (day2-day1)-1 = 1, (day3-day2)-1 = 0, (day4-day3)-1 = 3, (day5-day4)-1 = 0 -> Max = 3
    assert_eq!(stats_daily.longest_gap_days, Some(3));

    // Streaks (daily interval = 1 day):
    // day1 -> day2 (gap 1) NO
    // day2 -> day3 (gap 0) YES (Streak: day2, day3 = 2)
    // day3 -> day4 (gap 3) NO
    // day4 -> day5 (gap 0) YES (Streak: day4, day5 = 2)
    // Longest = 2. Current = 2 (ends on day5, today is > day5) -> Should current be 0? Yes.
    assert_eq!(stats_daily.current_streak, 0); // Assuming test runs after day5
    assert_eq!(stats_daily.longest_streak, 2);
    assert_eq!(stats_daily.streak_interval_days, 1);

    // PBs
    assert_eq!(stats_daily.personal_bests.max_weight, Some(55.0));
    assert_eq!(stats_daily.personal_bests.max_reps, Some(12));
    assert_eq!(stats_daily.personal_bests.max_duration_minutes, Some(12));
    assert_eq!(stats_daily.personal_bests.max_distance_km, Some(5.5));

    // --- Test with 2-day streak interval ---
    service.set_streak_interval(2)?;
    let stats_2day = service.get_exercise_stats("Test Stats")?;

    assert_eq!(stats_2day.streak_interval_days, 2);
    // Streaks (2-day interval):
    // day1 -> day2 (gap 1 <= 2) YES (Streak: day1, day2 = 2)
    // day2 -> day3 (gap 0 <= 2) YES (Streak: day1, day2, day3 = 3)
    // day3 -> day4 (gap 3 > 2) NO
    // day4 -> day5 (gap 0 <= 2) YES (Streak: day4, day5 = 2)
    // Longest = 3. Current = 0 (ends on day5, today > day5+2)
    assert_eq!(stats_2day.current_streak, 0);
    assert_eq!(stats_2day.longest_streak, 3);

    // --- Test Edge Cases ---
    // Test stats for exercise with no workouts
    service.create_exercise("No Workouts", ExerciseType::Cardio, None)?;
    let no_workout_result = service.get_exercise_stats("No Workouts");
    assert!(no_workout_result.is_err());
    match no_workout_result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::NoWorkoutDataFound(_)) => (),
        _ => panic!("Expected NoWorkoutDataFound error"),
    }

    // Test stats for exercise with one workout
    service.create_exercise("One Workout", ExerciseType::Resistance, None)?;
    let day_single = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
    service.add_workout(
        "One Workout",
        day_single,
        Some(1),
        Some(5),
        Some(10.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let one_workout_stats = service.get_exercise_stats("One Workout")?;

    assert_eq!(one_workout_stats.total_workouts, 1);
    assert_eq!(one_workout_stats.first_workout_date, Some(day_single));
    assert_eq!(one_workout_stats.last_workout_date, Some(day_single));
    assert!(one_workout_stats.avg_workouts_per_week.is_none());
    assert!(one_workout_stats.longest_gap_days.is_none());
    assert_eq!(one_workout_stats.current_streak, 0); // Needs >= 1 day gap from today
    assert_eq!(one_workout_stats.longest_streak, 1);
    assert_eq!(one_workout_stats.personal_bests.max_weight, Some(10.0));
    assert_eq!(one_workout_stats.personal_bests.max_reps, Some(5));
    assert!(one_workout_stats
        .personal_bests
        .max_duration_minutes
        .is_none());
    assert!(one_workout_stats.personal_bests.max_distance_km.is_none());

    Ok(())
}

#[test]
fn test_get_latest_bodyweight() -> Result<()> {
    let service = create_test_service()?;

    // Test when empty
    assert!(service.get_latest_bodyweight()?.is_none());

    // Add some entries
    service.add_bodyweight_entry(Utc::now() - Duration::days(2), 70.0)?;
    service.add_bodyweight_entry(Utc::now() - Duration::days(1), 71.0)?; // Latest

    // Get latest
    let latest = service.get_latest_bodyweight()?;
    assert!(latest.is_some());
    assert_eq!(latest.unwrap(), 71.0);

    Ok(())
}

#[test]
fn test_bodyweight_workout_needs_log() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?;

    // Try adding BW workout *before* logging bodyweight
    // This test now relies on the logic in main.rs to fetch the BW.
    // We simulate that check here.
    let latest_bw = service.get_latest_bodyweight()?;
    assert!(latest_bw.is_none()); // Should be none initially

    // If main.rs were running, it would get None and bail.
    // We can simulate the direct call to add_workout with None for bodyweight_to_use
    // which should now trigger the internal error check.
    let result = service.add_workout(
        "Pull-ups",
        Utc::now().date_naive(),
        Some(3),
        Some(5),
        Some(10.0), // Additional weight
        None,
        None,
        None,
        None,
        None,
        None, // Simulate not finding a logged bodyweight
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Bodyweight is required"));

    // Now log bodyweight and try again
    service.add_bodyweight_entry(Utc::now(), 75.0)?;
    let logged_bw = service.get_latest_bodyweight()?.unwrap();

    // Simulate main.rs fetching BW and passing it
    let add_result = service.add_workout(
        "Pull-ups",
        Utc::now().date_naive(),
        Some(3),
        Some(5),
        Some(10.0), // Additional weight
        None,
        None,
        None,
        None,
        None,
        Some(logged_bw), // Pass the fetched BW
    );

    assert!(add_result.is_ok());
    let (id, _) = add_result?;

    // Verify workout weight was calculated correctly
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, id);
    assert_eq!(workouts[0].weight, Some(85.0)); // 75.0 (logged) + 10.0 (additional)

    Ok(())
}

#[test]
fn test_add_list_bodyweight() -> Result<()> {
    let service = create_test_service()?;
    let date1 = Utc::now() - Duration::days(2);
    let date2 = Utc::now() - Duration::days(1);
    let date3 = Utc::now();

    // Add entries
    service.add_bodyweight_entry(date1, 70.5)?;
    service.add_bodyweight_entry(date2, 71.0)?;
    service.add_bodyweight_entry(date3, 70.8)?;

    // List entries (default limit should be high enough)
    let entries = service.list_bodyweights(10)?;
    assert_eq!(entries.len(), 3);

    // Check order (descending by timestamp)
    assert_eq!(entries[0].1, 70.8); // date3
                                    // Tolerate small difference in timestamp comparison
    assert!((entries[0].0 - date3).num_milliseconds().abs() < 100);

    assert_eq!(entries[1].1, 71.0); // date2
    assert!((entries[1].0 - date2).num_milliseconds().abs() < 100);

    assert_eq!(entries[2].1, 70.5); // date1
    assert!((entries[2].0 - date1).num_milliseconds().abs() < 100);

    // Test limit
    let limited_entries = service.list_bodyweights(1)?;
    assert_eq!(limited_entries.len(), 1);
    assert_eq!(limited_entries[0].1, 70.8); // Should be the latest one

    Ok(())
}

#[test]
fn test_target_bodyweight_config() -> Result<()> {
    let mut service = create_test_service()?;

    // Initially None
    assert!(service.get_target_bodyweight().is_none());

    // Set a target
    service.set_target_bodyweight(Some(78.5))?;
    assert_eq!(service.config.target_bodyweight, Some(78.5));
    assert_eq!(service.get_target_bodyweight(), Some(78.5));

    // Set another target
    service.set_target_bodyweight(Some(77.0))?;
    assert_eq!(service.config.target_bodyweight, Some(77.0));
    assert_eq!(service.get_target_bodyweight(), Some(77.0));

    // Clear the target
    service.set_target_bodyweight(None)?;
    assert!(service.config.target_bodyweight.is_none());
    assert!(service.get_target_bodyweight().is_none());

    // Test invalid input
    let result_neg = service.set_target_bodyweight(Some(-10.0));
    assert!(result_neg.is_err());
    match result_neg.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    Ok(())
}

//task-athlete-tui/src/app/actions.rs
// task-athlete-tui/src/app/actions.rs
use super::modals::{handle_log_bodyweight_modal_input, handle_set_target_weight_modal_input}; // Use specific modal handlers
use super::navigation::{
    bw_table_next, bw_table_previous, log_list_next, log_list_previous, log_table_next,
    log_table_previous,
};
use super::state::{ActiveModal, ActiveTab, App, BodyweightFocus, LogBodyweightField, LogFocus, SetTargetWeightField};
use super::data::log_change_date;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// Make handle_key_event a method on App
impl App {
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle based on active modal first
        if self.active_modal != ActiveModal::None {
            return self.handle_modal_input(key); // Call modal handler method
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.active_modal = ActiveModal::Help,
            KeyCode::F(1) => self.active_tab = ActiveTab::Log,
            KeyCode::F(2) => self.active_tab = ActiveTab::History,
            KeyCode::F(3) => self.active_tab = ActiveTab::Graphs,
            KeyCode::F(4) => self.active_tab = ActiveTab::Bodyweight,
            _ => {
                // Delegate to tab-specific handler
                match self.active_tab {
                    ActiveTab::Log => self.handle_log_input(key)?,
                    ActiveTab::History => self.handle_history_input(key)?,
                    ActiveTab::Graphs => self.handle_graphs_input(key)?,
                    ActiveTab::Bodyweight => self.handle_bodyweight_input(key)?,
                }
            }
        }
        // Data refresh is now handled by the main loop *after* input handling
        // self.refresh_data_for_active_tab(); // Remove refresh call from here
        Ok(())
    }

    fn handle_modal_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.active_modal {
            ActiveModal::Help => {
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('?')
                    || key.code == KeyCode::Enter
                {
                    self.active_modal = ActiveModal::None;
                }
            }
            ActiveModal::LogBodyweight { .. } => handle_log_bodyweight_modal_input(self, key)?, // Pass self
            ActiveModal::SetTargetWeight { .. } => handle_set_target_weight_modal_input(self, key)?, // Pass self
            _ => {
                if key.code == KeyCode::Esc {
                    self.active_modal = ActiveModal::None;
                }
            }
        }
        Ok(())
    }

    fn handle_log_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.log_focus {
            LogFocus::ExerciseList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_list_previous(self),
                KeyCode::Char('j') | KeyCode::Down => log_list_next(self),
                KeyCode::Tab => self.log_focus = LogFocus::SetList,
                KeyCode::Char('a') => { /* TODO */ }
                KeyCode::Char('g') => { /* TODO */ }
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                _ => {}
            },
            LogFocus::SetList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_table_previous(self),
                KeyCode::Char('j') | KeyCode::Down => log_table_next(self),
                KeyCode::Tab => self.log_focus = LogFocus::ExerciseList,
                KeyCode::Char('e') | KeyCode::Enter => { /* TODO */ }
                KeyCode::Char('d') | KeyCode::Delete => { /* TODO */ }
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_history_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO
        Ok(())
    }

    fn handle_graphs_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO
        Ok(())
    }

    fn handle_bodyweight_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('l') => {
                self.active_modal = ActiveModal::LogBodyweight {
                    weight_input: String::new(),
                    date_input: "today".to_string(),
                    focused_field: LogBodyweightField::Weight,
                    error_message: None,
                };
            }
            KeyCode::Char('t') => {
                self.active_modal = ActiveModal::SetTargetWeight {
                    weight_input: self
                        .bw_target
                        .map_or(String::new(), |w| format!("{:.1}", w)),
                    focused_field: SetTargetWeightField::Weight,
                    error_message: None,
                };
            }
            KeyCode::Char('r') => self.bw_cycle_graph_range(), // Keep cycle logic here for now
            _ => match self.bw_focus {
                BodyweightFocus::History => match key.code {
                    KeyCode::Char('k') | KeyCode::Up => bw_table_previous(self),
                    KeyCode::Char('j') | KeyCode::Down => bw_table_next(self),
                    KeyCode::Tab => self.bw_focus = BodyweightFocus::Actions,
                    _ => {}
                },
                BodyweightFocus::Actions => match key.code {
                    KeyCode::Tab => self.bw_focus = BodyweightFocus::History,
                    _ => {}
                },
                BodyweightFocus::Graph => match key.code {
                    KeyCode::Tab => self.bw_focus = BodyweightFocus::Actions,
                    _ => {}
                },
            },
        }
        Ok(())
    }

    // Keep cycle graph range here as it modifies App state directly
    fn bw_cycle_graph_range(&mut self) {
        self.bw_graph_range_months = match self.bw_graph_range_months {
            1 => 3,
            3 => 6,
            6 => 12,
            12 => 0,
            _ => 1,
        };
        self.update_bw_graph_data(); // Call data update method
    }
}

//task-athlete-tui/src/app/data.rs
// task-athlete-tui/src/app/data.rs
use super::state::App;
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use task_athlete_lib::{DbError, Workout, WorkoutFilters};

// Make refresh logic methods on App
impl App {
    // Fetch or update data based on the active tab
    pub fn refresh_data_for_active_tab(&mut self) {
        self.clear_expired_error(); // Check and clear status bar error first

        match self.active_tab {
            super::state::ActiveTab::Log => self.refresh_log_data(),
            super::state::ActiveTab::History => {} // TODO
            super::state::ActiveTab::Graphs => {}  // TODO
            super::state::ActiveTab::Bodyweight => self.refresh_bodyweight_data(),
        }
    }

    // --- Log Tab Data ---
    pub(crate) fn refresh_log_data(&mut self) { // Make crate-public if needed by other app modules
        let filters = WorkoutFilters {
            date: Some(self.log_viewed_date),
            ..Default::default()
        };
        match self.service.list_workouts(filters) {
            Ok(workouts) => {
                let mut unique_names = workouts
                    .iter()
                    .map(|w| w.exercise_name.clone())
                    .collect::<Vec<_>>();
                unique_names.sort_unstable();
                unique_names.dedup();
                self.log_exercises_today = unique_names;

                if self.log_exercise_list_state.selected().unwrap_or(0)
                    >= self.log_exercises_today.len()
                {
                    self.log_exercise_list_state
                        .select(if self.log_exercises_today.is_empty() {
                            None
                        } else {
                            Some(self.log_exercises_today.len().saturating_sub(1))
                        });
                }

                self.update_log_sets_for_selected_exercise(&workouts);
            }
            Err(e) => {
                if e.downcast_ref::<DbError>()
                    .map_or(false, |dbe| matches!(dbe, DbError::ExerciseNotFound(_)))
                {
                    self.log_exercises_today.clear();
                    self.log_sets_for_selected_exercise.clear();
                } else {
                    self.set_error(format!("Error fetching log data: {}", e))
                }
            }
        }
    }

    // Make crate-public
    pub(crate) fn update_log_sets_for_selected_exercise(&mut self, all_workouts_for_date: &[Workout]) {
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) = self.log_exercises_today.get(selected_index) {
                self.log_sets_for_selected_exercise = all_workouts_for_date
                    .iter()
                    .filter(|w| &w.exercise_name == selected_exercise_name)
                    .cloned()
                    .collect();

                if self.log_set_table_state.selected().unwrap_or(0)
                    >= self.log_sets_for_selected_exercise.len()
                {
                    self.log_set_table_state.select(
                        if self.log_sets_for_selected_exercise.is_empty() {
                            None
                        } else {
                            Some(self.log_sets_for_selected_exercise.len() - 1)
                        },
                    );
                } else if self.log_set_table_state.selected().is_none()
                    && !self.log_sets_for_selected_exercise.is_empty()
                {
                    self.log_set_table_state.select(Some(0));
                }
            } else {
                self.log_sets_for_selected_exercise.clear();
                self.log_set_table_state.select(None);
            }
        } else {
            self.log_sets_for_selected_exercise.clear();
            self.log_set_table_state.select(None);
        }
    }

    // --- Bodyweight Tab Data ---
     pub(crate) fn refresh_bodyweight_data(&mut self) {
        match self.service.list_bodyweights(1000) {
            Ok(entries) => {
                self.bw_history = entries;

                if self.bw_history_state.selected().unwrap_or(0) >= self.bw_history.len() {
                    self.bw_history_state.select(if self.bw_history.is_empty() {
                        None
                    } else {
                        Some(self.bw_history.len() - 1)
                    });
                } else if self.bw_history_state.selected().is_none() && !self.bw_history.is_empty()
                {
                    self.bw_history_state.select(Some(0));
                }

                self.bw_latest = self.bw_history.first().map(|(_, _, w)| *w);
                self.bw_target = self.service.get_target_bodyweight(); // Refresh target

                self.update_bw_graph_data();
            }
            Err(e) => self.set_error(format!("Error fetching bodyweight data: {}", e)),
        }
    }

    // Make crate-public
    pub(crate) fn update_bw_graph_data(&mut self) {
        if self.bw_history.is_empty() {
            self.bw_graph_data.clear();
            self.bw_graph_x_bounds = [0.0, 1.0];
            self.bw_graph_y_bounds = [0.0, 1.0];
            return;
        }

        let now_naive = Utc::now().date_naive();
        let start_date_filter = if self.bw_graph_range_months > 0 {
            let mut year = now_naive.year();
            let mut month = now_naive.month();
            let day = now_naive.day();
            let months_ago = self.bw_graph_range_months;
            let total_months = (year * 12 + month as i32 - 1) - months_ago as i32;
            year = total_months / 12;
            month = (total_months % 12 + 1) as u32;
            let last_day_of_target_month = NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap();
            NaiveDate::from_ymd_opt(year, month, day.min(last_day_of_target_month.day()))
                .unwrap_or(last_day_of_target_month)
        } else {
            self.bw_history.last().map(|(_, d, _)| *d).unwrap_or(now_naive)
        };

        let filtered_data: Vec<_> = self.bw_history.iter()
            .filter(|(_, date, _)| *date >= start_date_filter)
            .rev()
            .collect();

        if filtered_data.is_empty() {
            self.bw_graph_data.clear();
            return;
        }

        let first_day_epoch = filtered_data.first().unwrap().1.num_days_from_ce();
        self.bw_graph_data = filtered_data.iter()
            .map(|(_, date, weight)| {
                let days_since_first = (date.num_days_from_ce() - first_day_epoch) as f64;
                (days_since_first, *weight)
            })
            .collect();

        let first_ts = self.bw_graph_data.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last_ts = self.bw_graph_data.last().map(|(x, _)| *x).unwrap_or(first_ts + 1.0);
        self.bw_graph_x_bounds = [first_ts, last_ts];

        let min_weight = self.bw_graph_data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let max_weight = self.bw_graph_data.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);
        let y_min = self.bw_target.map_or(min_weight, |t| t.min(min_weight));
        let y_max = self.bw_target.map_or(max_weight, |t| t.max(max_weight));
        let y_padding = ((y_max - y_min) * 0.1).max(1.0);
        self.bw_graph_y_bounds = [(y_min - y_padding).max(0.0), y_max + y_padding];
    }
}


// Function needs to be associated with App or take &mut App
// Move it outside the impl block but keep it in this file, taking &mut App
pub fn log_change_date(app: &mut App, days: i64) {
     if let Some(new_date) = app.log_viewed_date.checked_add_signed(Duration::days(days)) {
         app.log_viewed_date = new_date;
         app.log_exercise_list_state.select(if app.log_exercises_today.is_empty() { None } else { Some(0) });
         app.log_set_table_state.select(if app.log_sets_for_selected_exercise.is_empty() { None } else { Some(0) });
         // Data will be refreshed by the main loop
     }
 }

//task-athlete-tui/src/app/mod.rs
// task-athlete-tui/src/app/mod.rs
use thiserror::Error;

// Declare the modules within the app directory
pub mod actions;
pub mod data;
pub mod modals;
pub mod navigation;
pub mod state;

// Re-export the main App struct and other necessary types for convenience
pub use state::{ActiveModal, ActiveTab, App, BodyweightFocus, LogFocus}; // Add other enums if needed

// Define App-specific errors here
#[derive(Error, Debug, Clone)] // Added Clone
pub enum AppInputError {
    #[error("Invalid date format: {0}. Use YYYY-MM-DD or shortcuts.")]
    InvalidDate(String),
    #[error("Invalid number format: {0}")]
    InvalidNumber(String),
    #[error("Input field cannot be empty.")]
    InputEmpty,
    #[error("Field requires a selection.")]
    SelectionRequired,
    #[error("Database error: {0}")] // Generic way to show DB errors in modals
    DbError(String),
}

//task-athlete-tui/src/app/modals.rs
// task-athlete-tui/src/app/modals.rs
use super::state::{ActiveModal, App, LogBodyweightField, SetTargetWeightField};
use super::AppInputError;
use anyhow::Result;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use task_athlete_lib::DbError;

// --- Parsing Helpers (moved here) ---

fn parse_modal_date(date_str: &str) -> Result<NaiveDate, AppInputError> {
    let trimmed = date_str.trim().to_lowercase();
    match trimmed.as_str() {
        "today" => Ok(Utc::now().date_naive()),
        "yesterday" => Ok(Utc::now().date_naive() - Duration::days(1)),
        _ => NaiveDate::parse_from_str(&trimmed, "%Y-%m-%d")
            .map_err(|_| AppInputError::InvalidDate(date_str.to_string())),
    }
}

fn parse_modal_weight(weight_str: &str) -> Result<f64, AppInputError> {
    let trimmed = weight_str.trim();
    if trimmed.is_empty() {
        return Err(AppInputError::InputEmpty);
    }
    trimmed
        .parse::<f64>()
        .map_err(|e| AppInputError::InvalidNumber(e.to_string()))
        .and_then(|w| {
            if w > 0.0 {
                Ok(w)
            } else {
                Err(AppInputError::InvalidNumber(
                    "Weight must be positive".to_string(),
                ))
            }
        })
}

// --- Submission Logic ---

fn submit_log_bodyweight(
    app: &mut App, // Pass App mutably
    weight_input: &str,
    date_input: &str,
) -> Result<(), AppInputError> {
    let weight = parse_modal_weight(weight_input)?;
    let date = parse_modal_date(date_input)?;

    let timestamp = date
        .and_hms_opt(12, 0, 0)
        .and_then(|ndt| Utc.from_local_datetime(&ndt).single())
        .ok_or_else(|| AppInputError::InvalidDate("Internal date conversion error".into()))?;

    match app.service.add_bodyweight_entry(timestamp, weight) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(db_err) = e.downcast_ref::<DbError>() {
                if let DbError::BodyweightEntryExists(_) = db_err {
                    return Err(AppInputError::InvalidDate(
                        "Entry already exists for this date".to_string(),
                    ));
                }
                // Return specific DB error message if possible
                return Err(AppInputError::DbError(db_err.to_string()));
            }
            // Generic error for other DB issues
            Err(AppInputError::DbError(format!("DB Error: {}", e)))
        }
    }
}

fn submit_set_target_weight(app: &mut App, weight_input: &str) -> Result<(), AppInputError> {
    let weight = parse_modal_weight(weight_input)?;
    match app.service.set_target_bodyweight(Some(weight)) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error setting target: {}", // ConfigError usually doesn't need DbError type
            e
        ))),
    }
}

fn submit_clear_target_weight(app: &mut App) -> Result<(), AppInputError> {
    match app.service.set_target_bodyweight(None) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error clearing target: {}",
            e
        ))),
    }
}

// --- Input Handling ---

pub fn handle_log_bodyweight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut date_to_submit = String::new();
    let mut should_submit = false;
    let mut focus_after_input = LogBodyweightField::Weight; // Default

    if let ActiveModal::LogBodyweight {
        ref mut weight_input,
        ref mut date_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;
        focus_after_input = *focused_field; // Store current focus

        match focused_field {
            LogBodyweightField::Weight => match key.code {
                KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                KeyCode::Backspace => {
                    weight_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = LogBodyweightField::Date
                }
                KeyCode::Up => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Esc => {
                    // Handle Esc directly here to avoid further processing
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Date => match key.code {
                KeyCode::Char(c) => date_input.push(c),
                KeyCode::Backspace => {
                    date_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = LogBodyweightField::Confirm
                }
                KeyCode::Up => *focused_field = LogBodyweightField::Weight,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Confirm => match key.code {
                KeyCode::Enter => {
                    // Prepare to submit *after* this block releases the borrow
                    should_submit = true;
                    weight_to_submit = weight_input.clone();
                    date_to_submit = date_input.clone();
                }
                KeyCode::Left | KeyCode::Backspace => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Up => *focused_field = LogBodyweightField::Date,
                KeyCode::Down | KeyCode::Tab => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Cancel => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                KeyCode::Right => *focused_field = LogBodyweightField::Confirm,
                KeyCode::Up => *focused_field = LogBodyweightField::Date,
                KeyCode::Down | KeyCode::Tab => *focused_field = LogBodyweightField::Weight,
                _ => {}
            },
        }
    } // Mutable borrow of app.active_modal ends here

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        let submit_result = submit_log_bodyweight(app, &weight_to_submit, &date_to_submit);

        // Handle result: Re-borrow ONLY if necessary to set error
        if submit_result.is_ok() {
            app.active_modal = ActiveModal::None; // Submission successful, close modal
                                                  // Refresh handled by main loop
        } else {
            // Submission failed, need to put error back into modal state
            if let ActiveModal::LogBodyweight {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submit_result.unwrap_err().to_string());
                // Keep the modal open by not setting it to None
            }
            // If modal somehow changed state between submit check and here, error is lost, which is unlikely
        }
    }

    Ok(())
}

pub fn handle_set_target_weight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut submit_action: Option<fn(&mut App, &str) -> Result<(), AppInputError>> = None; // For Set
    let mut clear_action: Option<fn(&mut App) -> Result<(), AppInputError>> = None; // For Clear
    let mut focus_after_input = SetTargetWeightField::Weight; // Default

    if let ActiveModal::SetTargetWeight {
        ref mut weight_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        *error_message = None; // Clear error on any input
        focus_after_input = *focused_field;

        match focused_field {
            SetTargetWeightField::Weight => match key.code {
                KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                KeyCode::Backspace => {
                    weight_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = SetTargetWeightField::Set
                }
                KeyCode::Up => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Set => match key.code {
                KeyCode::Enter => {
                    // Prepare to submit *after* this block
                    weight_to_submit = weight_input.clone();
                    submit_action = Some(submit_set_target_weight);
                }
                KeyCode::Right | KeyCode::Tab => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Up => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Down => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Clear => match key.code {
                KeyCode::Enter => {
                    // Prepare to clear *after* this block
                    clear_action = Some(submit_clear_target_weight);
                }
                KeyCode::Left => *focused_field = SetTargetWeightField::Set,
                KeyCode::Right | KeyCode::Tab => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Up => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Down => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Cancel => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                KeyCode::Left => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Tab => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Up => *focused_field = SetTargetWeightField::Clear,
                _ => {}
            },
        }
    } // Mutable borrow of app.active_modal ends here

    // --- Submission/Clear Logic ---
    let mut submit_result: Result<(), AppInputError> = Ok(()); // Default to Ok

    if let Some(action) = submit_action {
        submit_result = action(app, &weight_to_submit);
    } else if let Some(action) = clear_action {
        submit_result = action(app);
    }

    // Only process result if an action was attempted
    if submit_action.is_some() || clear_action.is_some() {
        if submit_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Refresh handled by main loop
        } else {
            // Re-borrow ONLY if necessary to set error
            if let ActiveModal::SetTargetWeight {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submit_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}

//task-athlete-tui/src/app/navigation.rs
// task-athlete-tui/src/app/navigation.rs
use super::state::App;
use task_athlete_lib::WorkoutFilters; // Keep lib imports

// --- Log Tab Navigation ---

// Need to take &mut App now
pub fn log_list_next(app: &mut App) {
    let current_selection = app.log_exercise_list_state.selected();
    let list_len = app.log_exercises_today.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.log_exercise_list_state.select(Some(i));
    // Refresh sets based on new selection (needs access to service or pre-fetched data)
    let workouts_for_date = app.service.list_workouts(WorkoutFilters {
        date: Some(app.log_viewed_date),
        ..Default::default()
    }).unwrap_or_default(); // Handle error appropriately if needed
    app.update_log_sets_for_selected_exercise(&workouts_for_date); // Use the method from data.rs
}

pub fn log_list_previous(app: &mut App) {
    let current_selection = app.log_exercise_list_state.selected();
    let list_len = app.log_exercises_today.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.log_exercise_list_state.select(Some(i));
    let workouts_for_date = app.service.list_workouts(WorkoutFilters {
        date: Some(app.log_viewed_date),
        ..Default::default()
    }).unwrap_or_default();
    app.update_log_sets_for_selected_exercise(&workouts_for_date);
}

pub fn log_table_next(app: &mut App) {
    let current_selection = app.log_set_table_state.selected();
    let list_len = app.log_sets_for_selected_exercise.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.log_set_table_state.select(Some(i));
}

pub fn log_table_previous(app: &mut App) {
    let current_selection = app.log_set_table_state.selected();
    let list_len = app.log_sets_for_selected_exercise.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.log_set_table_state.select(Some(i));
}

// --- Bodyweight Tab Navigation ---

pub fn bw_table_next(app: &mut App) {
    let current_selection = app.bw_history_state.selected();
    let list_len = app.bw_history.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.bw_history_state.select(Some(i));
}

pub fn bw_table_previous(app: &mut App) {
    let current_selection = app.bw_history_state.selected();
    let list_len = app.bw_history.len();
    if list_len == 0 { return; }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.bw_history_state.select(Some(i));
}

//task-athlete-tui/src/app/state.rs
// task-athlete-tui/src/app/state.rs
use crate::app::AppInputError; // Use error from parent mod
use ratatui::widgets::{ListState, TableState};
use std::time::Instant;
use task_athlete_lib::{AppService, Workout}; // Keep lib imports

// Represents the active UI tab
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveTab {
    Log,
    History,
    Graphs,
    Bodyweight,
}

// Represents which pane has focus in a multi-pane tab
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFocus {
    ExerciseList,
    SetList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyweightFocus {
    Graph,
    Actions,
    History,
}

// Fields within the Log Bodyweight modal
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogBodyweightField {
    Weight,
    Date,
    Confirm,
    Cancel,
}

// Fields within the Set Target Weight modal
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetTargetWeightField {
    Weight,
    Set,
    Clear,
    Cancel,
}

// Represents the state of active modals
#[derive(Clone, Debug, PartialEq)]
pub enum ActiveModal {
    None,
    Help,
    LogBodyweight {
        weight_input: String,
        date_input: String,
        focused_field: LogBodyweightField,
        error_message: Option<String>,
    },
    SetTargetWeight {
        weight_input: String,
        focused_field: SetTargetWeightField,
        error_message: Option<String>,
    },
    // Add more here
}

// Holds the application state
pub struct App {
    pub service: AppService,
    pub active_tab: ActiveTab,
    pub should_quit: bool,
    pub active_modal: ActiveModal,
    pub last_error: Option<String>, // For status bar errors
    pub error_clear_time: Option<Instant>,

    // === Log Tab State ===
    pub log_focus: LogFocus,
    pub log_viewed_date: chrono::NaiveDate,
    pub log_exercises_today: Vec<String>,
    pub log_exercise_list_state: ListState,
    pub log_sets_for_selected_exercise: Vec<Workout>,
    pub log_set_table_state: TableState,

    // === History Tab State ===
    // TODO

    // === Graph Tab State ===
    // TODO

    // === Bodyweight Tab State ===
    pub bw_focus: BodyweightFocus,
    pub bw_history: Vec<(i64, chrono::NaiveDate, f64)>,
    pub bw_history_state: TableState,
    pub bw_target: Option<f64>,
    pub bw_latest: Option<f64>,
    pub bw_graph_data: Vec<(f64, f64)>,
    pub bw_graph_x_bounds: [f64; 2],
    pub bw_graph_y_bounds: [f64; 2],
    pub bw_graph_range_months: u32,
}

impl App {
    pub fn new(service: AppService) -> Self {
        let today = chrono::Utc::now().date_naive();
        let mut app = App {
            active_tab: ActiveTab::Log,
            should_quit: false,
            active_modal: ActiveModal::None,
            log_focus: LogFocus::ExerciseList,
            log_viewed_date: today,
            log_exercises_today: Vec::new(),
            log_exercise_list_state: ListState::default(),
            log_sets_for_selected_exercise: Vec::new(),
            log_set_table_state: TableState::default(),
            bw_focus: BodyweightFocus::History,
            bw_history: Vec::new(),
            bw_history_state: TableState::default(),
            bw_target: service.get_target_bodyweight(),
            bw_latest: None,
            bw_graph_data: Vec::new(),
            bw_graph_x_bounds: [0.0, 1.0],
            bw_graph_y_bounds: [0.0, 1.0],
            bw_graph_range_months: 3,
            last_error: None,
            error_clear_time: None,
            service,
        };
        app.log_exercise_list_state.select(Some(0));
        app.log_set_table_state.select(Some(0));
        app.bw_history_state.select(Some(0));
        // Initial data load is now called explicitly in main loop or where needed
        // app.refresh_data_for_active_tab(); // Remove initial call here
        app
    }

    // Method to set status bar errors
    pub fn set_error(&mut self, msg: String) {
        self.last_error = Some(msg);
        self.error_clear_time =
            Some(Instant::now() + chrono::Duration::seconds(5).to_std().unwrap());
    }

    // Method to clear expired error messages (called in refresh_data_for_active_tab)
    pub(crate) fn clear_expired_error(&mut self) {
        if let Some(clear_time) = self.error_clear_time {
            if Instant::now() >= clear_time {
                self.last_error = None;
                self.error_clear_time = None;
            }
        }
    }
}

//task-athlete-tui/src/main.rs
// task-athlete-tui/src/main.rs
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, time::Duration};
use task_athlete_lib::AppService;

// Declare modules
mod app;
mod ui;

// Use items from modules
use crate::app::App; // Get App struct from app module

fn main() -> Result<()> {
    // Initialize the library service
    let app_service = AppService::initialize().expect("Failed to initialize AppService");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(app_service);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err); // Print errors to stderr
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Ensure data is fresh before drawing (moved inside loop)
        app.refresh_data_for_active_tab(); // Refresh data *before* drawing

        terminal.draw(|f| ui::render_ui(f, app))?;

        // Poll for events with a timeout (e.g., 250ms)
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events
                if key.kind == KeyEventKind::Press {
                    // Pass key event to the app's input handler
                    // handle_key_event is now a method on App
                    app.handle_key_event(key)?;
                }
            }
            // TODO: Handle other events like resize if needed
            // if let Event::Resize(width, height) = event::read()? {
            //     // Handle resize
            // }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

//task-athlete-tui/src/ui/bodyweight_tab.rs
// task-athlete-tui/src/ui/bodyweight_tab.rs
use crate::app::{state::BodyweightFocus, App}; // Use App from crate::app
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, LegendPosition, Paragraph, Row,
        Table, Wrap,
    },
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_bodyweight_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_bodyweight_graph(f, app, chunks[0]);
    render_bodyweight_bottom(f, app, chunks[1]);
}

pub fn render_bodyweight_graph(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let target_data;
    let mut datasets = vec![];

    let data_points: Vec<(f64, f64)> = app
        .bw_graph_data
        .iter()
        .map(|(x, y)| {
            let display_weight = match app.service.config.units {
                Units::Metric => *y,
                Units::Imperial => *y * 2.20462,
            };
            (*x, display_weight)
        })
        .collect();

    datasets.push(
        Dataset::default()
            .name("Bodyweight")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points),
    );

    if let Some(target_raw) = app.bw_target {
        let target_display = match app.service.config.units {
            Units::Metric => target_raw,
            Units::Imperial => target_raw * 2.20462,
        };
        if app.bw_graph_x_bounds[0] <= app.bw_graph_x_bounds[1] {
            target_data = vec![
                (app.bw_graph_x_bounds[0], target_display),
                (app.bw_graph_x_bounds[1], target_display),
            ];
            datasets.push(
                Dataset::default()
                    .name("Target")
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::ITALIC),
                    )
                    .data(&target_data),
            );
        }
    }

    let display_y_bounds = match app.service.config.units {
        Units::Metric => app.bw_graph_y_bounds,
        Units::Imperial => [
            app.bw_graph_y_bounds[0] * 2.20462,
            app.bw_graph_y_bounds[1] * 2.20462,
        ],
    };

    let range_label = match app.bw_graph_range_months {
        1 => "1M",
        3 => "3M",
        6 => "6M",
        12 => "1Y",
        _ => "All",
    };
    let chart_title = format!("Bodyweight Trend ({})", range_label);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(chart_title)
                .border_style(if app.bw_focus == BodyweightFocus::Graph {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .x_axis(
            Axis::default()
                .title("Date".italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(app.bw_graph_x_bounds)
                .labels(vec![]),
        )
        .y_axis(
            Axis::default()
                .title(format!("Weight ({})", weight_unit).italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(display_y_bounds)
                .labels({
                    let min_label = display_y_bounds[0].ceil() as i32;
                    let max_label = display_y_bounds[1].floor() as i32;
                    let range = (max_label - min_label).max(1);
                    let step = (range / 5).max(1);
                    (min_label..=max_label)
                        .step_by(step as usize)
                        .map(|w| Span::from(format!("{:.0}", w)))
                        .collect()
                }),
        )
        .legend_position(Some(LegendPosition::TopLeft));

    f.render_widget(chart, area);
}

fn render_bodyweight_bottom(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_bodyweight_status(f, app, chunks[0]);
    render_bodyweight_history(f, app, chunks[1]);
}

fn render_bodyweight_status(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let (latest_weight_str, latest_date_str) = match app.bw_history.first() {
        Some((_, date, w)) => {
            let display_w = match app.service.config.units {
                Units::Metric => *w,
                Units::Imperial => *w * 2.20462,
            };
            (
                format!("{:.1} {}", display_w, weight_unit),
                format!("(on {})", date.format("%Y-%m-%d")),
            )
        }
        None => ("N/A".to_string(), "".to_string()),
    };
    let target_weight_str = match app.bw_target {
        Some(w) => {
            let display_w = match app.service.config.units {
                Units::Metric => w,
                Units::Imperial => w * 2.20462,
            };
            format!("{:.1} {}", display_w, weight_unit)
        }
        None => "Not Set".to_string(),
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Latest: ", Style::default().bold()),
            Span::raw(latest_weight_str),
            Span::styled(
                format!(" {}", latest_date_str),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().bold()),
            Span::raw(target_weight_str),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " [L]og New ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [T]arget Weight ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [R]ange Cycle ",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status & Actions")
                .border_style(if app.bw_focus == BodyweightFocus::Actions {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_bodyweight_history(f: &mut Frame, app: &mut App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let table_block = Block::default()
        .borders(Borders::ALL)
        .title("History")
        .border_style(if app.bw_focus == BodyweightFocus::History {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let weight_cell_header = format!("Weight ({})", weight_unit);
    let header_cells = ["Date", weight_cell_header.as_str()]
        .into_iter()
        .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.bw_history.iter().map(|(_, date, weight_kg)| {
        let display_weight = match app.service.config.units {
            Units::Metric => *weight_kg,
            Units::Imperial => *weight_kg * 2.20462,
        };
        Row::new(vec![
            Cell::from(date.format("%Y-%m-%d").to_string()),
            Cell::from(format!("{:.1}", display_weight)),
        ])
    });

    let widths = [Constraint::Length(12), Constraint::Min(10)];
    let table = Table::new(rows, widths)
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.bw_history_state);
}

//task-athlete-tui/src/ui/layout.rs
// task-athlete-tui/src/ui/layout.rs
use crate::{
    app::{ActiveTab, App}, // Use App from crate::app
    ui::{ // Use sibling UI modules
        bodyweight_tab::render_bodyweight_tab,
        log_tab::render_log_tab,
        modals::render_modal,
        placeholders::render_placeholder,
        status_bar::render_status_bar,
        tabs::render_tabs,
    },
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

// Main UI rendering function moved here
pub fn render_ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Create main layout: Tabs on top, content below, status bar at bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status Bar
        ])
        .split(size);

    render_tabs(f, app, main_chunks[0]);
    render_main_content(f, app, main_chunks[1]);
    render_status_bar(f, app, main_chunks[2]);

    // Render modal last if active
    if app.active_modal != crate::app::state::ActiveModal::None {
        render_modal(f, app);
    }
}

// Render the content area based on the active tab
fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let content_block = ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::NONE);
    f.render_widget(content_block, area);
    let content_area = area.inner(&ratatui::layout::Margin { vertical: 0, horizontal: 0 });

    match app.active_tab {
        ActiveTab::Log => render_log_tab(f, app, content_area),
        ActiveTab::History => render_placeholder(f, "History Tab", content_area),
        ActiveTab::Graphs => render_placeholder(f, "Graphs Tab", content_area),
        ActiveTab::Bodyweight => render_bodyweight_tab(f, app, content_area),
    }
}

/// Helper function to create a centered rectangle for modals
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let percent_x = percent_x.min(100);
    let percent_y = percent_y.min(100);
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

//task-athlete-tui/src/ui/log_tab.rs
// task-athlete-tui/src/ui/log_tab.rs
use crate::app::{state::LogFocus, App}; // Use App from crate::app
use chrono::{Duration, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_log_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let today_str = Utc::now().date_naive();
    let date_header_str = if app.log_viewed_date == today_str {
        format!("--- Today ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else if app.log_viewed_date == today_str - Duration::days(1) {
        format!("--- Yesterday ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else {
        format!("--- {} ---", app.log_viewed_date.format("%Y-%m-%d"))
    };

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let date_header = Paragraph::new(date_header_str)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(date_header, outer_chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer_chunks[1]);

    render_log_exercise_list(f, app, chunks[0]);
    render_log_set_list(f, app, chunks[1]);
}

fn render_log_exercise_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .log_exercises_today
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title("Exercises Logged")
        .border_style(if app.log_focus == LogFocus::ExerciseList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.log_exercise_list_state);
}

fn render_log_set_list(f: &mut Frame, app: &mut App, area: Rect) {
     let selected_exercise_name = app
        .log_exercise_list_state
        .selected()
        .and_then(|i| app.log_exercises_today.get(i));

     let title = match selected_exercise_name {
         Some(name) => format!("Sets for: {}", name),
         None => "Select an Exercise".to_string(),
     };

     let table_block = Block::default()
         .borders(Borders::ALL)
         .title(title)
         .border_style(if app.log_focus == LogFocus::SetList {
             Style::default().fg(Color::Yellow)
         } else {
             Style::default().fg(Color::DarkGray)
         });

     let weight_unit = match app.service.config.units { Units::Metric => "kg", Units::Imperial => "lbs" };
     let dist_unit = match app.service.config.units { Units::Metric => "km", Units::Imperial => "mi" };
     let weight_cell = format!("Weight ({})", weight_unit);
     let distance_cell = format!("Distance ({})", dist_unit);
     let header_cells = ["Set", "Reps", &weight_cell, "Duration", &distance_cell, "Notes"]
         .into_iter()
         .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
     let header = Row::new(header_cells).height(1).bottom_margin(1);

     let rows = app.log_sets_for_selected_exercise.iter().enumerate().map(|(i, w)| {
         let weight_display = match app.service.config.units {
             Units::Metric => w.weight,
             Units::Imperial => w.weight.map(|kg| kg * 2.20462),
         };
         let weight_str = weight_display.map_or("-".to_string(), |v| format!("{:.1}", v));

         let dist_val = match app.service.config.units {
             Units::Metric => w.distance,
             Units::Imperial => w.distance.map(|km| km * 0.621_371),
         };
         let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1}", v));

         Row::new(vec![
             Cell::from(format!("{}", i + 1)),
             Cell::from(w.reps.map_or("-".to_string(), |v| v.to_string())),
             Cell::from(weight_str),
             Cell::from(w.duration_minutes.map_or("-".to_string(), |v| format!("{} min", v))),
             Cell::from(dist_str),
             Cell::from(w.notes.clone().unwrap_or_else(|| "-".to_string())),
         ])
     });

     let widths = [
         Constraint::Length(5), Constraint::Length(6), Constraint::Length(8),
         Constraint::Length(10), Constraint::Length(10), Constraint::Min(10),
     ];

     let table = Table::new(rows, widths)
         .header(header)
         .block(table_block)
         .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
         .highlight_symbol(">> ");

     f.render_stateful_widget(table, area, &mut app.log_set_table_state);
}

//task-athlete-tui/src/ui/mod.rs
// task-athlete-tui/src/ui/mod.rs

// Declare UI component modules
mod bodyweight_tab;
mod layout;
mod log_tab;
mod modals;
mod placeholders;
mod status_bar;
mod tabs;

// Re-export the main render function
pub use layout::render_ui; // Assuming render_ui is moved to layout.rs or stays here

//task-athlete-tui/src/ui/modals.rs
// task-athlete-tui/src/ui/modals.rs
use crate::{
    app::{state::{ActiveModal, LogBodyweightField, SetTargetWeightField}, App}, // Use App from crate::app
    ui::layout::centered_rect, // Use centered_rect from layout
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f), // Don't need app state for help text
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
        ActiveModal::None => {} // Should not happen if called correctly
    }
}

fn render_help_modal(f: &mut Frame) { // Removed unused `_app`
    let block = Block::default().title("Help (?)").borders(Borders::ALL)
               .title_style(Style::new().bold()).border_style(Style::new().yellow());
    let area = centered_rect(60, 70, f.size());
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let help_text = vec![
         Line::from("--- Global ---").style(Style::new().bold().underlined()),
         Line::from(" Q: Quit Application"),
         Line::from(" ?: Show/Hide This Help"),
         Line::from(" F1-F4: Switch Tabs"),
         Line::from(""),
         Line::from("--- Log Tab (F1) ---").style(Style::new().bold().underlined()),
         Line::from(" k / ↑: Navigate Up"),
         Line::from(" j / ↓: Navigate Down"),
         Line::from(" Tab: Switch Focus (Exercises List <=> Sets Table)"),
         Line::from(" h / ←: View Previous Day"),
         Line::from(" l / →: View Next Day"),
         Line::from(" a: Add New Workout Entry (for viewed day) (TODO)"),
         Line::from(" l: Log New Set (for selected exercise) (TODO)"),
         Line::from(" e / Enter: Edit Selected Set/Entry (TODO)"),
         Line::from(" d / Delete: Delete Selected Set/Entry (TODO)"),
         Line::from(" g: Go to Graphs for Selected Exercise (TODO)"),
         Line::from(""),
         Line::from("--- History Tab (F2) ---").style(Style::new().bold().underlined()),
         Line::from(" k/j / ↑/↓: Scroll History"),
         Line::from(" PgUp/PgDown: Scroll History Faster (TODO)"),
         Line::from(" / or f: Activate Filter Mode (TODO)"),
         Line::from(" e / Enter: Edit Selected Workout (TODO)"),
         Line::from(" d / Delete: Delete Selected Workout (TODO)"),
         Line::from(" Esc: Clear Filter / Exit Filter Mode (TODO)"),
         Line::from(""),
         Line::from("--- Graphs Tab (F3) ---").style(Style::new().bold().underlined()),
         Line::from(" Tab: Switch Focus (Selections) (TODO)"),
         Line::from(" k/j / ↑/↓: Navigate Selection List (TODO)"),
         Line::from(" Enter: Confirm Selection (TODO)"),
         Line::from(" /: Filter Exercise List (TODO)"),
         Line::from(""),
         Line::from("--- Bodyweight Tab (F4) ---").style(Style::new().bold().underlined()),
         Line::from(" Tab: Cycle Focus (Graph, Actions, History) (TODO)"),
         Line::from(" k/j / ↑/↓: Navigate History Table (when focused)"),
         Line::from(" l: Log New Bodyweight Entry"),
         Line::from(" t: Set/Clear Target Bodyweight"),
         Line::from(" r: Cycle Graph Time Range (1M > 3M > 6M > 1Y > All)"),
         Line::from(""),
         Line::from(Span::styled(" Press Esc, ?, or Enter to close ", Style::new().italic().yellow())),
     ];

    let paragraph = Paragraph::new(help_text).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 }));
}

fn render_log_bodyweight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::LogBodyweight { weight_input, date_input, focused_field, error_message } = &app.active_modal {
        let weight_unit = match app.service.config.units { Units::Metric => "kg", Units::Imperial => "lbs" };
        let block = Block::default().title("Log New Bodyweight").borders(Borders::ALL).border_style(Style::new().yellow());
        let area = centered_rect(50, 11, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                 Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
                 Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
                 Constraint::Length(1),
            ]).split(area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 }));

        f.render_widget(Paragraph::new(format!("Weight ({}):", weight_unit)), chunks[0]);
        f.render_widget(Paragraph::new("Date (YYYY-MM-DD / today):"), chunks[2]);

        let weight_style = if *focused_field == LogBodyweightField::Weight { Style::default().reversed() } else { Style::default() };
        f.render_widget(Paragraph::new(weight_input.as_str()).style(weight_style), chunks[1]);

        let date_style = if *focused_field == LogBodyweightField::Date { Style::default().reversed() } else { Style::default() };
        f.render_widget(Paragraph::new(date_input.as_str()).style(date_style), chunks[3]);

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[4]);

        let ok_button = Paragraph::new(" OK ").alignment(ratatui::layout::Alignment::Center)
                        .style(if *focused_field == LogBodyweightField::Confirm { Style::default().reversed() } else { Style::default() });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ").alignment(ratatui::layout::Alignment::Center)
                            .style(if *focused_field == LogBodyweightField::Cancel { Style::default().reversed() } else { Style::default() });
        f.render_widget(cancel_button, button_layout[1]);

        if let Some(err) = error_message {
            f.render_widget(Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)), chunks[6]);
        }

        match focused_field {
            LogBodyweightField::Weight => f.set_cursor(chunks[1].x + weight_input.chars().count() as u16, chunks[1].y),
            LogBodyweightField::Date => f.set_cursor(chunks[3].x + date_input.chars().count() as u16, chunks[3].y),
            _ => {}
        }
    }
}

fn render_set_target_weight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::SetTargetWeight { weight_input, focused_field, error_message } = &app.active_modal {
         let weight_unit = match app.service.config.units { Units::Metric => "kg", Units::Imperial => "lbs" };
         let block = Block::default().title("Set Target Bodyweight").borders(Borders::ALL).border_style(Style::new().yellow());
         let area = centered_rect(50, 11, f.size());
         f.render_widget(Clear, area);
         f.render_widget(block, area);

         let chunks = Layout::default()
             .direction(Direction::Vertical)
             .margin(1)
             .constraints([
                 Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
                 Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
             ]).split(area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 1 }));

         f.render_widget(Paragraph::new(format!("Target Weight ({}):", weight_unit)), chunks[0]);

         let weight_style = if *focused_field == SetTargetWeightField::Weight { Style::default().reversed() } else { Style::default() };
         f.render_widget(Paragraph::new(weight_input.as_str()).style(weight_style), chunks[1]);

         let button_layout = Layout::default()
             .direction(Direction::Horizontal)
             .constraints([ Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33) ])
             .split(chunks[3]);

         let set_button = Paragraph::new(" Set ").alignment(ratatui::layout::Alignment::Center)
                          .style(if *focused_field == SetTargetWeightField::Set { Style::default().reversed() } else { Style::default() });
         f.render_widget(set_button, button_layout[0]);

         let clear_button = Paragraph::new(" Clear Target ").alignment(ratatui::layout::Alignment::Center)
                           .style(if *focused_field == SetTargetWeightField::Clear { Style::default().reversed() } else { Style::default() });
         f.render_widget(clear_button, button_layout[1]);

         let cancel_button = Paragraph::new(" Cancel ").alignment(ratatui::layout::Alignment::Center)
                             .style(if *focused_field == SetTargetWeightField::Cancel { Style::default().reversed() } else { Style::default() });
         f.render_widget(cancel_button, button_layout[2]);

         if let Some(err) = error_message {
             f.render_widget(Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)), chunks[5]);
         }

         match focused_field {
             SetTargetWeightField::Weight => f.set_cursor(chunks[1].x + weight_input.chars().count() as u16, chunks[1].y),
             _ => {}
         }
     }
}

//task-athlete-tui/src/ui/placeholders.rs
// task-athlete-tui/src/ui/placeholders.rs
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_placeholder(f: &mut Frame, title: &str, area: Rect) {
    let placeholder_text = Paragraph::new(format!("{} - Implementation Pending", title))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    f.render_widget(placeholder_text, area);
}

//task-athlete-tui/src/ui/status_bar.rs
// task-athlete-tui/src/ui/status_bar.rs
use crate::app::{state::ActiveModal, App}; // Use App from crate::app
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.active_modal {
         ActiveModal::None => match app.active_tab {
             crate::app::ActiveTab::Log => "[Tab] Focus | [↑↓/jk] Nav | [←→/hl] Date | [a]dd | [l]og set | [e]dit | [d]elete | [g]raphs | [?] Help | [Q]uit ",
             crate::app::ActiveTab::History => "[↑↓/jk] Nav | [/f] Filter | [e]dit | [d]elete | [?] Help | [Q]uit ",
             crate::app::ActiveTab::Graphs => "[Tab] Focus | [↑↓/jk] Nav | [/] Filter Exercise | [Enter] Select | [?] Help | [Q]uit ",
             crate::app::ActiveTab::Bodyweight => "[↑↓/jk] Nav Hist | [l]og | [t]arget | [r]ange | [?] Help | [Q]uit ",
         }.to_string(),
         ActiveModal::Help => " [Esc/Enter/?] Close Help ".to_string(),
         ActiveModal::LogBodyweight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
         ActiveModal::SetTargetWeight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
     };

    let error_text = app.last_error.as_deref().unwrap_or("");

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(area);

    let status_paragraph = Paragraph::new(status_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status_paragraph, status_chunks[0]);

    let error_paragraph = Paragraph::new(error_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::Red))
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(error_paragraph, status_chunks[1]);
}

//task-athlete-tui/src/ui/tabs.rs
// task-athlete-tui/src/ui/tabs.rs
use crate::app::{ActiveTab, App}; // Use App from crate::app
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};

pub fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ["Log (F1)", "History (F2)", "Graphs (F3)", "Bodyweight (F4)"]
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::Gray))))
        .collect();

    let selected_tab_index = match app.active_tab {
        ActiveTab::Log => 0,
        ActiveTab::History => 1,
        ActiveTab::Graphs => 2,
        ActiveTab::Bodyweight => 3,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(selected_tab_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

