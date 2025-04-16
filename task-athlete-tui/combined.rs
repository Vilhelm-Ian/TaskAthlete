//src/app/actions.rs
// task-athlete-tui/src/app/actions.rs
use super::data::log_change_date;
use super::modals::{
    handle_add_workout_modal_input, handle_confirm_delete_modal_input,
    handle_create_exercise_modal_input, handle_edit_workout_modal_input,
    handle_log_bodyweight_modal_input, handle_set_target_weight_modal_input,
}; // Use specific modal handlers
use super::navigation::{
    bw_table_next, bw_table_previous, log_list_next, log_list_previous, log_table_next,
    log_table_previous,
};
use super::state::{
    ActiveModal, ActiveTab, AddExerciseField, AddWorkoutField, App, BodyweightFocus,
    LogBodyweightField, LogFocus, SetTargetWeightField,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
use task_athlete_lib::{ExerciseDefinition, ExerciseType, Units, Workout, WorkoutFilters};

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
        Ok(())
    }

    fn handle_modal_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.active_modal {
            ActiveModal::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('?') => {
                        self.active_modal = ActiveModal::None;
                    }
                    _ => {} // Ignore other keys in help
                }
            }
            ActiveModal::LogBodyweight { .. } => handle_log_bodyweight_modal_input(self, key)?,
            ActiveModal::SetTargetWeight { .. } => handle_set_target_weight_modal_input(self, key)?,
            ActiveModal::AddWorkout { .. } => handle_add_workout_modal_input(self, key)?,
            ActiveModal::CreateExercise { .. } => handle_create_exercise_modal_input(self, key)?,
            ActiveModal::EditWorkout { .. } => handle_edit_workout_modal_input(self, key)?,
            ActiveModal::ConfirmDeleteWorkout { .. } => {
                handle_confirm_delete_modal_input(self, key)?
            }
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
                KeyCode::Char('a') => self.open_add_workout_modal()?,
                KeyCode::Char('c') => self.open_create_exercise_modal()?, // NEW: Open create modal
                KeyCode::Char('g') => { /* TODO */ }
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                _ => {}
            },
            LogFocus::SetList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_table_previous(self),
                KeyCode::Char('j') | KeyCode::Down => log_table_next(self),
                KeyCode::Tab => self.log_focus = LogFocus::ExerciseList,
                KeyCode::Char('e') | KeyCode::Enter => self.open_edit_workout_modal()?, // EDIT
                KeyCode::Char('d') | KeyCode::Delete => self.open_delete_confirmation_modal()?, // DELETE
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

    fn open_add_workout_modal(&mut self) -> Result<()> {
        let mut initial_exercise_input = String::new();
        let mut initial_sets = "1".to_string();
        let mut initial_reps = String::new();
        let mut initial_weight = String::new();
        let mut initial_duration = String::new();
        let mut initial_distance = String::new();
        let initial_notes = String::new();
        let mut resolved_exercise = None;

        // Fetch all identifiers for suggestions
        let all_identifiers = self.get_all_exercise_identifiers();

        // Try to pre-fill from selected exercise's last entry
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) = self.log_exercises_today.get(selected_index) {
                initial_exercise_input = selected_exercise_name.clone();
                match self
                    .service
                    .resolve_exercise_identifier(selected_exercise_name)
                {
                    Ok(Some(def)) => {
                        let last_workout = self.get_last_or_specific_workout(&def.name, None);
                        self.populate_workout_inputs_from_def_and_last_workout(
                            &def,
                            last_workout,
                            &mut initial_sets,
                            &mut initial_reps,
                            &mut initial_weight,
                            &mut initial_duration,
                            &mut initial_distance,
                        );
                        resolved_exercise = Some(def.clone());
                    }
                    Ok(None) => { /* Handle unlikely case where selected name doesn't resolve */ }
                    Err(e) => {
                        self.set_error(format!("Error resolving exercise: {}", e));
                        // Proceed without pre-filling fields if resolution fails
                    }
                }
            }
        }

        self.active_modal = ActiveModal::AddWorkout {
            exercise_input: initial_exercise_input,
            sets_input: initial_sets,
            reps_input: initial_reps,
            weight_input: initial_weight,
            duration_input: initial_duration,
            distance_input: initial_distance,
            notes_input: initial_notes,
            focused_field: AddWorkoutField::Exercise,
            error_message: None,
            resolved_exercise,
            all_exercise_identifiers: all_identifiers,
            exercise_suggestions: Vec::new(), // Start with empty suggestions ALWAYS
            suggestion_list_state: ListState::default(),
        };
        Ok(())
    }

    // Helper to populate workout fields based on resolved exercise and last workout
    fn populate_workout_inputs_from_def_and_last_workout(
        &self,
        def: &ExerciseDefinition,
        last_workout_opt: Option<Workout>,
        sets_input: &mut String,
        reps_input: &mut String,
        weight_input: &mut String,
        duration_input: &mut String,
        distance_input: &mut String,
        // notes_input: &mut String, // Notes are usually not pre-filled
    ) {
        if let Some(last_workout) = last_workout_opt {
            *sets_input = last_workout.sets.map_or("1".to_string(), |v| v.to_string());
            *reps_input = last_workout.reps.map_or(String::new(), |v| v.to_string());
            *duration_input = last_workout
                .duration_minutes
                .map_or(String::new(), |v| v.to_string());
            // *notes_input = last_workout.notes.clone().unwrap_or_default(); // Optionally prefill notes

            // Weight logic
            if def.type_ == ExerciseType::BodyWeight {
                let bodyweight_used = self.service.config.bodyweight.unwrap_or(0.0);
                let added_weight = last_workout
                    .weight
                    .map_or(0.0, |w| w - bodyweight_used)
                    .max(0.0);
                *weight_input = if added_weight > 0.0 {
                    format!("{:.1}", added_weight)
                } else {
                    String::new() // Clear if only bodyweight was used
                };
            } else {
                *weight_input = last_workout
                    .weight
                    .map_or(String::new(), |v| format!("{:.1}", v));
            }

            // Distance Logic
            if let Some(dist_km) = last_workout.distance {
                let display_dist = match self.service.config.units {
                    Units::Metric => dist_km,
                    Units::Imperial => dist_km * 0.621371,
                };
                *distance_input = format!("{:.1}", display_dist);
            } else {
                *distance_input = String::new(); // Clear distance if not present
            }
        } else {
            // Reset fields if no last workout found for this exercise
            *sets_input = "1".to_string();
            *reps_input = String::new();
            *weight_input = String::new();
            *duration_input = String::new();
            *distance_input = String::new();
            // *notes_input = String::new();
        }
    }

    fn populate_workout_inputs_from_def_and_workout(
        &self,
        def: &ExerciseDefinition,
        workout: &Workout, // The specific workout being edited
        sets_input: &mut String,
        reps_input: &mut String,
        weight_input: &mut String,
        duration_input: &mut String,
        distance_input: &mut String,
        notes_input: &mut String,
    ) {
        *sets_input = workout.sets.map_or("1".to_string(), |v| v.to_string());
        *reps_input = workout.reps.map_or(String::new(), |v| v.to_string());
        *duration_input = workout
            .duration_minutes
            .map_or(String::new(), |v| v.to_string());
        *notes_input = workout.notes.clone().unwrap_or_default();

        // Weight logic (same as before, but applied to the specific workout's weight)
        if def.type_ == ExerciseType::BodyWeight {
            let bodyweight_used = self.service.config.bodyweight.unwrap_or(0.0);
            let added_weight = workout.weight.map_or(0.0, |w| w - bodyweight_used).max(0.0);
            *weight_input = if added_weight > 0.0 {
                format!("{:.1}", added_weight)
            } else {
                String::new()
            };
        } else {
            *weight_input = workout
                .weight
                .map_or(String::new(), |v| format!("{:.1}", v));
        }

        // Distance Logic (same as before)
        if let Some(dist_km) = workout.distance {
            let display_dist = match self.service.config.units {
                Units::Metric => dist_km,
                Units::Imperial => dist_km * 0.621371,
            };
            *distance_input = format!("{:.1}", display_dist);
        } else {
            *distance_input = String::new();
        }
    }

    fn open_edit_workout_modal(&mut self) -> Result<()> {
        let selected_set_index = match self.log_set_table_state.selected() {
            Some(i) => i,
            None => return Ok(()), // No set selected, do nothing
        };

        let workout_to_edit = match self.log_sets_for_selected_exercise.get(selected_set_index) {
            Some(w) => w.clone(),  // Clone to avoid borrow issues
            None => return Ok(()), // Index out of bounds (shouldn't happen)
        };

        let mut sets_input = "1".to_string();
        let mut reps_input = String::new();
        let mut weight_input = String::new();
        let mut duration_input = String::new();
        let mut distance_input = String::new();
        let mut notes_input = String::new();
        let mut resolved_exercise = None;

        // Get definition and *this specific workout's* data for fields
        // We pass the workout_id here to potentially load *that* specific workout if needed,
        // but populate_workout_inputs currently uses the *last* workout for hints.
        // We will override with the actual data below.
        match self.get_data_for_workout_modal(
            &workout_to_edit.exercise_name,
            Some(workout_to_edit.id as u64),
        ) {
            Ok((def, _)) => {
                // We don't need the last_workout here, we have the specific one
                // Populate directly from the workout being edited
                self.populate_workout_inputs_from_def_and_workout(
                    &def,
                    &workout_to_edit, // Use the specific workout
                    &mut sets_input,
                    &mut reps_input,
                    &mut weight_input,
                    &mut duration_input,
                    &mut distance_input,
                    &mut notes_input,
                );
                resolved_exercise = Some(def.clone());
            }
            Err(e) => {
                self.set_error(format!("Error getting exercise details: {}", e));
                return Ok(()); // Don't open modal if we can't get details
            }
        }

        self.active_modal = ActiveModal::EditWorkout {
            workout_id: workout_to_edit.id as u64,
            exercise_name: workout_to_edit.exercise_name.clone(), // Store for display
            sets_input,
            reps_input,
            weight_input,
            duration_input,
            distance_input,
            notes_input,
            focused_field: AddWorkoutField::Sets, // Start focus on Sets (exercise not editable)
            error_message: None,
            resolved_exercise,
        };

        Ok(())
    }

    // NEW: Open Delete Confirmation Modal
    fn open_delete_confirmation_modal(&mut self) -> Result<()> {
        let selected_index = match self.log_set_table_state.selected() {
            Some(i) => i,
            None => return Ok(()), // No set selected
        };

        if let Some(workout) = self.log_sets_for_selected_exercise.get(selected_index) {
            self.active_modal = ActiveModal::ConfirmDeleteWorkout {
                workout_id: workout.id as u64,
                exercise_name: workout.exercise_name.clone(),
                set_index: selected_index + 1, // Display 1-based index
            };
        }

        Ok(())
    }

    pub fn filter_exercise_suggestions(&mut self) {
        if let ActiveModal::AddWorkout {
             ref exercise_input,
             ref all_exercise_identifiers,
             ref mut exercise_suggestions,
             ref mut suggestion_list_state,
             .. // ignore other fields
         } = self.active_modal {
            let input_lower = exercise_input.to_lowercase();
            if input_lower.is_empty() {
                *exercise_suggestions = Vec::new(); // Clear suggestions if input is empty
            } else {
                *exercise_suggestions = all_exercise_identifiers
                    .iter()
                    .filter(|identifier| identifier.to_lowercase().starts_with(&input_lower))
                    .cloned()
                    .take(5) // Limit suggestions to a reasonable number (e.g., 5)
                    .collect();
            }
             // Reset selection when suggestions change
            suggestion_list_state.select(if exercise_suggestions.is_empty() { None } else { Some(0) });
         }
    }

    fn open_create_exercise_modal(&mut self) -> Result<()> {
        self.active_modal = ActiveModal::CreateExercise {
            name_input: String::new(),
            muscles_input: String::new(),
            selected_type: ExerciseType::Resistance, // Default to Resistance
            focused_field: AddExerciseField::Name,   // Start focus on name
            error_message: None,
        };
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

    // Helper specifically for getting the workout being edited
    pub fn get_workout_by_id(&self, workout_id: &str) -> Option<Workout> {
        // We could query the service, but if the workout is already loaded in the log tab,
        // it might be faster to find it there. This assumes the ID is present in the loaded data.
        // This is potentially fragile if the log data isn't comprehensive.
        // A safer approach is to always query the service.
        let filters = WorkoutFilters {
            exercise_name: Some(workout_id),
            ..Default::default()
        };
        match self.service.list_workouts(filters) {
            Ok(mut workouts) if !workouts.is_empty() => workouts.pop(),
            _ => None, // Workout not found or error
        }
        // Alternative: Search in existing log_sets_for_selected_exercise
        // self.log_sets_for_selected_exercise.iter().find(|w| w.id == workout_id).cloned()
    }

    fn get_data_for_workout_modal(
        &mut self,
        exercise_identifier: &str,
        workout_id_for_context: Option<u64>, // Pass Some(id) when editing
    ) -> Result<(ExerciseDefinition, Option<Workout>), anyhow::Error> {
        let def = self
            .service
            .resolve_exercise_identifier(exercise_identifier)?
            .ok_or_else(|| anyhow::anyhow!("Exercise '{}' not found.", exercise_identifier))?;
        let last_workout = self.get_last_or_specific_workout(&def.name, workout_id_for_context);
        Ok((def, last_workout))
    }
}

//src/app/data.rs
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
    pub(crate) fn refresh_log_data(&mut self) {
        // Make crate-public if needed by other app modules
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
    pub(crate) fn update_log_sets_for_selected_exercise(
        &mut self,
        all_workouts_for_date: &[Workout],
    ) {
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) =
                self.log_exercises_today.get(selected_index).cloned()
            {
                self.log_sets_for_selected_exercise = all_workouts_for_date
                    .iter()
                    .filter(|w| w.exercise_name == selected_exercise_name)
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

                self.bw_latest = self.bw_history.first().map(|(_, w)| *w);
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
            self.bw_history
                .last()
                .map(|(ts, _)| ts.date_naive())
                .unwrap_or(now_naive)
        };

        let filtered_data: Vec<_> = self
            .bw_history
            .iter()
            .filter(|(ts, _)| ts.date_naive() >= start_date_filter)
            .rev()
            .collect();

        if filtered_data.is_empty() {
            self.bw_graph_data.clear();
            return;
        }
        let first_day_epoch = filtered_data
            .first()
            .unwrap()
            .0
            .date_naive()
            .num_days_from_ce();
        self.bw_graph_data = filtered_data
            .iter()
            .map(|(date, weight)| {
                let days_since_first = (date.num_days_from_ce() - first_day_epoch) as f64;
                (days_since_first, *weight)
            })
            .collect();

        let first_ts = self.bw_graph_data.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last_ts = self
            .bw_graph_data
            .last()
            .map(|(x, _)| *x)
            .unwrap_or(first_ts + 1.0);
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
        app.log_exercise_list_state
            .select(if app.log_exercises_today.is_empty() {
                None
            } else {
                Some(0)
            });
        app.log_set_table_state
            .select(if app.log_sets_for_selected_exercise.is_empty() {
                None
            } else {
                Some(0)
            });
        // Data will be refreshed by the main loop
    }
}

//src/app/mod.rs
use thiserror::Error;

// Declare the modules within the app directory
pub mod actions;
pub mod data;
pub mod modals;
pub mod navigation;
pub mod state;

// Re-export the main App struct and other necessary types for convenience
pub use state::{
    ActiveModal,
    ActiveTab,
    AddExerciseField,
    AddWorkoutField,
    App,
    BodyweightFocus,
    LogBodyweightField,
    LogFocus,
    SetTargetWeightField, // Add AddExerciseField
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

//src/app/modals.rs
// task-athlete-tui/src/app/modals.rs
use super::state::{
    ActiveModal, AddExerciseField, AddWorkoutField, App, LogBodyweightField, SetTargetWeightField,
};
use super::AppInputError;
use anyhow::{bail, Result};
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::str::FromStr;
use task_athlete_lib::{DbError, ExerciseDefinition, ExerciseType, Units};

// --- Parsing Helpers (moved here) ---

fn parse_optional_int<T: FromStr>(input: &str) -> Result<Option<T>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<T>()
            .map(Some)
            .map_err(|_| {
                AppInputError::InvalidNumber(format!("'{}' is not a valid integer", trimmed))
            })
            .and_then(|opt_val| {
                // Basic validation (can be extended)
                if let Some(val) = opt_val.as_ref() {
                    // Assuming T supports comparison with 0 (like i64)
                    // This requires a bound, maybe add later if T is generic
                    // if *val < 0 { return Err(AppInputError::InvalidNumber("Value cannot be negative".into())) }
                }
                Ok(opt_val)
            })
    }
}

fn parse_optional_float(input: &str) -> Result<Option<f64>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<f64>()
            .map(Some)
            .map_err(|_| {
                AppInputError::InvalidNumber(format!("'{}' is not a valid number", trimmed))
            })
            .and_then(|opt_val| {
                if let Some(val) = opt_val {
                    if val < 0.0 {
                        return Err(AppInputError::InvalidNumber(
                            "Value cannot be negative".into(),
                        ));
                    }
                }
                Ok(opt_val)
            })
    }
}

// Helper to increment/decrement a numeric string field
fn modify_numeric_input<T>(input_str: &mut String, delta: T, min_val: Option<T>, is_float: bool)
where
    T: FromStr
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy
        + std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Debug,
{
    // let current_val = if is_float {
    //     input_str.parse::<f64>().ok()
    // } else {
    //     input_str.parse::<i64>().ok()
    // };

    let mut num_val: T = match input_str.parse::<T>() {
        Ok(v) => v,
        Err(_) => return, // Cannot parse, do nothing
    };

    num_val = num_val + delta; // Apply delta

    // Apply minimum value constraint
    if let Some(min) = min_val {
        if num_val < min {
            num_val = min;
        }
    }

    // Update the string
    if is_float {
        *input_str = format!("{:.1}", num_val); // Format floats nicely
    } else {
        *input_str = num_val.to_string();
    }
}

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

fn submit_add_workout(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::AddWorkout {
         exercise_input: _, // Use resolved_exercise name
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         resolved_exercise, // Use the stored resolved exercise
         .. // ignore focused_field, error_message, suggestions etc.
     } = modal_state {

        // 1. Validate Exercise Selection
        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             // This error should ideally be prevented by the input handler (not allowing Tab/Enter without resolution)
             AppInputError::DbError("Exercise not resolved. Select a valid exercise.".to_string())
        })?;
        let canonical_name = &exercise_def.name; // Already resolved

        // 2. Parse numeric inputs
        let sets = parse_optional_int::<i64>(sets_input)?;
        let reps = parse_optional_int::<i64>(reps_input)?;
        let weight_arg = parse_optional_float(weight_input)?; // This is the value from the input field
        let duration = parse_optional_int::<i64>(duration_input)?;
        let distance_arg = parse_optional_float(distance_input)?; // Value from input field

        // 3. Notes
        let notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // 4. Bodyweight & Units (Service layer handles this based on type and config)
        let bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight // Pass the configured bodyweight
        } else {
            None
        };


        // 5. Call AppService
        match app.service.add_workout(
            canonical_name,
            app.log_viewed_date, // Use the date currently viewed in the log tab
            sets,
            reps,
            weight_arg, // Pass the weight from the input field
            duration,
            distance_arg, // Pass the distance from the input field
            notes,
            None, // No implicit type needed (already resolved)
            None, // No implicit muscles needed (already resolved)
            bodyweight_to_use, // Pass configured bodyweight if needed
        ) {
            Ok((_workout_id, pb_info)) => {
                 if let Some(pb) = pb_info {
                    // Simple message if any PB achieved
                    if pb.any_pb() {
                         // Using set_error might be confusing, maybe a different status method?
                         // For now, use set_error for feedback.
                         app.set_error("🎉 New Personal Best achieved!".to_string());
                    }
                 }
                Ok(()) // Signal success to close modal
            }
            Err(e) => {
                 // Convert service error to modal error
                 if let Some(db_err) = e.downcast_ref::<DbError>() {
                     Err(AppInputError::DbError(db_err.to_string()))
                 } else if let Some(cfg_err) = e.downcast_ref::<task_athlete_lib::ConfigError>() {
                      Err(AppInputError::DbError(cfg_err.to_string())) // Use DbError variant for simplicity
                 }
                 else {
                     Err(AppInputError::DbError(format!("Error adding workout: {}", e)))
                 }
            }
        }

     } else {
         // Should not happen if called correctly
         Err(AppInputError::DbError("Internal error: Invalid modal state for add workout".to_string()))
     }
}

fn submit_edit_workout(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::EditWorkout {
         workout_id,
         // exercise_name is not submitted for change here
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         resolved_exercise, // Needed for type context (bodyweight)
         .. // ignore focused_field, error_message
     } = modal_state {

        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             AppInputError::DbError("Internal error: Exercise context missing for edit.".to_string())
        })?;

        // Parse inputs (reuse existing helpers)
        let sets = parse_optional_int(sets_input)?;
        let reps = parse_optional_int(reps_input)?;
        let weight_arg = parse_optional_float(weight_input)?;
        let duration = parse_optional_int::<i64>(duration_input)?;
        let distance_arg = parse_optional_float(distance_input)?;
        let notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // Bodyweight & Units handled by service layer
        let bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight
        } else { None };

        // Call AppService's edit_workout (assuming its signature)
        // Adjust the signature call based on your actual AppService::edit_workout method
        match app.service.edit_workout(
            *workout_id as i64, None, sets, reps, weight_arg, duration, distance_arg, notes, None
        ) {
            Ok(_) => Ok(()), // Success
            Err(e) => {
                Err(AppInputError::DbError(format!("Error editing workout: {}", e)))
            }
        }
    } else {
        Err(AppInputError::DbError("Internal error: Invalid modal state for edit workout".to_string()))
    }
}

fn submit_delete_workout_set(app: &mut App, workout_id: u64) -> Result<(), AppInputError> {
    match app.service.delete_workouts(&vec![workout_id as i64]) {
        Ok(_) => {
            // Adjust selection after deletion if necessary
            if let Some(selected_index) = app.log_set_table_state.selected() {
                if selected_index >= app.log_sets_for_selected_exercise.len().saturating_sub(1) {
                    // Adjust if last item deleted
                    let new_index = app.log_sets_for_selected_exercise.len().saturating_sub(2); // Select new last item
                    app.log_set_table_state.select(
                        if new_index > 0 || app.log_sets_for_selected_exercise.len() == 1 {
                            Some(new_index)
                        } else {
                            None
                        },
                    );
                }
            }
            Ok(())
        }
        Err(e) => Err(AppInputError::DbError(format!(
            "Error deleting workout: {}",
            e
        ))),
    }
}

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

fn submit_create_exercise(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
        // ignore focused_field, error_message
        ..
    } = modal_state
    {
        let trimmed_name = name_input.trim();
        if trimmed_name.is_empty() {
            return Err(AppInputError::ExerciseNameEmpty);
        }

        let muscles_opt = if muscles_input.trim().is_empty() {
            None
        } else {
            Some(muscles_input.trim())
        };

        // Call AppService to create the exercise
        match app
            .service
            .create_exercise(trimmed_name, *selected_type, muscles_opt)
        {
            Ok(_) => Ok(()), // Signal success to close modal
            Err(e) => {
                // Convert service error to modal error
                if let Some(db_err) = e.downcast_ref::<DbError>() {
                    // Handle specific unique constraint error
                    if let DbError::ExerciseNameNotUnique(name) = db_err {
                        return Err(AppInputError::DbError(format!(
                            "Exercise '{}' already exists.",
                            name
                        )));
                    }
                    Err(AppInputError::DbError(db_err.to_string()))
                } else {
                    Err(AppInputError::DbError(format!(
                        "Error creating exercise: {}",
                        e
                    )))
                }
            }
        }
    } else {
        // Should not happen if called correctly
        Err(AppInputError::DbError(
            "Internal error: Invalid modal state for create exercise".to_string(),
        ))
    }
}

// --- Input Handling ---
pub fn handle_edit_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;

    if let ActiveModal::EditWorkout {
        // workout_id and exercise_name are not directly modified by input
        ref mut sets_input,
        ref mut reps_input,
        ref mut weight_input,
        ref mut duration_input,
        ref mut distance_input,
        ref mut notes_input,
        ref mut focused_field,
        ref mut error_message,
        // resolved_exercise is needed for context but not directly edited
        ..
    } = app.active_modal
    {
        *error_message = None; // Clear error on input

        // Handle Shift+Tab for reverse navigation (simplified for edit modal)
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            match *focused_field {
                AddWorkoutField::Sets => *focused_field = AddWorkoutField::Cancel, // Wrap around up
                AddWorkoutField::Reps => *focused_field = AddWorkoutField::Sets,
                AddWorkoutField::Weight => *focused_field = AddWorkoutField::Reps,
                AddWorkoutField::Duration => *focused_field = AddWorkoutField::Weight,
                AddWorkoutField::Distance => *focused_field = AddWorkoutField::Duration,
                AddWorkoutField::Notes => *focused_field = AddWorkoutField::Distance,
                AddWorkoutField::Confirm => *focused_field = AddWorkoutField::Notes,
                AddWorkoutField::Cancel => *focused_field = AddWorkoutField::Confirm,
                _ => {} // Ignore Exercise/Suggestions fields
            }
        } else {
            // --- Handle other fields (Sets, Reps, etc.) ---
            // Reuse AddWorkoutField enum, but skip Exercise/Suggestions focus states
            match *focused_field {
                // Skip Exercise and Suggestions fields
                AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                    *focused_field = AddWorkoutField::Sets; // Should not be focusable, move to Sets
                }
                AddWorkoutField::Sets => {
                    match key.code {
                        KeyCode::Char(c) if c.is_digit(10) => sets_input.push(c),
                        KeyCode::Backspace => {
                            sets_input.pop();
                        }
                        KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                        KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                        KeyCode::Enter | KeyCode::Tab => {
                            *focused_field = AddWorkoutField::Reps;
                        }
                        KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Cancel;
                        } // Defined above
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Cancel;
                        } // Simple Up goes to Cancel
                        KeyCode::Down => {
                            *focused_field = AddWorkoutField::Reps;
                        } // Simple Down goes forward
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Reps => match key.code {
                    KeyCode::Char(c) if c.is_digit(10) => reps_input.push(c),
                    KeyCode::Backspace => {
                        reps_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Sets;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Sets;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Weight => match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                    KeyCode::Backspace => {
                        weight_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true),
                    KeyCode::Down => {
                        modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Reps;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Reps;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Duration => match key.code {
                    KeyCode::Char(c) if c.is_digit(10) => duration_input.push(c),
                    KeyCode::Backspace => {
                        duration_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Distance => match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                    KeyCode::Backspace => {
                        distance_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true),
                    KeyCode::Down => {
                        modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Notes;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Notes;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Notes => match key.code {
                    KeyCode::Char(c) => notes_input.push(c),
                    KeyCode::Backspace => {
                        notes_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Confirm;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Confirm;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Confirm => {
                    match key.code {
                        KeyCode::Enter => should_submit = true,
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Cancel;
                        }
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Notes;
                        }
                        KeyCode::Down | KeyCode::Tab | KeyCode::Right => {
                            *focused_field = AddWorkoutField::Cancel;
                        } // Wrap around
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Cancel => {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Tab => {
                            *focused_field = AddWorkoutField::Confirm;
                        }
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Confirm;
                        }
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Notes;
                        }
                        KeyCode::Down => {
                            *focused_field = AddWorkoutField::Sets;
                        } // Wrap around down to Sets
                        _ => {}
                    }
                }
            }
        }
    } // End mutable borrow of app.active_modal

    // --- Submission Logic ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::EditWorkout { .. } = modal_state_clone {
            submission_result = submit_edit_workout(app, &modal_state_clone);
        } else {
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly".to_string(),
            ));
        }

        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
        } else {
            // Re-borrow to set error
            if let ActiveModal::EditWorkout {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}

pub fn handle_confirm_delete_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut should_delete = false;
    let mut workout_id_to_delete: u64 = 0; // Placeholder

    if let ActiveModal::ConfirmDeleteWorkout { workout_id, .. } = &app.active_modal {
        workout_id_to_delete = *workout_id; // Capture the ID
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                should_delete = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Backspace => {
                app.active_modal = ActiveModal::None; // Close modal, do nothing
                return Ok(());
            }
            _ => {} // Ignore other keys
        }
    }

    if should_delete {
        let delete_result = submit_delete_workout_set(app, workout_id_to_delete);
        if delete_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
        } else {
            // If delete fails, show error in status bar (modal is already closed or will be replaced)
            // Or, we could potentially transition to an Error modal, but status bar is simpler.
            app.set_error(delete_result.unwrap_err().to_string());
            app.active_modal = ActiveModal::None; // Close the confirmation modal even on error
        }
    }

    Ok(())
}

pub fn handle_add_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;
    let mut needs_suggestion_update = false;
    // Flag to indicate that workout fields should be repopulated
    let mut repopulate_fields_for_resolved_exercise: Option<ExerciseDefinition> = None;

    if let ActiveModal::AddWorkout {
        ref mut exercise_input,
        ref mut sets_input,
        ref mut reps_input,
        ref mut weight_input,
        ref mut duration_input,
        ref mut distance_input,
        ref mut notes_input,
        ref mut focused_field,
        ref mut error_message,
        ref mut resolved_exercise,
        ref mut exercise_suggestions,
        ref mut suggestion_list_state,
        .. // ignore all_exercise_identifiers
    } = app.active_modal
    {
        *error_message = None; // Clear error on most inputs
        let mut focus_changed = false;

        // --- Main Input Handling Logic ---
        match *focused_field {
            AddWorkoutField::Exercise => match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None; // Invalidate resolution
                    needs_suggestion_update = true; // Filter suggestions after borrow
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None; // Invalidate resolution
                    needs_suggestion_update = true; // Filter suggestions after borrow
                }
                KeyCode::Down => {
                    if !exercise_suggestions.is_empty() {
                        *focused_field = AddWorkoutField::Suggestions;
                        suggestion_list_state.select(Some(0));
                        focus_changed = true;
                    } else {
                        // No suggestions, behave like Tab (go to Sets)
                        // Attempt to resolve before moving
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone();
                                if resolved_exercise.as_ref() != Some(&def) { // Check if it changed
                                    repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets;
                                focus_changed = true;
                                *exercise_suggestions = Vec::new(); // Clear suggestions
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => {
                                *error_message = Some(format!("Exercise '{}' not found.", exercise_input));
                                // Optionally clear fields if resolution fails? Maybe not.
                            }
                            Err(e) => *error_message = Some(format!("Error: {}", e)),
                        }
                    }
                }
                 KeyCode::Tab => {
                    // Attempt to resolve current input before moving
                    if resolved_exercise.is_none() && !exercise_input.is_empty() {
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone(); // Update input to canonical name
                                if resolved_exercise.as_ref() != Some(&def) { // Check if it *really* changed
                                    repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets;
                                focus_changed = true;
                                *exercise_suggestions = Vec::new(); // Clear suggestions after resolving/moving away
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => {
                                *error_message = Some(format!("Exercise '{}' not found. Cannot move.", exercise_input));
                                // Optionally clear fields if resolution fails? Maybe not.
                            } // Stay if not resolved
                            Err(e) => {
                                *error_message = Some(format!("Error resolving: {}. Cannot move.", e));
                            } // Stay if error
                        }
                    } else { // Move if already resolved or empty
                        *focused_field = AddWorkoutField::Sets;
                        focus_changed = true;
                        *exercise_suggestions = Vec::new(); // Clear suggestions
                        suggestion_list_state.select(None);
                    }
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Cancel;
                    focus_changed = true;
                    *exercise_suggestions = Vec::new();
                    suggestion_list_state.select(None);
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },

            AddWorkoutField::Suggestions => match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                KeyCode::Up => {
                     if !exercise_suggestions.is_empty() {
                        let current_selection = suggestion_list_state.selected().unwrap_or(0);
                        let new_selection = if current_selection == 0 {
                            exercise_suggestions.len() - 1
                        } else {
                            current_selection - 1
                        };
                        suggestion_list_state.select(Some(new_selection));
                    }
                }
                KeyCode::Down => {
                    if !exercise_suggestions.is_empty() {
                        let current_selection = suggestion_list_state.selected().unwrap_or(0);
                        let new_selection = if current_selection >= exercise_suggestions.len() - 1 {
                            0
                        } else {
                            current_selection + 1
                        };
                        suggestion_list_state.select(Some(new_selection));
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected_index) = suggestion_list_state.selected() {
                        if let Some(selected_suggestion) = exercise_suggestions.get(selected_index) {
                            match app.service.resolve_exercise_identifier(selected_suggestion) {
                                Ok(Some(def)) => {
                                    *exercise_input = def.name.clone();
                                    if resolved_exercise.as_ref() != Some(&def) { // Check if it changed
                                        repopulate_fields_for_resolved_exercise = Some(def.clone());
                                    }
                                    *resolved_exercise = Some(def);
                                    *focused_field = AddWorkoutField::Sets;
                                    focus_changed = true;
                                    *exercise_suggestions = Vec::new(); // Clear suggestions after selection
                                    suggestion_list_state.select(None);
                                }
                                Ok(None) => {
                                    *error_message = Some(format!("Could not resolve selected '{}'.", selected_suggestion));
                                    *focused_field = AddWorkoutField::Exercise;
                                    focus_changed = true;
                                    // Do not clear suggestions if resolution failed
                                }
                                Err(e) => {
                                    *error_message = Some(format!("Error resolving selected: {}", e));
                                    *focused_field = AddWorkoutField::Exercise;
                                    focus_changed = true;
                                    // Do not clear suggestions if resolution failed
                                }
                            }
                        }
                    } else {
                         // If somehow Enter hit with no selection, try resolving current input
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone();
                                if resolved_exercise.as_ref() != Some(&def) {
                                     repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets; // Move to next field
                                focus_changed = true;
                                *exercise_suggestions = Vec::new();
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => { // Enter pressed but input not resolvable -> back to input
                                *focused_field = AddWorkoutField::Exercise;
                                focus_changed = true;
                            }
                             Err(e) => { // Error resolving -> back to input
                                *error_message = Some(format!("Error resolving input: {}", e));
                                *focused_field = AddWorkoutField::Exercise;
                                focus_changed = true;
                            }
                        }
                    }
                }
                KeyCode::Tab | KeyCode::Esc => {
                    // Exit suggestion list back to input field
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                    // Keep suggestions visible for now when going back via Esc/Tab
                }
                _ => {}
            },

             // --- Handle other fields (Sets, Reps, etc.) ---
             // Common pattern: Clear suggestions and move focus
             AddWorkoutField::Sets => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None); // Clear suggestions
                match key.code {
                    KeyCode::Char(c) if c.is_digit(10) => sets_input.push(c),
                    KeyCode::Backspace => { sets_input.pop(); }
                    KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                    KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    // Shift+Tab for reverse navigation
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; } // Simple Up goes back
                    KeyCode::Down => { *focused_field = AddWorkoutField::Reps; focus_changed = true; } // Simple Down goes forward
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Reps => {
                 *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                 match key.code {
                     KeyCode::Char(c) if c.is_digit(10) => reps_input.push(c),
                     KeyCode::Backspace => { reps_input.pop(); }
                     KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                     KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                     KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                     KeyCode::BackTab => { *focused_field = AddWorkoutField::Sets; focus_changed = true; }
                     KeyCode::Up => { *focused_field = AddWorkoutField::Sets; focus_changed = true; }
                     KeyCode::Down => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                 }
            }
            AddWorkoutField::Weight => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                    KeyCode::Backspace => { weight_input.pop(); }
                    KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true),
                    KeyCode::Down => modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Duration => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if c.is_digit(10) => duration_input.push(c),
                    KeyCode::Backspace => { duration_input.pop(); }
                    KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Distance => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                    KeyCode::Backspace => { distance_input.pop(); }
                    KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true),
                    KeyCode::Down => modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Notes => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) => notes_input.push(c),
                    KeyCode::Backspace => { notes_input.pop(); }
                    // Treat Enter like Tab for Notes
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; }
                     KeyCode::BackTab => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; } // Go down to Confirm
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Confirm => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Enter => {
                        // Final validation before submit?
                         if resolved_exercise.is_none() {
                            *error_message = Some("Cannot submit: Exercise not resolved.".to_string());
                            *focused_field = AddWorkoutField::Exercise; // Send user back to fix it
                         } else {
                            should_submit = true;
                         }
                    }
                    KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => { *focused_field = AddWorkoutField::Cancel; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Down | KeyCode::Tab | KeyCode::Right => { *focused_field = AddWorkoutField::Cancel; focus_changed = true; } // Wrap around
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Cancel => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    KeyCode::Right | KeyCode::Tab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; }
                    KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; } // Cycle left from Cancel goes to Confirm
                    KeyCode::Up => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; } // Wrap around down
                    _ => {}
                }
            }
        }

        // If focus moved away from Exercise field and it wasn't resolved, try to resolve now.
        // This is a fallback, main resolution happens on Tab/Enter/Suggestion select.
        if focus_changed
            && *focused_field != AddWorkoutField::Exercise
            && *focused_field != AddWorkoutField::Suggestions
            && resolved_exercise.is_none()
            && !exercise_input.is_empty() // Only try if there's input
        {
            match app.service.resolve_exercise_identifier(exercise_input) {
                Ok(Some(def)) => {
                    *exercise_input = def.name.clone(); // Update input field too
                    if resolved_exercise.as_ref() != Some(&def) { // Check if changed
                         repopulate_fields_for_resolved_exercise = Some(def.clone());
                    }
                    *resolved_exercise = Some(def);
                }
                 Ok(None) => {
                     // Allow moving away, but show error if resolution failed?
                     // Or maybe just clear the resolved_exercise state?
                     // Let's clear it and maybe show a warning if they try to submit.
                     *resolved_exercise = None;
                     // Optional: *error_message = Some(format!("Warning: Exercise '{}' not resolved.", exercise_input));
                 }
                Err(e) => {
                    *resolved_exercise = None; // Clear on error too
                    *error_message = Some(format!("Error resolving '{}': {}", exercise_input, e));
                }
            }
        }

    } // End mutable borrow of app.active_modal

    // --- Repopulate Fields (Deferred until borrow ends) ---
    if let Some(def_to_repopulate) = repopulate_fields_for_resolved_exercise {
        // Re-borrow mutably to update fields
        let last_workout = app.get_last_or_specific_workout(&def_to_repopulate.name, None);
        if let ActiveModal::AddWorkout {
            ref mut sets_input,
            ref mut reps_input,
            ref mut weight_input,
            ref mut duration_input,
            ref mut distance_input,
            // notes_input not typically repopulated
            ..
        } = app.active_modal
        {
            if let Some(workout) = last_workout {
                *sets_input = parse_option_to_input(workout.sets);
                *reps_input = parse_option_to_input(workout.reps);
                *weight_input = parse_option_to_input(workout.weight);
                *distance_input = parse_option_to_input(workout.distance);
                *duration_input = parse_option_to_input(workout.duration_minutes);
            }
        }
    }

    // --- Filter suggestions (Deferred until borrow ends) ---
    if needs_suggestion_update {
        app.filter_exercise_suggestions();
    }

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        // Clone the state *before* calling submit, as submit needs immutable borrow
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::AddWorkout { .. } = modal_state_clone {
            submission_result = submit_add_workout(app, &modal_state_clone);
        } else {
            // This case should be rare due to the check within the Confirm handler
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly before submit".to_string(),
            ));
        }

        // --- Handle Submission Result ---
        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Data refresh will happen in the main loop
        } else {
            // Re-borrow mutably ONLY if necessary to set error
            if let ActiveModal::AddWorkout {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
                // Optionally, set focus back to a relevant field? e.g., Exercise if that was the issue?
                // Or just keep focus where it was (Confirm button).
            }
        }
    }

    Ok(())
}

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

pub fn handle_create_exercise_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;
    let mut focus_changed = false; // To potentially trigger re-renders if needed

    if let ActiveModal::CreateExercise {
        ref mut name_input,
        ref mut muscles_input,
        ref mut selected_type,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;

        // Handle Shift+Tab for reverse navigation
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            match *focused_field {
                AddExerciseField::Name => *focused_field = AddExerciseField::Cancel,
                AddExerciseField::Muscles => *focused_field = AddExerciseField::Name,
                AddExerciseField::TypeResistance => *focused_field = AddExerciseField::Muscles,
                AddExerciseField::TypeCardio => *focused_field = AddExerciseField::TypeResistance,
                AddExerciseField::TypeBodyweight => *focused_field = AddExerciseField::TypeCardio,
                AddExerciseField::Confirm => *focused_field = AddExerciseField::TypeBodyweight,
                AddExerciseField::Cancel => *focused_field = AddExerciseField::Confirm,
            }
            focus_changed = true;
        } else {
            // Handle normal key presses
            match *focused_field {
                AddExerciseField::Name => match key.code {
                    KeyCode::Char(c) => name_input.push(c),
                    KeyCode::Backspace => {
                        name_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddExerciseField::Muscles;
                        focus_changed = true;
                    }
                    KeyCode::Up => *focused_field = AddExerciseField::Cancel, // Wrap around up
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::Muscles => match key.code {
                    KeyCode::Char(c) => muscles_input.push(c),
                    KeyCode::Backspace => {
                        muscles_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddExerciseField::TypeResistance; // Move to first type
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Name;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Type Selection Fields ---
                AddExerciseField::TypeResistance => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::Resistance, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::TypeCardio;
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        // Wrap around left (or could go to Muscles - Tab/Shift-Tab is better)
                        *focused_field = AddExerciseField::Cancel;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeCardio => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::Cardio, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        *focused_field = AddExerciseField::TypeResistance;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles; // Jump back to Muscles
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeBodyweight => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::BodyWeight, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Confirm; // Move to confirm
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        *focused_field = AddExerciseField::TypeCardio;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles; // Jump back to Muscles
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Button Fields ---
                AddExerciseField::Confirm => match key.code {
                    KeyCode::Enter => {
                        should_submit = true;
                    }
                    KeyCode::Left | KeyCode::Backspace => {
                        *focused_field = AddExerciseField::Cancel;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the types section
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Cancel; // Cycle behavior
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::Cancel => match key.code {
                    KeyCode::Enter | KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    KeyCode::Right => {
                        *focused_field = AddExerciseField::Confirm;
                        focus_changed = true;
                    }
                    KeyCode::Left | KeyCode::Backspace => {
                        *focused_field = AddExerciseField::Confirm; // Cycle behavior
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the types section
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Name; // Wrap around to top
                        focus_changed = true;
                    }
                    _ => {}
                },
            }
        }
    } // End mutable borrow of app.active_modal

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::CreateExercise { .. } = modal_state_clone {
            submission_result = submit_create_exercise(app, &modal_state_clone);
        // Pass the clone
        } else {
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly".to_string(),
            ));
        }

        // --- Handle Submission Result ---
        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Refresh handled by main loop
        } else {
            // Submission failed, re-borrow mutably ONLY if necessary to set error
            if let ActiveModal::CreateExercise {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}

fn parse_option_to_input<T>(option: Option<T>) -> String
where
    T: std::fmt::Display,
{
    if let Some(s) = option {
        format!("{}", s)
    } else {
        String::new()
    }
}

//src/app/navigation.rs
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

//src/app/state.rs
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
    Suggestions,
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
        all_exercise_identifiers: Vec<String>,
        // Holds the currently filtered suggestions based on input
        exercise_suggestions: Vec<String>,
        // State for navigating the suggestion list
        suggestion_list_state: ListState,
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
    EditWorkout {
        workout_id: u64,       // ID of the workout being edited
        exercise_name: String, // Display only, non-editable in this modal
        sets_input: String,
        reps_input: String,
        weight_input: String,
        duration_input: String,
        distance_input: String,
        notes_input: String,
        focused_field: AddWorkoutField, // Reuse AddWorkoutField for focus, minus Exercise/Suggestions
        error_message: Option<String>,
        // Store the definition for context (e.g., bodyweight type)
        // This is technically redundant if exercise_name is fixed,
        // but useful for consistency and potential future enhancements.
        // Store the resolved definition temporarily after user leaves exercise field
        resolved_exercise: Option<ExerciseDefinition>,
    },
    ConfirmDeleteWorkout {
        workout_id: u64,
        exercise_name: String,
        set_index: usize, // For display purposes ("Delete set X of Y?")
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
            active_modal: ActiveModal::None, // Initialize with None
            // ... other fields ...
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
    pub fn get_last_or_specific_workout(
        &self,
        canonical_name: &str,
        id: Option<u64>,
    ) -> Option<Workout> {
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

    pub fn get_all_exercise_identifiers(&self) -> Vec<String> {
        let mut identifiers = Vec::new();
        // Add exercise names
        if let Ok(exercises) = self.service.list_exercises(None, None) {
            identifiers.extend(exercises.into_iter().map(|e| e.name));
        }
        // Add aliases
        if let Ok(aliases) = self.service.list_aliases() {
            identifiers.extend(aliases.into_keys());
        }
        identifiers.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        identifiers.dedup_by(|a, b| a.eq_ignore_ascii_case(b)); // Remove duplicates (like name matching alias)
        identifiers
    }
}

//src/main.rs
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

//src/ui/bodyweight_tab.rs
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
        Some((date, w)) => {
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

    let rows = app.bw_history.iter().map(|(date, weight_kg)| {
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

//src/ui/layout.rs
// task-athlete-tui/src/ui/layout.rs
use crate::{
    app::{ActiveTab, App}, // Use App from crate::app
    ui::{
        // Use sibling UI modules
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
    let content_area = area.inner(&ratatui::layout::Margin {
        vertical: 0,
        horizontal: 0,
    });

    match app.active_tab {
        ActiveTab::Log => render_log_tab(f, app, content_area),
        ActiveTab::History => render_placeholder(f, "History Tab", content_area),
        ActiveTab::Graphs => render_placeholder(f, "Graphs Tab", content_area),
        ActiveTab::Bodyweight => render_bodyweight_tab(f, app, content_area),
    }
}

/// Helper function to create a centered rectangle with fixed dimensions.
/// Ensures the dimensions do not exceed the available screen size `r`.
pub fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    // Clamp dimensions to the screen size
    let clamped_width = width.min(r.width);
    let clamped_height = height.min(r.height);

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Calculate margins to center the fixed height
            Constraint::Length(r.height.saturating_sub(clamped_height) / 2),
            Constraint::Length(clamped_height), // Use the clamped fixed height
            Constraint::Length(r.height.saturating_sub(clamped_height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            // Calculate margins to center the fixed width
            Constraint::Length(r.width.saturating_sub(clamped_width) / 2),
            Constraint::Length(clamped_width), // Use the clamped fixed width
            Constraint::Length(r.width.saturating_sub(clamped_width) / 2),
        ])
        .split(popup_layout[1])[1] // Take the middle chunk
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

//src/ui/log_tab.rs
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

//src/ui/mod.rs
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

//src/ui/modals.rs
// task-athlete-tui/src/ui/modals.rs
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField, LogBodyweightField, SetTargetWeightField},
        AddExerciseField, App,
    }, // Use App from crate::app
    ui::layout::centered_rect, // Use centered_rect from layout
    ui::layout::centered_rect_fixed,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::{ExerciseType, Units};

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f), // Don't need app state for help text
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
        ActiveModal::AddWorkout { .. } => render_add_workout_modal(f, app),
        ActiveModal::CreateExercise { .. } => render_create_exercise_modal(f, app),
        ActiveModal::EditWorkout { .. } => render_edit_workout_modal(f, app),
        ActiveModal::ConfirmDeleteWorkout { .. } => render_confirmation_modal(f, app),
        ActiveModal::None => {} // Should not happen if called correctly
    }
}

fn render_help_modal(f: &mut Frame) {
    // Removed unused `_app`
    let block = Block::default()
        .title("Help (?)")
        .borders(Borders::ALL)
        .title_style(Style::new().bold())
        .border_style(Style::new().yellow());
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
        Line::from(Span::styled(
            " Press Esc, ?, or Enter to close ",
            Style::new().italic().yellow(),
        )),
    ];

    let paragraph = Paragraph::new(help_text).wrap(Wrap { trim: false });
    f.render_widget(
        paragraph,
        area.inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}

fn render_log_bodyweight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::LogBodyweight {
        weight_input,
        date_input,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let block = Block::default()
            .title("Log New Bodyweight")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());
        let area = centered_rect(60, 11, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        // Get the inner area *after* the block's margin/border
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            // No margin here, use inner_area directly
            .constraints([
                Constraint::Length(1), // Weight label
                Constraint::Length(1), // Weight input
                Constraint::Length(1), // Date label
                Constraint::Length(1), // Date input
                Constraint::Length(1), // Spacer/Buttons row
                Constraint::Length(1), // Error Message (if any) - adjusted constraints
                Constraint::Min(0),    // Remaining space (might not be needed)
            ])
            .split(inner_area); // Split the inner_area

        f.render_widget(
            Paragraph::new(format!("Weight ({}):", weight_unit)),
            chunks[0],
        );
        f.render_widget(Paragraph::new("Date (YYYY-MM-DD / today):"), chunks[2]);

        // --- Input Field Rendering with Padding ---
        let base_input_style = Style::default().fg(Color::White); // Or another visible color

        // Weight Input
        let weight_input_area = chunks[1]; // Area for the whole line
                                           // Create a padded area *within* this line for the text itself
        let weight_text_area = weight_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let weight_style = if *focused_field == LogBodyweightField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        // Render the paragraph within the padded text_area
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_text_area,
        );

        // Date Input
        let date_input_area = chunks[3]; // Area for the whole line
                                         // Create a padded area *within* this line for the text itself
        let date_text_area = date_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let date_style = if *focused_field == LogBodyweightField::Date {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        // Render the paragraph within the padded text_area
        f.render_widget(
            Paragraph::new(date_input.as_str()).style(date_style),
            date_text_area,
        );
        // --- End Input Field Rendering ---

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[4]); // Buttons in chunk 4

        let base_button_style = Style::default().fg(Color::White);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[5],
            ); // Error in chunk 5
        }

        // --- Cursor Positioning (using padded areas) ---
        match focused_field {
            LogBodyweightField::Weight => {
                // Calculate cursor position relative to the padded weight_text_area
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1)); // Clamp to padded area
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            LogBodyweightField::Date => {
                // Calculate cursor position relative to the padded date_text_area
                let cursor_x = (date_text_area.x + date_input.chars().count() as u16)
                    .min(date_text_area.right().saturating_sub(1)); // Clamp to padded area
                f.set_cursor(cursor_x, date_text_area.y);
            }
            _ => {}
        }
        // --- End Cursor Positioning ---
    }
}

fn render_add_workout_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::AddWorkout {
        exercise_input,
        sets_input,
        reps_input,
        weight_input,
        duration_input,
        distance_input,
        notes_input,
        focused_field,
        error_message,
        resolved_exercise,
        exercise_suggestions, // Get suggestions
        suggestion_list_state, // Get list state
                              // all_exercise_identifiers is not needed for rendering
        ..
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Add New Workout Entry")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        // --- Calculate required height (including potential suggestions) ---
        let mut required_height = 2; // Borders/Padding
        required_height += 1; // Exercise label
        required_height += 1; // Exercise input
        required_height += 1; // Sets/Reps labels
        required_height += 1; // Sets/Reps inputs
        required_height += 1; // Weight/Duration labels
        required_height += 1; // Weight/Duration inputs
        required_height += 1; // Distance label
        required_height += 1; // Distance input
        required_height += 1; // Notes label
        required_height += 3; // Notes input (multi-line)
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error Message
        }
        // Note: We don't add suggestion height here, we'll draw it as a popup *over* other content

        let fixed_width = 80; // Keep width fixed
        let area = centered_rect_fixed(fixed_width, required_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        // Define constraints without suggestions initially
        let mut constraints = vec![
            Constraint::Length(1), // Exercise label
            Constraint::Length(1), // Exercise input
            Constraint::Length(1), // Sets/Reps labels
            Constraint::Length(1), // Sets/Reps inputs
            Constraint::Length(1), // Weight/Duration labels
            Constraint::Length(1), // Weight/Duration inputs
            Constraint::Length(1), // Distance label
            Constraint::Length(1), // Distance input
            Constraint::Length(1), // Notes label
            Constraint::Length(3), // Notes input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0)); // Remainder

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        // --- Render Main Modal Content (Mostly Unchanged) ---
        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };

        // Row 1: Exercise Input
        f.render_widget(Paragraph::new("Exercise Name/Alias:"), chunks[0]);
        let ex_style = if *focused_field == AddWorkoutField::Exercise
            || *focused_field == AddWorkoutField::Suggestions
        {
            // Highlight input if suggestions are focused too
            base_input_style.reversed()
        } else {
            base_input_style
        };
        let exercise_input_area = chunks[1].inner(&input_margin);
        f.render_widget(
            Paragraph::new(exercise_input.as_str()).style(ex_style),
            exercise_input_area,
        );

        // ... (Render Sets/Reps, Weight/Duration, Distance, Notes, Buttons, Error - unchanged logic, adjust chunk indices if error exists) ...
        let error_chunk_index = if error_message.is_some() {
            chunks.len() - 2
        } else {
            chunks.len() - 1
        }; // Error is before Min(0)
        let button_chunk_index = if error_message.is_some() {
            error_chunk_index - 1
        } else {
            error_chunk_index
        }; // Buttons are before error (or Min(0))
        let notes_chunk_index = button_chunk_index - 2; // Notes area is before spacer and buttons
        let notes_label_chunk_index = notes_chunk_index - 1;
        let distance_input_chunk_index = notes_label_chunk_index - 1;
        let distance_label_chunk_index = distance_input_chunk_index - 1;
        let weight_dur_inputs_chunk_index = distance_label_chunk_index - 1;
        let weight_dur_label_chunk_index = weight_dur_inputs_chunk_index - 1;
        let sets_reps_inputs_chunk_index = weight_dur_label_chunk_index - 1;
        let sets_reps_label_chunk_index = sets_reps_inputs_chunk_index - 1;

        // Row 2: Sets/Reps Labels
        let sets_reps_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_label_chunk_index]);
        f.render_widget(Paragraph::new("Sets:"), sets_reps_layout[0]);
        f.render_widget(Paragraph::new("Reps:"), sets_reps_layout[1]);
        // Row 2: Sets/Reps Inputs
        let sets_reps_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_inputs_chunk_index]);
        let sets_style = if *focused_field == AddWorkoutField::Sets {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(sets_input.as_str()).style(sets_style),
            sets_reps_inputs[0].inner(&input_margin),
        );
        let reps_style = if *focused_field == AddWorkoutField::Reps {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(reps_input.as_str()).style(reps_style),
            sets_reps_inputs[1].inner(&input_margin),
        );

        // Row 3: Weight/Duration Labels
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let weight_label_text = if resolved_exercise
            .as_ref()
            .map_or(false, |def| def.type_ == ExerciseType::BodyWeight)
        {
            format!("Added Weight ({}):", weight_unit)
        } else {
            format!("Weight ({}):", weight_unit)
        };
        let weight_dur_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_label_chunk_index]);
        f.render_widget(Paragraph::new(weight_label_text), weight_dur_layout[0]);
        f.render_widget(Paragraph::new("Duration (min):"), weight_dur_layout[1]);
        // Row 3: Weight/Duration Inputs
        let weight_dur_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_inputs_chunk_index]);
        let weight_style = if *focused_field == AddWorkoutField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_dur_inputs[0].inner(&input_margin),
        );
        let dur_style = if *focused_field == AddWorkoutField::Duration {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(duration_input.as_str()).style(dur_style),
            weight_dur_inputs[1].inner(&input_margin),
        );

        // Row 4: Distance Label & Input
        let dist_unit = match app.service.config.units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        f.render_widget(
            Paragraph::new(format!("Distance ({}):", dist_unit)),
            chunks[distance_label_chunk_index],
        );
        let dist_style = if *focused_field == AddWorkoutField::Distance {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(distance_input.as_str()).style(dist_style),
            chunks[distance_input_chunk_index].inner(&input_margin),
        );

        // Row 5: Notes Label & Input
        f.render_widget(Paragraph::new("Notes:"), chunks[notes_label_chunk_index]);
        let notes_style = if *focused_field == AddWorkoutField::Notes {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(notes_input.as_str())
                .wrap(Wrap { trim: false })
                .style(notes_style)
                .block(Block::default().borders(Borders::LEFT)),
            chunks[notes_chunk_index].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );

        // Row 6: Buttons
        let base_button_style = Style::default().fg(Color::White);
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);
        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 7: Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // --- Render Suggestions Popup ---
        if (*focused_field == AddWorkoutField::Exercise
            || *focused_field == AddWorkoutField::Suggestions)
            && !exercise_suggestions.is_empty()
        {
            let suggestions_height = exercise_suggestions.len() as u16 + 2; // +2 for border
            let suggestions_width = chunks[1].width; // Match input width
            let suggestions_x = chunks[1].x;
            // Position below the input field
            let suggestions_y = chunks[1].y + 1;

            // Create the popup area, ensuring it doesn't go off-screen
            let popup_area = Rect {
                x: suggestions_x,
                y: suggestions_y,
                width: suggestions_width.min(f.size().width.saturating_sub(suggestions_x)),
                height: suggestions_height.min(f.size().height.saturating_sub(suggestions_y)),
            };

            // Convert suggestions to ListItems
            let list_items: Vec<ListItem> = exercise_suggestions
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();

            // Create the list widget
            let suggestions_list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title("Suggestions"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            // Render the popup
            f.render_widget(Clear, popup_area); // Clear background under popup
                                                // Render statefully using a mutable clone of the state
            let mut list_state = suggestion_list_state.clone(); // Clone state for rendering
            f.render_stateful_widget(suggestions_list, popup_area, &mut list_state);
        }

        // --- Cursor Positioning ---
        match focused_field {
            // Position cursor in input field even when suggestions are focused
            AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                let cursor_x = (exercise_input_area.x + exercise_input.chars().count() as u16)
                    .min(exercise_input_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, exercise_input_area.y);
            }
            AddWorkoutField::Sets => f.set_cursor(
                sets_reps_inputs[0].x + 1 + sets_input.chars().count() as u16,
                sets_reps_inputs[0].y,
            ),
            AddWorkoutField::Reps => f.set_cursor(
                sets_reps_inputs[1].x + 1 + reps_input.chars().count() as u16,
                sets_reps_inputs[1].y,
            ),
            AddWorkoutField::Weight => f.set_cursor(
                weight_dur_inputs[0].x + 1 + weight_input.chars().count() as u16,
                weight_dur_inputs[0].y,
            ),
            AddWorkoutField::Duration => f.set_cursor(
                weight_dur_inputs[1].x + 1 + duration_input.chars().count() as u16,
                weight_dur_inputs[1].y,
            ),
            AddWorkoutField::Distance => f.set_cursor(
                chunks[distance_input_chunk_index].x + 1 + distance_input.chars().count() as u16,
                chunks[distance_input_chunk_index].y,
            ),
            AddWorkoutField::Notes => {
                let lines: Vec<&str> = notes_input.lines().collect();
                let last_line = lines.last().unwrap_or(&"");
                let notes_area = chunks[notes_chunk_index].inner(&Margin {
                    vertical: 0,
                    horizontal: 1,
                }); // Area inside border
                let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
                let cursor_x = notes_area.x + last_line.chars().count() as u16;
                f.set_cursor(
                    cursor_x.min(notes_area.right() - 1),
                    cursor_y.min(notes_area.bottom() - 1),
                );
            }
            _ => {} // No cursor for buttons
        }
    }
}

fn render_set_target_weight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::SetTargetWeight {
        weight_input,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let block = Block::default()
            .title("Set Target Bodyweight")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());
        let area = centered_rect(60, 11, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        // Get the inner area *after* the block's margin/border
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            // No margin here, use inner_area directly
            .constraints([
                Constraint::Length(1), // Target label
                Constraint::Length(1), // Target input
                Constraint::Length(1), // Spacer/Buttons row
                Constraint::Length(1), // Buttons row
                Constraint::Length(1), // Error Message (if any) - adjusted constraints
                Constraint::Min(0),    // Remaining space
            ])
            .split(inner_area); // Split the inner_area

        f.render_widget(
            Paragraph::new(format!("Target Weight ({}):", weight_unit)),
            chunks[0],
        );

        // --- Input Field Rendering with Padding ---
        let base_input_style = Style::default().fg(Color::White);
        let weight_input_area = chunks[1];
        let weight_text_area = weight_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let weight_style = if *focused_field == SetTargetWeightField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_text_area,
        );
        // --- End Input Field Rendering ---

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]); // Buttons in chunk 3

        let base_button_style = Style::default().fg(Color::White);
        let set_button = Paragraph::new(" Set ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Set {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(set_button, button_layout[0]);

        let clear_button = Paragraph::new(" Clear Target ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Clear {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(clear_button, button_layout[1]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[2]);

        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[4],
            ); // Error in chunk 4
        }

        // --- Cursor Positioning (using padded area) ---
        match focused_field {
            SetTargetWeightField::Weight => {
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            _ => {}
        }
        // --- End Cursor Positioning ---
    }
}

fn render_create_exercise_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Create New Exercise")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        // --- Calculate Fixed Height ---
        let mut required_height = 2; // Top/Bottom border/padding
        required_height += 1; // Name label
        required_height += 1; // Name input
        required_height += 1; // Muscles label
        required_height += 1; // Muscles input
        required_height += 1; // Type label
        required_height += 1; // Type options
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error message line
        }
        // Add a little extra vertical padding if desired
        // required_height += 1;

        // --- Use centered_rect_fixed ---
        let fixed_width = 60; // Keep a fixed width (adjust as needed)
        let area = centered_rect_fixed(fixed_width, required_height, f.size());

        f.render_widget(Clear, area); // Clear the background
        f.render_widget(block, area); // Render the block border/title

        // Define the inner area *after* the block border/padding
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        // --- Layout Constraints ---
        // Define constraints based on the required elements. The Min(0) handles extra space if any.
        let mut constraints = vec![
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Muscles label
            Constraint::Length(1), // Muscles input
            Constraint::Length(1), // Type label
            Constraint::Length(1), // Type options
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0)); // Remainder (handles any extra space from fixed height)

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints) // Use the dynamically built constraints
            .split(inner_area);

        // --- Render Widgets ---
        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };
        let base_button_style = Style::default().fg(Color::White);

        // Row 1: Name
        f.render_widget(Paragraph::new("Name:"), chunks[0]);
        let name_style = if *focused_field == AddExerciseField::Name {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(name_input.as_str()).style(name_style),
            chunks[1].inner(&input_margin),
        );

        // Row 2: Muscles
        f.render_widget(Paragraph::new("Muscles (comma-separated):"), chunks[2]);
        let muscles_style = if *focused_field == AddExerciseField::Muscles {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(muscles_input.as_str()).style(muscles_style),
            chunks[3].inner(&input_margin),
        );

        // Row 3: Type
        f.render_widget(Paragraph::new("Type:"), chunks[4]);

        let type_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[5]); // Types in chunk 5

        // Render Type Options (same as before)
        let res_text = " Resistance ";
        let res_style = if *selected_type == ExerciseType::Resistance {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let res_para = Paragraph::new(res_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeResistance {
                res_style.reversed()
            } else {
                res_style
            });
        f.render_widget(res_para, type_layout[0]);

        let card_text = " Cardio ";
        let card_style = if *selected_type == ExerciseType::Cardio {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let card_para = Paragraph::new(card_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeCardio {
                card_style.reversed()
            } else {
                card_style
            });
        f.render_widget(card_para, type_layout[1]);

        let bw_text = " BodyWeight ";
        let bw_style = if *selected_type == ExerciseType::BodyWeight {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let bw_para = Paragraph::new(bw_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeBodyweight {
                bw_style.reversed()
            } else {
                bw_style
            });
        f.render_widget(bw_para, type_layout[2]);

        // Row 4: Buttons (adjust chunk index based on error message presence)
        let button_chunk_index = if error_message.is_some() { 8 } else { 7 }; // Spacer is before buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);

        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 5: Error Message (if present)
        if let Some(err) = error_message {
            // Error message is always the second to last chunk before Min(0)
            let error_chunk_index = chunks.len() - 2;
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // --- Cursor Positioning --- (remains the same logic)
        match focused_field {
            AddExerciseField::Name => {
                let cursor_x = (chunks[1].x + 1 + name_input.chars().count() as u16)
                    .min(chunks[1].right().saturating_sub(1));
                f.set_cursor(cursor_x, chunks[1].y);
            }
            AddExerciseField::Muscles => {
                let cursor_x = (chunks[3].x + 1 + muscles_input.chars().count() as u16)
                    .min(chunks[3].right().saturating_sub(1));
                f.set_cursor(cursor_x, chunks[3].y);
            }
            _ => {} // No cursor for type options or buttons
        }
    }
}

fn render_edit_workout_modal(f: &mut Frame, app: &App) {
    // This is almost identical to render_add_workout_modal, but with a different title,
    // a read-only exercise field, and no suggestions.
    if let ActiveModal::EditWorkout {
        // workout_id is not displayed directly
        exercise_name, // Display this
        sets_input,
        reps_input,
        weight_input,
        duration_input,
        distance_input,
        notes_input,
        focused_field,
        error_message,
        resolved_exercise,
        ..
        // No suggestion fields needed here
    } = &app.active_modal
    {
        let block = Block::default()
            .title(format!("Edit Workout Entry ({})", exercise_name)) // Use exercise name in title
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        // Calculate required height (similar to Add modal, but no exercise input focus/suggestions)
        let mut required_height = 2; // Borders/Padding
        required_height += 1; // Exercise display (read-only)
                              // required_height += 1; // No exercise input row
        required_height += 1; // Sets/Reps labels
        required_height += 1; // Sets/Reps inputs
        required_height += 1; // Weight/Duration labels
        required_height += 1; // Weight/Duration inputs
        required_height += 1; // Distance label
        required_height += 1; // Distance input
        required_height += 1; // Notes label
        required_height += 3; // Notes input (multi-line)
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error Message
        }

        let fixed_width = 80;
        let area = centered_rect_fixed(fixed_width, required_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(1), // Exercise display
            // Constraint::Length(1), // No exercise input
            Constraint::Length(1), // Sets/Reps labels
            Constraint::Length(1), // Sets/Reps inputs
            Constraint::Length(1), // Weight/Duration labels
            Constraint::Length(1), // Weight/Duration inputs
            Constraint::Length(1), // Distance label
            Constraint::Length(1), // Distance input
            Constraint::Length(1), // Notes label
            Constraint::Length(3), // Notes input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };

        // Row 0: Exercise Display (read-only)
        f.render_widget(
            Paragraph::new(format!("Exercise: {}", exercise_name))
                .style(Style::default().fg(Color::DarkGray)), // Style as read-only
            chunks[0],
        );

        // Row 1: Sets/Reps Labels & Inputs (chunk indices shift by -1 compared to Add modal)
        let sets_reps_label_chunk_index = 1;
        let sets_reps_inputs_chunk_index = 2;
        let weight_dur_label_chunk_index = 3;
        let weight_dur_inputs_chunk_index = 4;
        let distance_label_chunk_index = 5;
        let distance_input_chunk_index = 6;
        let notes_label_chunk_index = 7;
        let notes_chunk_index = 8;
        let button_chunk_index = 10; // After spacer at 9
        let error_chunk_index = if error_message.is_some() { 11 } else { 10 }; // After buttons

        let sets_reps_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_label_chunk_index]);
        f.render_widget(Paragraph::new("Sets:"), sets_reps_layout[0]);
        f.render_widget(Paragraph::new("Reps:"), sets_reps_layout[1]);
        let sets_reps_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_inputs_chunk_index]);
        let sets_style = if *focused_field == AddWorkoutField::Sets {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(sets_input.as_str()).style(sets_style),
            sets_reps_inputs[0].inner(&input_margin),
        );
        let reps_style = if *focused_field == AddWorkoutField::Reps {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(reps_input.as_str()).style(reps_style),
            sets_reps_inputs[1].inner(&input_margin),
        );

        // Row 2: Weight/Duration Labels & Inputs
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let weight_label_text = if resolved_exercise
            .as_ref()
            .map_or(false, |def| def.type_ == ExerciseType::BodyWeight)
        {
            format!("Added Weight ({}):", weight_unit)
        } else {
            format!("Weight ({}):", weight_unit)
        };
        let weight_dur_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_label_chunk_index]);
        f.render_widget(Paragraph::new(weight_label_text), weight_dur_layout[0]);
        f.render_widget(Paragraph::new("Duration (min):"), weight_dur_layout[1]);
        let weight_dur_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_inputs_chunk_index]);
        let weight_style = if *focused_field == AddWorkoutField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_dur_inputs[0].inner(&input_margin),
        );
        let dur_style = if *focused_field == AddWorkoutField::Duration {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(duration_input.as_str()).style(dur_style),
            weight_dur_inputs[1].inner(&input_margin),
        );

        // Row 3: Distance Label & Input
        let dist_unit = match app.service.config.units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        f.render_widget(
            Paragraph::new(format!("Distance ({}):", dist_unit)),
            chunks[distance_label_chunk_index],
        );
        let dist_style = if *focused_field == AddWorkoutField::Distance {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(distance_input.as_str()).style(dist_style),
            chunks[distance_input_chunk_index].inner(&input_margin),
        );

        // Row 4: Notes Label & Input
        f.render_widget(Paragraph::new("Notes:"), chunks[notes_label_chunk_index]);
        let notes_style = if *focused_field == AddWorkoutField::Notes {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(notes_input.as_str())
                .wrap(Wrap { trim: false })
                .style(notes_style)
                .block(Block::default().borders(Borders::LEFT)),
            chunks[notes_chunk_index].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );

        // Row 5: Buttons
        let base_button_style = Style::default().fg(Color::White);
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);
        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 6: Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // Cursor Positioning (Copied from Add modal, excluding Exercise/Suggestions)
        match focused_field {
            AddWorkoutField::Sets => f.set_cursor(
                sets_reps_inputs[0].x + 1 + sets_input.chars().count() as u16,
                sets_reps_inputs[0].y,
            ),
            AddWorkoutField::Reps => f.set_cursor(
                sets_reps_inputs[1].x + 1 + reps_input.chars().count() as u16,
                sets_reps_inputs[1].y,
            ),
            AddWorkoutField::Weight => f.set_cursor(
                weight_dur_inputs[0].x + 1 + weight_input.chars().count() as u16,
                weight_dur_inputs[0].y,
            ),
            AddWorkoutField::Duration => f.set_cursor(
                weight_dur_inputs[1].x + 1 + duration_input.chars().count() as u16,
                weight_dur_inputs[1].y,
            ),
            AddWorkoutField::Distance => f.set_cursor(
                chunks[distance_input_chunk_index].x + 1 + distance_input.chars().count() as u16,
                chunks[distance_input_chunk_index].y,
            ),
            AddWorkoutField::Notes => {
                let lines: Vec<&str> = notes_input.lines().collect();
                let last_line = lines.last().unwrap_or(&"");
                let notes_area = chunks[notes_chunk_index].inner(&Margin {
                    vertical: 0,
                    horizontal: 1,
                });
                let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
                let cursor_x = notes_area.x + last_line.chars().count() as u16;
                f.set_cursor(
                    cursor_x.min(notes_area.right() - 1),
                    cursor_y.min(notes_area.bottom() - 1),
                );
            }
            _ => {} // No cursor for buttons or read-only fields
        }
    }
}

// NEW: Render Confirmation Modal
fn render_confirmation_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::ConfirmDeleteWorkout {
        exercise_name,
        set_index,
        ..
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)); // Make it stand out

        let question = format!("Delete set {} of {}?", set_index, exercise_name);
        let options = "[Y]es / [N]o (Esc)";

        // Calculate text width for centering
        let question_width = question.len() as u16;
        let options_width = options.len() as u16;
        let text_width = question_width.max(options_width);
        let modal_width = text_width + 4; // Add padding
        let modal_height = 5; // Fixed height: border + question + options + border

        let area = centered_rect_fixed(modal_width, modal_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Question
                Constraint::Length(1), // Options
            ])
            .split(inner_area);

        f.render_widget(
            Paragraph::new(question).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(options).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );

        // No cursor needed for this simple modal
    }
}

//src/ui/placeholders.rs
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

//src/ui/status_bar.rs
// task-athlete-tui/src/ui/status_bar.rs
use crate::app::{state::ActiveModal, AddWorkoutField, App}; // Use App from crate::app
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match &app.active_modal {
         ActiveModal::None => match app.active_tab {
             crate::app::ActiveTab::Log => "[Tab] Focus | [↑↓/jk] Nav | [←→/hl] Date | [a]dd | [l]og set | [e]dit | [d]elete | [g]raphs | [?] Help | [Q]uit ",
             crate::app::ActiveTab::History => "[↑↓/jk] Nav | [/f] Filter | [e]dit | [d]elete | [?] Help | [Q]uit ",
             crate::app::ActiveTab::Graphs => "[Tab] Focus | [↑↓/jk] Nav | [/] Filter Exercise | [Enter] Select | [?] Help | [Q]uit ",
             crate::app::ActiveTab::Bodyweight => "[↑↓/jk] Nav Hist | [l]og | [t]arget | [r]ange | [?] Help | [Q]uit ",
         }.to_string(),
         ActiveModal::Help => " [Esc/Enter/?] Close Help ".to_string(),
         ActiveModal::LogBodyweight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
         ActiveModal::SetTargetWeight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
         ActiveModal::AddWorkout { focused_field, exercise_suggestions, .. } => { // Destructure focused_field
             match focused_field {
                 AddWorkoutField::Exercise if !exercise_suggestions.is_empty() =>
                     "Type name | [↓] Suggestions | [Tab] Next Field | [Esc] Cancel".to_string(),
                 AddWorkoutField::Exercise =>
                     "Type name/alias | [Tab] Next Field | [Esc] Cancel".to_string(),
                 AddWorkoutField::Suggestions =>
                     "[↑↓] Select | [Enter] Confirm Suggestion | [Esc/Tab] Back to Input".to_string(),
                 _ => // Generic hint for other fields
                      "[Esc] Cancel | [Enter] Confirm/Next | [Tab/↑↓] Navigate | [↑↓ Arrow] Inc/Dec Number ".to_string(),
             }
             },
         ActiveModal::CreateExercise { .. } => " [Esc] Cancel | [Enter] Confirm/Next | [Tab/↑↓/←→] Navigate ".to_string(),
         ActiveModal::EditWorkout { .. } => " [Esc] Cancel | [Enter] Confirm/Next | [Tab/↑↓] Navigate ".to_string(),
         ActiveModal::ConfirmDeleteWorkout { .. } => " Confirm Deletion: [Y]es / [N]o (Esc) ".to_string(),
     };

    let error_text = app.last_error.as_deref().unwrap_or("");

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(area);

    let status_paragraph =
        Paragraph::new(status_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status_paragraph, status_chunks[0]);

    let error_paragraph = Paragraph::new(error_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::Red))
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(error_paragraph, status_chunks[1]);
}

//src/ui/tabs.rs
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

