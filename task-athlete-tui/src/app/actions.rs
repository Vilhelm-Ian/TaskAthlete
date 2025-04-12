// task-athlete-tui/src/app/actions.rs
use super::modals::{handle_log_bodyweight_modal_input, handle_set_target_weight_modal_input}; // Use specific modal handlers
use super::navigation::{
    bw_table_next, bw_table_previous, log_list_next, log_list_previous, log_table_next,
    log_table_previous,
};
use super::state::{ActiveModal, ActiveTab, App, BodyweightFocus, LogBodyweightField, LogFocus, SetTargetWeightField};
use super::data::log_change_date;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

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
        // Data refresh is now handled by the main loop *after* input handling
        // self.refresh_data_for_active_tab(); // Remove refresh call from here
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
            ActiveModal::LogBodyweight { .. } => handle_log_bodyweight_modal_input(self, key)?, // Pass self
            ActiveModal::SetTargetWeight { .. } => handle_set_target_weight_modal_input(self, key)?, // Pass self
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
                KeyCode::Char('a') => { /* TODO */ }
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
