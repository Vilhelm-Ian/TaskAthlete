// task-athlete-tui/src/app.rs
use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::{ListState, TableState};
use std::collections::HashMap;
use std::time::Instant;
use task_athlete_lib::{
    AppService, ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters,
};

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
    Graph,   // Maybe just for range selection in future?
    Actions, // Placeholder if actions become selectable
    History,
}

// Represents the state of active modals (simplified for now)
#[derive(Clone, Debug, PartialEq)]
pub enum ActiveModal {
    None,
    Help,
    // Add more here: AddWorkout, LogSet, EditWorkout, ConfirmDelete, etc.
    // Example:
    // AddWorkout {
    //     exercise_input: String,
    //     date_input: String,
    //     // ... other fields ...
    //     focused_field: AddWorkoutField,
    // }
}

// Holds the application state
pub struct App {
    pub service: AppService, // The core service from the library
    pub active_tab: ActiveTab,
    pub should_quit: bool,
    pub active_modal: ActiveModal,
    pub last_error: Option<String>, // To display errors

    // === Log Tab State ===
    pub log_focus: LogFocus,
    pub log_viewed_date: NaiveDate,
    pub log_exercises_today: Vec<String>, // Names of exercises logged on viewed_date
    pub log_exercise_list_state: ListState,
    pub log_sets_for_selected_exercise: Vec<Workout>, // Sets for the selected exercise on viewed_date
    pub log_set_table_state: TableState,

    // === History Tab State ===
    // TODO: Implement history tab state (scroll offset, selection, filter, data)

    // === Graph Tab State ===
    // TODO: Implement graph tab state (selections, data)

    // === Bodyweight Tab State ===
    pub bw_focus: BodyweightFocus,
    pub bw_history: Vec<(i64, NaiveDate, f64)>, // (id, date, weight)
    pub bw_history_state: TableState,
    pub bw_target: Option<f64>,
    pub bw_latest: Option<f64>,
    pub bw_graph_data: Vec<(f64, f64)>, // (days_since_epoch, weight)
    pub bw_graph_x_bounds: [f64; 2],
    pub bw_graph_y_bounds: [f64; 2],
    pub bw_graph_range_months: u32, // 1, 3, 6, 12, 0 (All)

    // For debouncing error messages
    error_clear_time: Option<Instant>,
}

impl App {
    pub fn new(service: AppService) -> Self {
        let today = Utc::now().date_naive();
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
            bw_latest: None, // Will be fetched
            bw_graph_data: Vec::new(),
            bw_graph_x_bounds: [0.0, 1.0], // Placeholder
            bw_graph_y_bounds: [0.0, 1.0], // Placeholder
            bw_graph_range_months: 3,      // Default to 3 months
            last_error: None,
            error_clear_time: None,
            service, // Move service in
        };
        // Select first item by default if lists are populated
        app.log_exercise_list_state.select(Some(0));
        app.log_set_table_state.select(Some(0));
        app.bw_history_state.select(Some(0));
        app.refresh_data_for_active_tab(); // Initial data load
        app
    }

    // Fetch or update data based on the active tab
    pub fn refresh_data_for_active_tab(&mut self) {
        // Clear error message after a delay
        if let Some(clear_time) = self.error_clear_time {
            if Instant::now() >= clear_time {
                self.last_error = None;
                self.error_clear_time = None;
            }
        }

        match self.active_tab {
            ActiveTab::Log => self.refresh_log_data(),
            ActiveTab::History => {} // TODO
            ActiveTab::Graphs => {}  // TODO
            ActiveTab::Bodyweight => self.refresh_bodyweight_data(),
        }
    }

    fn set_error(&mut self, msg: String) {
        self.last_error = Some(msg);
        self.error_clear_time = Some(Instant::now() + Duration::seconds(5).to_std().unwrap());
    }

    // --- Log Tab Data ---
    fn refresh_log_data(&mut self) {
        let filters = WorkoutFilters {
            date: Some(self.log_viewed_date),
            ..Default::default()
        };
        match self.service.list_workouts(filters) {
            Ok(workouts) => {
                // Get unique exercise names for the left pane
                let mut unique_names = workouts
                    .iter()
                    .map(|w| w.exercise_name.clone())
                    .collect::<Vec<_>>();
                unique_names.sort_unstable();
                unique_names.dedup();
                self.log_exercises_today = unique_names;

                // Clamp selection index
                if self.log_exercise_list_state.selected().unwrap_or(0)
                    >= self.log_exercises_today.len()
                {
                    self.log_exercise_list_state
                        .select(if self.log_exercises_today.is_empty() {
                            None
                        } else {
                            Some(self.log_exercises_today.len() - 1)
                        });
                }

                // Update sets for the currently selected exercise
                self.update_log_sets_for_selected_exercise(&workouts);
            }
            Err(e) => self.set_error(format!("Error fetching log data: {}", e)),
        }
    }

    fn update_log_sets_for_selected_exercise(&mut self, all_workouts_for_date: &[Workout]) {
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) = self.log_exercises_today.get(selected_index) {
                self.log_sets_for_selected_exercise = all_workouts_for_date
                    .iter()
                    .filter(|w| &w.exercise_name == selected_exercise_name)
                    .cloned() // Clone the workouts needed for the right pane
                    .collect();

                // Clamp selection index
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
    fn refresh_bodyweight_data(&mut self) {
        // Fetch history using the updated library function
        match self.service.list_bodyweights(1000) {
            // Fetch more for graph
            Ok(entries) => {
                // Assign directly - entries is already Vec<(i64, NaiveDate, f64)>
                self.bw_history = entries;

                // Clamp selection
                if self.bw_history_state.selected().unwrap_or(0) >= self.bw_history.len() {
                    self.bw_history_state.select(if self.bw_history.is_empty() {
                        None
                    } else {
                        Some(self.bw_history.len() - 1)
                    });
                }

                // Update latest and target
                self.bw_latest = self.bw_history.first().map(|(_, _, w)| *w);
                self.bw_target = self.service.get_target_bodyweight(); // Refresh target

                // Update graph data
                self.update_bw_graph_data();
            }
            Err(e) => self.set_error(format!("Error fetching bodyweight data: {}", e)),
        }
    }

    fn update_bw_graph_data(&mut self) {
        if self.bw_history.is_empty() {
            self.bw_graph_data.clear();
            self.bw_graph_x_bounds = [0.0, 1.0];
            self.bw_graph_y_bounds = [0.0, 1.0];
            return;
        }

        let now_naive = Utc::now().date_naive();
        let start_date_filter = if self.bw_graph_range_months > 0 {
            now_naive
                .checked_sub_months(chrono::Months::new(self.bw_graph_range_months))
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        // Fallback
        } else {
            self.bw_history
                .last()
                .map(|(_, d, _)| *d)
                .unwrap_or(now_naive) // All time: use earliest date
        };

        // Filter data for the selected range and reverse for chronological order
        let filtered_data: Vec<_> = self
            .bw_history
            .iter()
            .filter(|(_, date, _)| *date >= start_date_filter)
            .rev() // Reverse to chronological for graphing
            .collect();

        if filtered_data.is_empty() {
            self.bw_graph_data.clear();
            // Keep old bounds? Or reset? Resetting might be jarring. Let's keep old bounds.
            return;
        }

        let first_day_epoch = filtered_data.first().unwrap().1.num_days_from_ce(); // Use num_days_from_ce for x-axis

        self.bw_graph_data = filtered_data
            .iter()
            .map(|(_, date, weight)| {
                let days_since_first = (date.num_days_from_ce() - first_day_epoch) as f64;
                (days_since_first, *weight)
            })
            .collect();

        // Calculate bounds
        let first_ts = self.bw_graph_data.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last_ts = self
            .bw_graph_data
            .last()
            .map(|(x, _)| *x)
            .unwrap_or(first_ts + 1.0); // Avoid zero range
        self.bw_graph_x_bounds = [first_ts, last_ts];

        let min_weight = self
            .bw_graph_data
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_weight = self
            .bw_graph_data
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);

        // Include target weight in y-bounds calculation if set
        let y_min = self.bw_target.map_or(min_weight, |t| t.min(min_weight));
        let y_max = self.bw_target.map_or(max_weight, |t| t.max(max_weight));

        // Add padding to y-bounds
        let y_padding = (y_max - y_min) * 0.1;
        self.bw_graph_y_bounds = [(y_min - y_padding).max(0.0), y_max + y_padding];
        // Ensure min is not negative
    }

    // --- Input Handling ---
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle based on active modal first
        if self.active_modal != ActiveModal::None {
            return self.handle_modal_input(key);
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.active_modal = ActiveModal::Help,
            KeyCode::F(1) => self.active_tab = ActiveTab::Log,
            KeyCode::F(2) => self.active_tab = ActiveTab::History,
            KeyCode::F(3) => self.active_tab = ActiveTab::Graphs,
            KeyCode::F(4) => self.active_tab = ActiveTab::Bodyweight,
            // Potentially Ctrl+Left/Right for tab switching too
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
            // Handle other modals (Add/Edit/Delete/LogBW/TargetBW etc.)
            _ => {
                // Default close on Esc for now
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
                KeyCode::Char('k') | KeyCode::Up => self.log_list_previous(),
                KeyCode::Char('j') | KeyCode::Down => self.log_list_next(),
                KeyCode::Tab => self.log_focus = LogFocus::SetList,
                KeyCode::Char('l') => { /* TODO: Open Log Set Modal */ }
                KeyCode::Char('a') => { /* TODO: Open Add Workout Modal */ }
                KeyCode::Char('g') => { /* TODO: Go to Graphs for selected exercise */ }
                KeyCode::Char('h') | KeyCode::Left => self.log_change_date(-1),
                KeyCode::Char('l') | KeyCode::Right => self.log_change_date(1),
                _ => {}
            },
            LogFocus::SetList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => self.log_table_previous(),
                KeyCode::Char('j') | KeyCode::Down => self.log_table_next(),
                KeyCode::Tab => self.log_focus = LogFocus::ExerciseList,
                KeyCode::Char('e') | KeyCode::Enter => { /* TODO: Open Edit Set Modal */ }
                KeyCode::Char('d') | KeyCode::Delete => { /* TODO: Open Delete Confirmation Modal */
                }
                KeyCode::Char('h') | KeyCode::Left => self.log_change_date(-1),
                KeyCode::Char('l') | KeyCode::Right => self.log_change_date(1),
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_history_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO: Implement history input (scrolling, filtering, edit, delete)
        Ok(())
    }

    fn handle_graphs_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO: Implement graph input (changing selections, filtering exercise list)
        Ok(())
    }

    fn handle_bodyweight_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.bw_focus {
            BodyweightFocus::History => match key.code {
                KeyCode::Char('k') | KeyCode::Up => self.bw_table_previous(),
                KeyCode::Char('j') | KeyCode::Down => self.bw_table_next(),
                KeyCode::Char('l') => { /* TODO: Open Log BW Modal */ }
                KeyCode::Char('t') => { /* TODO: Open Target BW Modal */ }
                KeyCode::Char('r') => self.bw_cycle_graph_range(),
                KeyCode::Tab => self.bw_focus = BodyweightFocus::Actions, // Or Graph if interactive
                _ => {}
            },
            BodyweightFocus::Actions => match key.code {
                // Placeholder
                KeyCode::Char('l') => { /* TODO: Open Log BW Modal */ }
                KeyCode::Char('t') => { /* TODO: Open Target BW Modal */ }
                KeyCode::Char('r') => self.bw_cycle_graph_range(),
                KeyCode::Tab => self.bw_focus = BodyweightFocus::History,
                _ => {}
            },
            BodyweightFocus::Graph => match key.code {
                // Placeholder if graph becomes interactive
                KeyCode::Char('r') => self.bw_cycle_graph_range(),
                KeyCode::Tab => self.bw_focus = BodyweightFocus::Actions,
                _ => {}
            },
        }
        Ok(())
    }

    // --- Helper methods for list/table navigation ---

    fn log_list_next(&mut self) {
        let i = match self.log_exercise_list_state.selected() {
            Some(i) => {
                if i >= self.log_exercises_today.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.log_exercises_today.is_empty() {
            self.log_exercise_list_state.select(Some(i));
            // Refresh the sets shown in the right pane
            self.refresh_data_for_active_tab(); // Re-fetch might be needed if data wasn't passed
        }
    }

    fn log_list_previous(&mut self) {
        let i = match self.log_exercise_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.log_exercises_today.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        if !self.log_exercises_today.is_empty() {
            self.log_exercise_list_state.select(Some(i));
            // Refresh the sets shown in the right pane
            self.refresh_data_for_active_tab();
        }
    }

    fn log_table_next(&mut self) {
        let max_index = self.log_sets_for_selected_exercise.len().saturating_sub(1);
        let i = match self.log_set_table_state.selected() {
            Some(i) => {
                if i >= max_index {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.log_sets_for_selected_exercise.is_empty() {
            self.log_set_table_state.select(Some(i));
        }
    }

    fn log_table_previous(&mut self) {
        let max_index = self.log_sets_for_selected_exercise.len().saturating_sub(1);
        let i = match self.log_set_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    max_index
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        if !self.log_sets_for_selected_exercise.is_empty() {
            self.log_set_table_state.select(Some(i));
        }
    }

    fn log_change_date(&mut self, days: i64) {
        if let Some(new_date) = self
            .log_viewed_date
            .checked_add_signed(Duration::days(days))
        {
            // Prevent going too far into the future? Maybe allow it.
            self.log_viewed_date = new_date;
            self.log_exercise_list_state.select(Some(0)); // Reset selection
            self.log_set_table_state.select(Some(0));
            self.refresh_log_data(); // Fetch data for the new date
        }
    }

    fn bw_table_next(&mut self) {
        let max_index = self.bw_history.len().saturating_sub(1);
        let i = match self.bw_history_state.selected() {
            Some(i) => {
                if i >= max_index {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.bw_history.is_empty() {
            self.bw_history_state.select(Some(i));
        }
    }

    fn bw_table_previous(&mut self) {
        let max_index = self.bw_history.len().saturating_sub(1);
        let i = match self.bw_history_state.selected() {
            Some(i) => {
                if i == 0 {
                    max_index
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        if !self.bw_history.is_empty() {
            self.bw_history_state.select(Some(i));
        }
    }

    fn bw_cycle_graph_range(&mut self) {
        self.bw_graph_range_months = match self.bw_graph_range_months {
            1 => 3,
            3 => 6,
            6 => 12,
            12 => 0, // All time
            _ => 1,  // Default back to 1 month
        };
        self.update_bw_graph_data(); // Recalculate graph data for new range
    }
}
