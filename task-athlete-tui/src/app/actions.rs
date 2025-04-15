// task-athlete-tui/src/app/actions.rs
use super::data::log_change_date;
use super::modals::{
    handle_add_workout_modal_input, handle_create_exercise_modal_input,
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
use task_athlete_lib::{ExerciseDefinition, ExerciseType, Units, Workout};

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
            // NEW: Handle CreateExercise modal
            ActiveModal::CreateExercise { .. } => handle_create_exercise_modal_input(self, key)?,
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
                        let last_workout = self.get_last_workout_for_exercise(&def.name);
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
}
