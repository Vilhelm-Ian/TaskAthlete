use thiserror::Error;

// Declare the modules within the app directory
pub mod actions;
pub mod data;
pub mod input;
pub mod modals;
pub mod navigation;
pub mod navigation_helpers;
pub mod state;
pub mod utils;

// Re-export the main App struct and other necessary types for convenience
pub use state::{
    ActiveTab,
    AddWorkoutField,
    App, // Add AddExerciseField
}; // Add other enums if needed

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
    #[error("Exercise name cannot be empty.")] // NEW specific error
    ExerciseNameEmpty,
}
