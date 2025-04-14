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
         ActiveModal::CreateExercise { .. } => " [Esc] Cancel | [Enter] Confirm/Next | [Tab/↑↓/←→] Navigate ".to_string()
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
