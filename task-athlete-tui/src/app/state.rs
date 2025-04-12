// task-athlete-tui/src/app/state.rs
use crate::app::AppInputError; // Use error from parent mod
use chrono::Utc;
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
}
