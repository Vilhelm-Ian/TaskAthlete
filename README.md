# Task Athlete Library (`task-athlete-lib`)

`task-athlete-lib` is the core Rust library that powers the Task Athlete suite of workout tracking tools, including the [CLI (`ta`)](https://github.com/Vilhelm-Ian/TaskAthleteCLI), [TUI (`ta-tui`)](https://github.com/Vilhelm-Ian/TaskAthleteTUI), [GUI](https://github.com/Vilhelm-Ian/TaskAthleteGUI). It handles all business logic, data storage, configuration management, and calculations related to workout tracking.

## Table of Contents

- [Features](#features)
- [Core Concepts](#core-concepts)
  - [AppService](#appservice)
  - [Exercises](#exercises)
  - [Workouts](#workouts)
  - [Bodyweight](#bodyweight)
  - [Configuration](#configuration)
  - [Personal Bests (PBs)](#personal-bests-pbs)
  - [Statistics](#statistics)
- [Usage](#usage)
  - [Initialization](#initialization)
  - [Common Operations](#common-operations)
- [Data Storage](#data-storage)
- [Configuration Management](#configuration-management)
- [Error Handling](#error-handling)
- [Modules](#modules)
- [Contributing](#contributing)
- [License](#license)

## Features

*   **Exercise Definition:** Create, edit, and delete custom exercise types (Resistance, Cardio, Bodyweight) with specific logging parameters (weight, reps, duration, distance).
*   **Workout Logging:** Add, edit, and delete detailed workout entries, including sets, reps, weight, duration, distance, and notes.
*   **Bodyweight Tracking:** Log bodyweight entries, retrieve history, and manage target bodyweight.
*   **Alias Management:** Create and manage aliases for exercises for quicker input.
*   **Data Querying:** Filter and list workouts and exercises based on various criteria (date, type, muscle).
*   **Statistics Calculation:**
    *   Calculate personal bests (PBs) for weight, reps, duration, and distance.
    *   Determine workout streaks and total volume.
    *   Provide comprehensive stats for individual exercises.
*   **Configuration:** Manage user preferences such as units (Metric/Imperial), bodyweight, PB notifications, and theme settings.
*   **Data Persistence:** Uses SQLite for robust and local data storage.
*   **Graph Data Preparation:** Provides processed data suitable for generating progress graphs.
*   **Units Conversion:** Handles conversion between Metric and Imperial units for display and storage.

## Core Concepts

### AppService

The `AppService` struct is the main entry point and orchestrator for all library functionalities. It holds the database connection, loaded configuration, and provides methods for all operations.

### Exercises

*   **`ExerciseDefinition`**: Represents a type of exercise (e.g., "Bench Press", "Running"). It includes:
    *   Name (unique, case-insensitive)
    *   Type (`ExerciseType`: Resistance, Cardio, BodyWeight)
    *   Muscles targeted (optional, comma-separated string)
    *   Logging flags (`log_weight`, `log_reps`, `log_duration`, `log_distance`) to control which metrics are relevant for this exercise.
*   **Aliases**: Users can create short names (aliases) that map to canonical exercise names for faster workout logging.

### Workouts

*   **`Workout`**: Represents a single workout set or activity. It includes:
    *   Timestamp of the workout.
    *   Canonical exercise name.
    *   Optional metrics: sets, reps, weight (additional weight), duration (minutes), distance (stored in km).
    *   `bodyweight`: The user's bodyweight at the time of a `BodyWeight` type exercise, used for accurate volume/intensity calculations.
    *   Notes.
*   **Effective Weight**: For `BodyWeight` exercises, the library can calculate effective weight (additional weight + bodyweight used).

### Bodyweight

*   Users can log their bodyweight on specific dates.
*   The library supports setting and retrieving a target bodyweight.
*   The most recently logged bodyweight can be used for calculations in `BodyWeight` exercises.

### Configuration

*   **`Config`**: A struct holding user preferences, loaded from a `config.toml` file.
    *   `units`: `Units::Metric` or `Units::Imperial`.
    *   `bodyweight`: Current bodyweight.
    *   `target_bodyweight`: Desired bodyweight.
    *   `pb_notifications`: Settings for enabling/disabling notifications for different PB metrics.
    *   `streak_interval_days`: Interval for calculating workout streaks.
    *   `theme`: Basic theming options (e.g., `header_color`).
*   Configuration is typically stored in a platform-specific user config directory (e.g., `~/.config/workout-tracker-cli/config.toml`).

### Personal Bests (PBs)

*   The library automatically detects and can report new personal bests when a workout is added.
*   PBs are tracked for: Max Weight, Max Reps, Max Duration, Max Distance.
*   PB notification behavior is configurable.

### Statistics

*   **`ExerciseStats`**: Provides a summary for a given exercise, including:
    *   Total workouts, first/last workout date.
    *   Average workouts per week, longest gap between sessions.
    *   Current and longest workout streaks.
    *   All-time personal bests.
*   **Volume**: `calculate_daily_volume_filtered` calculates daily workout volume (sets * reps * weight) based on various filters.

## Usage

This library is intended to be used as a dependency by frontend applications (CLI, TUI, GUI).

### Initialization

First, initialize the `AppService`:

```rust
use task_athlete_lib::AppService;

match AppService::initialize() {
    Ok(service) => {
        // Use the service instance
        println!("Service initialized. DB path: {:?}", service.get_db_path());
        println!("Config path: {:?}", service.get_config_path());
    }
    Err(e) => {
        eprintln!("Failed to initialize Task Athlete service: {}", e);
    }
}
```

### Common Operations

(Examples of how to use key `AppService` methods)

```rust
# // Assuming 'service' is an initialized AppService instance
# use task_athlete_lib::{AppService, ExerciseType, AddWorkoutParams, WorkoutFilters, Units, Utc};
# use anyhow::Result;
#
# fn doc_examples(mut service: AppService) -> Result<()> {
#
// Create a new exercise
service.create_exercise("Squat", ExerciseType::Resistance, None, Some("legs,glutes"))?;

// Add a workout
let workout_params = AddWorkoutParams {
    exercise_identifier: "Squat",
    date: Utc::now(),
    sets: Some(3),
    reps: Some(8),
    weight: Some(100.0), // Assuming units are kg if Metric
    bodyweight_to_use: service.config.bodyweight, // Pass current bodyweight if BodyWeight exercise
    ..Default::default()
};
let (workout_id, pb_info_option) = service.add_workout(workout_params)?;
if let Some(pb_info) = pb_info_option {
    if pb_info.any_pb() {
        println!("New PB achieved for Squat! {:?}", pb_info);
    }
}

// List workouts for "Squat"
let squat_workouts = service.list_workouts(&WorkoutFilters {
    exercise_name: Some("Squat"),
    ..Default::default()
})?;
for workout in squat_workouts {
    println!("Squat workout on {:?}: {:?}", workout.timestamp, workout);
}

// Set units to Imperial
service.set_units(Units::Imperial)?;
service.save_config()?; // Persist change

// Get exercise statistics
match service.get_exercise_stats("Squat") {
    Ok(stats) => println!("Squat Stats: {:?}", stats),
    Err(e) => eprintln!("Error getting stats: {}", e),
}
# Ok(())
# }
```

Refer to the `AppService` struct's public methods in `src/lib.rs` for a complete list of available operations.

## Data Storage

*   All workout data, exercise definitions, and aliases are stored in a **SQLite database**.
*   The default database file is `workouts.sqlite`, located in the application's data directory (e.g., `~/.local/share/workout-tracker-cli/` on Linux).
*   The database schema is defined and initialized in `src/db.rs`.
*   The library handles schema migrations for some columns (e.g., adding `distance`, `bodyweight`, log flags to existing tables).

## Configuration Management

*   User configuration is stored in a TOML file (typically `config.toml`).
*   The path can be found using `AppService::get_config_path()` or the CLI command `ta config-path`.
*   The `config` module handles loading, saving, and defining the structure of the configuration.
*   Environment variable `WORKOUT_CONFIG_DIR` can override the default configuration directory.

## Error Handling

*   The library primarily uses `anyhow::Result` for operations in the `AppService` layer, providing context-rich errors.
*   The `db` module defines a specific `db::Error` enum for database-related errors.
*   The `config` module defines a `config::ConfigError` enum for configuration-related errors.
*   These specific errors are often wrapped by `anyhow::Error` when propagated from `AppService`.

## Modules

*   **`config.rs`**: Handles application configuration, including loading, saving, and defining the `Config` struct and related enums like `Units` and `StandardColor`.
*   **`db.rs`**: Manages all interactions with the SQLite database. This includes schema definition, CRUD operations for exercises, workouts, aliases, and bodyweight entries, as well as complex queries for statistics and filtering.
*   **`lib.rs` (root)**: Defines the main `AppService` struct, which acts as the public API for the library. It orchestrates calls to the `config` and `db` modules and implements higher-level business logic (e.g., PB checking, stats aggregation).

## Contributing

Contributions to `task-athlete-lib` are welcome! If you're interested in improving the core logic, adding new features, or fixing bugs, please:

1.  Fork the repository.
2.  Create a new branch for your feature or bug fix.
3.  Make your changes, ensuring to add relevant tests (see `tests/lib_test.rs`).
4.  Commit your changes and push them to your fork.
5.  Open a pull request against the main repository.

Please ensure your code adheres to the existing style and includes documentation where appropriate.

## License

This project is licensed under the [MIT License](LICENSE).
