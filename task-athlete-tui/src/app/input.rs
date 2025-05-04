// src/app/input.rs
use super::{
    data::{log_change_date, log_set_next_exercised_date, log_set_previous_exercised_date},
    modals::{
        handle_add_workout_modal_input, handle_confirm_delete_body_weigth_input,
        handle_confirm_delete_modal_input, handle_create_exercise_modal_input,
        handle_edit_workout_modal_input, handle_log_bodyweight_modal_input, handle_pb_modal_input,
        handle_set_target_weight_modal_input,
    },
    navigation::{
        bw_table_next, bw_table_previous, history_list_next, history_list_previous, log_list_next,
        log_list_previous, log_table_next, log_table_previous,
    },
    state::{
        ActiveModal, ActiveTab, App, BodyweightFocus, GraphsFocus, HistoryFocus,
        LogBodyweightField, LogFocus, SetTargetWeightField,
    },
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// Main key event handler method on App
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
            ActiveModal::PersonalBest { .. } => handle_pb_modal_input(self, key)?,
            ActiveModal::ConfirmDeleteWorkout { .. } => {
                handle_confirm_delete_modal_input(self, key)?;
            }
            ActiveModal::ConfirmDeleteBodyWeight { .. } => {
                handle_confirm_delete_body_weigth_input(self, key);
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
                KeyCode::Char('g') => {
                    // Navigate to Graphs tab with selected exercise
                    if let Some(selected_index) = self.log_exercise_list_state.selected() {
                        if let Some(selected_exercise_name) =
                            self.log_exercises_today.get(selected_index)
                        {
                            // Find the index of this exercise in the graphs list
                            if let Some(graph_index) = self
                                .graph_exercises_all
                                .iter()
                                .position(|name| name == selected_exercise_name)
                            {
                                // Set the selected exercise for the graph tab
                                self.graph_selected_exercise = Some(selected_exercise_name.clone());
                                // Update the graph list state
                                self.graph_exercise_list_state.select(Some(graph_index));
                                // Update graph data (will use selected exercise and potentially current/default type)
                                self.update_graph_data();
                                // Switch tab
                                self.active_tab = ActiveTab::Graphs;
                                // Set focus to the graph type list
                                self.graph_focus = GraphsFocus::GraphTypeList;
                            } else {
                                // Handle case where exercise exists in log but not in graph list (should ideally not happen if lists are synced)
                                self.set_error(
                                    "Selected exercise not found in graph list.".to_string(),
                                );
                            }
                        }
                    }
                }
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                KeyCode::Char('H') => log_set_previous_exercised_date(self)?,
                KeyCode::Char('L') => log_set_next_exercised_date(self)?,
                _ => {}
            },
            LogFocus::SetList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_table_previous(self),
                KeyCode::Char('j') | KeyCode::Down => log_table_next(self),
                KeyCode::Tab => self.log_focus = LogFocus::ExerciseList,
                KeyCode::Char('e') | KeyCode::Enter => self.open_edit_workout_modal()?, // EDIT
                KeyCode::Char('d') | KeyCode::Delete => {
                    self.open_delete_confirmation_modal();
                } // DELETE
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                KeyCode::Char('H') => log_set_previous_exercised_date(self)?,
                KeyCode::Char('L') => log_set_next_exercised_date(self)?,
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_history_input(&mut self, _key: KeyEvent) -> Result<()> {
        match self.history_focus {
            HistoryFocus::DayList => match _key.code {
                KeyCode::Char('k') | KeyCode::Up => history_list_previous(self),
                KeyCode::Char('j') | KeyCode::Down => history_list_next(self),
                KeyCode::Char('l') => {
                    // Navigate to Log tab for selected date
                    if let Some(selected_index) = self.history_list_state.selected() {
                        if let Some((date, _)) = self.history_data.get(selected_index) {
                            self.active_tab = ActiveTab::Log;
                            self.log_viewed_date = *date;
                            // Reset log focus/selection for the new date
                            self.log_focus = LogFocus::ExerciseList;
                            self.log_exercise_list_state.select(Some(0)); // Select first exercise if any
                            self.log_set_table_state.select(Some(0)); // Select first set if any
                                                                      // Log data will refresh automatically in the main loop
                        }
                    }
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_graphs_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.graph_focus {
            GraphsFocus::ExerciseList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => graphs_exercise_list_previous(self),
                KeyCode::Char('j') | KeyCode::Down => graphs_exercise_list_next(self)?,
                KeyCode::Tab => self.graph_focus = GraphsFocus::GraphTypeList,
                KeyCode::Enter => {
                    // Set selected exercise, trigger data update, move focus
                    if let Some(index) = self.graph_exercise_list_state.selected() {
                        if let Some(name) = self.graph_exercises_all.get(index) {
                            self.graph_selected_exercise = Some(name.clone());
                            self.update_graph_data(); // Update based on new selection
                            self.graph_focus = GraphsFocus::GraphTypeList;
                        }
                    }
                }
                _ => {}
            },
            GraphsFocus::GraphTypeList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => graphs_type_list_previous(self),
                KeyCode::Char('j') | KeyCode::Down => graphs_type_list_next(self),
                KeyCode::Tab => self.graph_focus = GraphsFocus::ExerciseList, // Cycle back
                KeyCode::Enter => {
                    // Set selected type, trigger data update, move focus (optional)
                    if let Some(index) = self.graph_type_list_state.selected() {
                        if let Some(graph_type) = self.graph_types_available.get(index) {
                            self.graph_selected_type = Some(*graph_type);
                            self.update_graph_data(); // Update based on new type
                                                      // Optionally move focus to graph display, or leave it here
                                                      // self.graph_focus = GraphsFocus::GraphDisplay;
                        }
                    }
                }
                _ => {}
            },
            GraphsFocus::History => {}
        }
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
                    KeyCode::Char('d') => {
                        self.open_delete_bodyweight_confirmation_modal();
                    }
                    KeyCode::Tab => self.bw_focus = BodyweightFocus::Actions,
                    _ => {}
                },
                BodyweightFocus::Actions => {
                    if key.code == KeyCode::Tab {
                        self.bw_focus = BodyweightFocus::History
                    }
                }
                BodyweightFocus::Graph => {
                    if key.code == KeyCode::Tab {
                        self.bw_focus = BodyweightFocus::Actions
                    }
                }
            },
        }
        Ok(())
    }
}

pub fn graphs_exercise_list_next(app: &mut App) -> Result<()> {
    let list_len = app.service.list_exercises(None, None)?.len();
    if list_len == 0 {
        return Ok(());
    }
    let current_selection = app.graph_exercise_list_state.selected();
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.graph_exercise_list_state.select(Some(i));
    Ok(())
}

pub fn graphs_exercise_list_previous(app: &mut App) {
    let list_len = app.graph_exercises_all.len();
    if list_len == 0 {
        return;
    }
    let current_selection = app.graph_exercise_list_state.selected();
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.graph_exercise_list_state.select(Some(i));
}

pub fn graphs_type_list_next(app: &mut App) {
    // Assuming app.graph_types_available is populated
    let list_len = app.graph_types_available.len();
    super::navigation_helpers::list_next(&mut app.graph_type_list_state, list_len);
}

pub fn graphs_type_list_previous(app: &mut App) {
    // Assuming app.graph_types_available is populated
    let list_len = app.graph_types_available.len();
    super::navigation_helpers::list_previous(&mut app.graph_type_list_state, list_len);
}
