// task-athlete-tui/src/app/state.rs
use crate::app::AppInputError; // Use error from parent mod
use chrono::Utc;
use ratatui::widgets::{ListState, TableState};
use std::time::Instant;
use task_athlete_lib::{AppService, ExerciseDefinition, ExerciseType, Workout, WorkoutFilters}; // Keep lib imports

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddExerciseField {
    Name,
    Muscles,
    TypeResistance, // Represents focus on the "Resistance" option
    TypeCardio,     // Represents focus on the "Cardio" option
    TypeBodyweight, // Represents focus on the "BodyWeight" option
    Confirm,
    Cancel,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddWorkoutField {
    Exercise, // Text input for exercise name/alias
    Sets,
    Reps,
    Weight,
    Duration,
    Distance,
    Notes,
    Confirm,
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
    AddWorkout {
        exercise_input: String, // Name or Alias
        sets_input: String,
        reps_input: String,
        weight_input: String, // Added weight for bodyweight, direct for others
        duration_input: String,
        distance_input: String,
        notes_input: String,
        focused_field: AddWorkoutField,
        error_message: Option<String>,
        // Store the resolved definition temporarily after user leaves exercise field
        resolved_exercise: Option<ExerciseDefinition>,
    },
    CreateExercise {
        name_input: String,
        muscles_input: String,
        selected_type: ExerciseType, // Store the currently selected type
        focused_field: AddExerciseField,
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
    pub bw_history: Vec<(chrono::DateTime<Utc>, f64)>,
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
    pub fn get_last_workout_for_exercise(&self, canonical_name: &str) -> Option<Workout> {
        let filters = WorkoutFilters {
            exercise_name: Some(canonical_name),
            limit: Some(1), // Get only the most recent one
            ..Default::default()
        };
        // Ignore errors here, just return None if fetch fails
        match self.service.list_workouts(filters) {
            Ok(workouts) if !workouts.is_empty() => workouts.into_iter().next(),
            _ => None,
        }
    }
}
