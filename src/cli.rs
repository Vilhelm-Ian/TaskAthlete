// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};

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
    BodyWeight
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Define a new exercise type
    CreateExercise {
        /// Name of the exercise (e.g., "Bench Press", "Running")
        #[arg(short, long)]
        name: String,
        /// Type of exercise
        #[arg(short, long, value_enum)]
        type_: ExerciseTypeCli,
        /// Comma-separated list of target muscles (e.g., "chest,triceps,shoulders")
        #[arg(short, long)]
        muscles: Option<String>,
    },
    DeleteExercise {
        identifier: String, // Can be either string or number
    },
    EditExercise {
        identifier: String, // Can be either string or number
        name: Option<String>,
        #[arg(short, long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Comma-separated list of target muscles (e.g., "chest,triceps,shoulders")
        #[arg(short, long)]
        muscles: Option<String>,
    },
    /// Add a new workout entry
    Add {
        /// Name of the exercise
        #[arg(short, long)]
        exercise: String,

        /// Number of sets performed
        #[arg(short, long)]
        sets: Option<i64>, // Use i64 for SQLite INTEGER compatibility

        /// Number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,

        /// Weight used (e.g., kg, lbs - specify unit in notes or exercise name if needed)
        #[arg(short, long)]
        weight: Option<f64>, // Use f64 for SQLite REAL compatibility

        /// Duration in minutes (for cardio or timed exercises)
        #[arg(short, long)]
        duration: Option<i64>,

        /// Additional notes about the workout
        #[arg(short, long)]
        notes: Option<String>,

        // Optional fields for implicit exercise creation during 'add'
        #[arg(long = "type", value_enum, requires = "muscles", id = "implicit-exercise-type")]
        exercise_type: Option<ExerciseTypeCli>,

        #[arg(long, requires = "implicit-exercise-type")] // Refer to the ID 
        muscles: Option<String>,
    },
    EditWorkout {
        identifier: String, // Can be the name or id
        /// New name of the exercise
        #[arg(short, long)]
        exercise: Option<String>,
        /// New number of sets performed
        #[arg(short, long)]
        sets: Option<i64>,
        /// New number of repetitions per set
        #[arg(short, long)]
        reps: Option<i64>,
        /// New weight used
        #[arg(short, long)]
        weight: Option<f64>,
        /// New duration in minutes
        #[arg(short, long)]
        duration: Option<i64>,
        /// New additional notes
        #[arg(short, long)]
        notes: Option<String>,
    },
    DeleteWorkout {
        /// ID of the workout to delete
        id: i64,
    },
    /// List workout entries
    List {
        /// Show only the last N entries
        #[arg(short, long, default_value_t = 20, conflicts_with_all = &["today", "yesterday", "exercise", "nth_last_day"])]
        limit: u32,
        #[arg(long, conflicts_with_all = &["yesterday", "nth_last_day", "limit"])]
        today: bool,
        #[arg(long, conflicts_with_all = &["today", "nth_last_day", "limit"])]
        yesterday: bool,
        #[arg(long, conflicts_with = "limit")]
        exercise: Option<String>,
        #[arg(long, requires = "exercise", value_name = "N", conflicts_with_all = &["today", "yesterday", "limit"])]
        nth_last_day: Option<u32>,
    },
     /// List defined exercise types
    ListExercises {
        /// Filter by exercise type
        #[arg(long, value_enum)]
        type_: Option<ExerciseTypeCli>,
        /// Filter by a target muscle (matches if the muscle is in the list)
        #[arg(long)]
        muscle: Option<String>,
    },
    /// Show the path to the database file
    DbPath,
    SetBodyWeight{ weight: f64}
}

// Function to parse CLI arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

