use crate::{
    app::{state::ActiveModal, App},
    ui::layout::centered_rect,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(super) fn render_confirmation_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::ConfirmDeleteWorkout {
        exercise_name,
        set_index,
        .. // Ignore workout_id
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD));

        let question = format!("Delete set {} of {}?", set_index, exercise_name);
        let options = "[Y]es / [N]o (Esc)";

        let question_width = question.len() as u16;
        let options_width = options.len() as u16;
        let text_width = question_width.max(options_width);
        let modal_width = text_width + 4; // Add padding for borders/margins
        let modal_height = 5; // Title, border, question, options, border

        let area = centered_rect(modal_width, modal_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)]) // Question, Options
            .split(inner_area);

        f.render_widget(
            Paragraph::new(question).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(options).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}

pub(super) fn render_confirmation_bodyweight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::ConfirmDeleteBodyWeight {
        
        set_index,
        .. // Ignore workout_id
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD));

        let question = format!("Delete bodyweight entry {}?", set_index);
        let options = "[Y]es / [N]o (Esc)";

        let question_width = question.len() as u16;
        let options_width = options.len() as u16;
        let text_width = question_width.max(options_width);
        let modal_width = text_width + 4; // Add padding for borders/margins
        let modal_height = 5; // Title, border, question, options, border

        let area = centered_rect(modal_width, modal_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)]) // Question, Options
            .split(inner_area);

        f.render_widget(
            Paragraph::new(question).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(options).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
