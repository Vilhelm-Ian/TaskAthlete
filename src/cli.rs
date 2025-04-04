// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};
use chrono::{NaiveDate, Utc, Duration}; 

#[derive(Parser, Debug)]
#[command(author, version, about = "A CLI tool to track workouts", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
            }
            else {
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
        identifier: String,
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

        /// Additional notes about the workout
        #[arg(short, long)]
        notes: Option<String>,

        /// Date of the workout ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, YYYY/MM/DD)
        #[arg(long, value_parser = parse_date_shorthand, default_value = "today")] // Feature 3
        date: NaiveDate,

        // Optional fields for implicit exercise creation during 'add' if exercise not found
        #[arg(long = "type", value_enum, requires = "implicit-muscles", id = "implicit-exercise-type")]
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
        id: i64,
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
        #[arg(short='t', long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Filter by a target muscle (matches if the muscle is in the list)
        #[arg(short='m', long)] // short 'm'
        muscle: Option<String>,
    },
    /// Create an alias for an existing exercise
    Alias { // Feature 1
        /// The alias name (e.g., "bp") - Must be unique
        alias_name: String,
        /// The ID, Name, or existing Alias of the exercise to alias
        exercise_identifier: String,
    },
    /// Delete an exercise alias
    Unalias { // Feature 1
        /// The alias name to delete
        alias_name: String,
    },
    /// List all defined exercise aliases
    ListAliases, // Feature 1
    /// Show the path to the database file
    DbPath,
    /// Show the path to the config file
    ConfigPath,
    /// Set your bodyweight in the config file
    SetBodyweight{
        /// Your current bodyweight
        weight: f64
    },
    /// Enable or disable Personal Best (PB) notifications
    SetPbNotification { // Feature 4
        /// Enable PB notifications (`true` or `false`)
        enabled: bool,
    },
    /// Show total workout volume (sets*reps*weight) per day
    Volume { // Feature 1
        /// Filter by exercise Name, ID or Alias
        #[arg(short = 'e', long)]
        exercise: Option<String>,

        /// Filter by a specific date ('today', 'yesterday', YYYY-MM-DD, DD.MM.YYYY, Weekday Name)
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["start_date", "end_date", "limit_days"])] // Corrected conflicts
        date: Option<NaiveDate>,

        /// Filter by exercise type
        #[arg(short = 't', long, value_enum)]
        type_: Option<ExerciseTypeCli>,

        /// Filter by target muscle (matches if muscle is in the list)
        #[arg(short, long)]
        muscle: Option<String>,

        /// Show only the last N days with workouts (when no date/range filters used)
        #[arg(short = 'n', long, default_value_t = 7, conflicts_with_all = &["date", "start_date", "end_date"])] // Corrected conflicts
        limit_days: u32,

        // Optional date range
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["date", "limit_days"])] // Corrected conflicts
        start_date: Option<NaiveDate>,
        #[arg(long, value_parser = parse_date_shorthand, conflicts_with_all = &["date", "limit_days"], requires="start_date")] // Corrected conflicts and added requires
        end_date: Option<NaiveDate>,
    },
    /// Set default units (Metric/Imperial)
    SetUnits { // Feature 3
        #[arg(value_enum)]
        units: UnitsCli,
     },
}

// Function to parse CLI arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum UnitsCli {
    Metric,
    Imperial,
}
