//! Defines the command-line interface structure using clap.

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
                    "Invalid date format: '{}'. Use 'today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, or YYYY/MM/DD.",
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
        #[arg(short = 't', long, value_enum)]
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
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// New comma-separated list of target muscles
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Add a new workout entry
    Add {
        /// Name, ID, or Alias of the exercise (will prompt to create if not found and type/muscles given)
        #[arg(short = 'e', long)]
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
        #[arg(short = 'd', long)]
        duration: Option<i64>,

        /// Distance covered (e.g., km, miles)
        #[arg(short = 'l', long)]
        distance: Option<f64>,

        /// Additional notes about the workout
        #[arg(short, long)]
        notes: Option<String>,

        /// Date of the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand, default_value = "today")]
        date: NaiveDate,

        // Optional fields for implicit exercise creation during 'add' if exercise not found
        #[arg(
            long = "type",
            value_enum,
            requires = "implicit-muscles",
            id = "implicit-exercise-type"
        )]
        implicit_type: Option<ExerciseTypeCli>,

        #[arg(long, requires = "implicit-exercise-type", id = "implicit-muscles")]
        implicit_muscles: Option<String>,
    },
    /// Edit an existing workout entry
    EditWorkout {
        /// ID of the workout entry to edit
        id: i64,
        /// New exercise Name, ID or Alias for the workout
        #[arg(short = 'e', long)]
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
        #[arg(short = 'd', long)]
        duration: Option<i64>,
        /// New distance covered (e.g., km, miles)
        #[arg(short = 'l', long)]
        distance: Option<f64>,
        /// New additional notes
        #[arg(short, long)]
        notes: Option<String>,
        /// New date for the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand)]
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
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = ["today_flag", "yesterday_flag", "nth_last_day_exercise"])]
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>,

        /// Show only the last N entries (when no date/day filters used)
        #[arg(short = 'n', long, default_value_t = 20, conflicts_with_all = ["today_flag", "yesterday_flag", "date", "nth_last_day_exercise"])]
        limit: u32,

        // Keep flags for backward compatibility or preference, but date is more versatile
        #[arg(long, conflicts_with_all = ["yesterday_flag", "date", "nth_last_day_exercise", "limit"])]
        today_flag: bool,
        #[arg(long, conflicts_with_all = ["today_flag", "date", "nth_last_day_exercise", "limit"])]
        yesterday_flag: bool,

        /// Show workouts for the Nth most recent day a specific exercise (Name, ID, Alias) was performed
        #[arg(long, value_name = "EXERCISE_IDENTIFIER", requires = "nth_last_day_n", conflicts_with_all = ["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_exercise: Option<String>,
        #[arg(long, value_name = "N", requires = "nth_last_day_exercise", conflicts_with_all = ["limit", "date", "today_flag", "yesterday_flag", "exercise", "type_", "muscle"])]
        nth_last_day_n: Option<u32>,
    },
    /// List defined exercise types
    ListExercises {
        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Filter by a target muscle (matches if the muscle is in the list)
        #[arg(short = 'm', long)]
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
        /// The alias name (e.g., "bp") - Must be unique
        alias_name: String,
        /// The ID, Name, or existing Alias of the exercise to alias
        exercise_identifier: String,
    },
    /// Delete an exercise alias
    Unalias {
        /// The alias name to delete
        alias_name: String,
    },
    /// List all defined exercise aliases
    ListAliases,
    /// Show the path to the database file
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
    SetTargetWeight { weight: f64 },
    /// Delete a bodyweight entry
    DeleteBodyweight { id: i64 },
    /// Clear your target bodyweight from the config file
    ClearTargetWeight,
    /// Show the path to the config file
    ConfigPath,
    /// Enable or disable Personal Best (PB) notifications globally
    SetPbNotification {
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
        /// Filter by exercise Name, ID or Alias
        #[arg(short = 'e', long)]
        exercise: Option<String>,

        /// Filter by a specific date ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY)
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = ["start_date", "end_date", "limit_days"])]
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>,

        /// Show only the last N days with workouts (when no date/range filters used)
        #[arg(short = 'n', long, default_value_t = 7, conflicts_with_all = ["date", "start_date", "end_date"])]
        limit_days: u32,

        // Optional date range
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = ["date", "limit_days"])]
        start_date: Option<NaiveDate>,
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = ["date", "limit_days"], requires="start_date")]
        end_date: Option<NaiveDate>,
    },
    /// Set default units (Metric/Imperial)
    SetUnits {
        #[arg(value_enum)]
        units: UnitsCli,
    },
    /// Generate shell completion script
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

// Function to build the command structure (needed for completion generation)
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
    use super::*;
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
        let result = parse_date_shorthand("2023-02-30");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));

        let result = parse_date_shorthand("32.10.2023");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));

        let result = parse_date_shorthand("2023/13/01");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid date format"));
    }
}
