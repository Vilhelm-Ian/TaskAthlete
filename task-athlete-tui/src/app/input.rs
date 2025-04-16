// src/app/input.rs
use super::{
    actions::{
        bw_cycle_graph_range, open_add_workout_modal, open_create_exercise_modal,
        open_delete_confirmation_modal, open_edit_workout_modal,
    },
    data::log_change_date,
    modals::{
        handle_add_workout_modal_input, handle_confirm_delete_modal_input,
        handle_create_exercise_modal_input, handle_edit_workout_modal_input,
        handle_log_bodyweight_modal_input, handle_set_target_weight_modal_input,
    },
    navigation::{
        bw_table_next, bw_table_previous, log_list_next, log_list_previous, log_table_next,
        log_table_previous,
    },
    state::{ActiveModal, ActiveTab, App, BodyweightFocus, LogFocus},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// Main key event handler method on App
impl App {
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

    // --- Modal Input Handling ---
    fn handle_modal_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.active_modal {
            ActiveModal::Help => Self::handle_help_modal_input(self, key),
            ActiveModal::LogBodyweight { .. } => handle_log_bodyweight_modal_input(self, key)?,
            ActiveModal::SetTargetWeight { .. } => handle_set_target_weight_modal_input(self, key)?,
            ActiveModal::AddWorkout { .. } => handle_add_workout_modal_input(self, key)?,
            ActiveModal::CreateExercise { .. } => handle_create_exercise_modal_input(self, key)?,
            ActiveModal::EditWorkout { .. } => handle_edit_workout_modal_input(self, key)?,
            ActiveModal::ConfirmDeleteWorkout { .. } => {
                handle_confirm_delete_modal_input(self, key)?
            }
            _ => {
                // Generic fallback for other modals if added later
                if key.code == KeyCode::Esc {
                    self.active_modal = ActiveModal::None;
                }
            }
        }
        Ok(())
    }

    fn handle_help_modal_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('?') => {
                self.active_modal = ActiveModal::None;
            }
            _ => {} // Ignore other keys in help
        }
    }

    // --- Tab-Specific Input Handling ---
    fn handle_log_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.log_focus {
            LogFocus::ExerciseList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_list_previous(self),
               KeyCode::Char('j') | KeyCode::Down => log_list_next(self),
               KeyCode::Tab => self.log_focus = LogFocus::SetList,
         ,       KeyCode::Char('a') => open_add_workout_modal(self)?,
                KeyCode::Char('c') => open_create_exercise_modal(self)?,
                KeyCode::Char('g') => { /* TODO: Go to Graphs */ }
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                _ => {}
            },
            LogFocus::SetList => match key.code {
                KeyCode::Char('k') | KeyCode::Up => log_table_previous(self),
                KeyCode::Char('j') | KeyCode::Down => log_table_next(self),
                KeyCode::Tab => self.log_focus = LogFocus::ExerciseList,
                KeyCode::Char('e') | KeyCode::Enter => open_edit_workout_modal(self)?,
                KeyCode::Char('d') | KeyCode::Delete => open_delete_confirmation_modal(self)?,
                KeyCode::Char('h') | KeyCode::Left => log_change_date(self, -1),
                KeyCode::Char('l') | KeyCode::Right => log_change_date(self, 1),
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_history_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO: History tab input
        Ok(())
    }

    fn handle_graphs_input(&mut self, _key: KeyEvent) -> Result<()> {
        // TODO: Graphs tab input
        Ok(())
    }

    fn handle_bodyweight_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('l') => self.open_log_bodyweight_modal(),
            KeyCode::Char('t') => self.open_set_target_weight_modal(),
            KeyCode::Char('r') => bw_cycle_graph_range(self),
            // Basic navigation for now, Tab focus cycle not implemented
            KeyCode::Char('k') | KeyCode::Up => bw_table_previous(self),
            KeyCode::Char('j') | KeyCode::Down => bw_table_next(self),
            //KeyCode::Tab => // TODO: Implement focus switching if needed
            _ => {}
        }
        Ok(())
    }
}
