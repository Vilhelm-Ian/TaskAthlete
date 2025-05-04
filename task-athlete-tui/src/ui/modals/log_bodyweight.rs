use super::helpers::{render_button_pair, render_error_message, render_input_field};
use crate::{
    app::{
        state::{ActiveModal, LogBodyweightField},
        App,
    },
    ui::layout::centered_rect,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders, Clear},
    Frame,
};
use task_athlete_lib::Units;

pub(super) fn render_log_bodyweight_modal(f: &mut Frame, app: &App) {
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

        let has_error = error_message.is_some();
        let height = 8 + u16::from(has_error); // Base height + error line
        let area = centered_rect(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Weight field
            Constraint::Length(2), // Date field
            Constraint::Length(1), // Buttons row
        ];
        if has_error {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0)); // Fill remainder

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let weight_text_area = render_input_field(
            f,
            chunks[0],
            &format!("Weight ({weight_unit}):"),
            weight_input,
            *focused_field == LogBodyweightField::Weight,
        );

        let date_text_area = render_input_field(
            f,
            chunks[1],
            "Date (YYYY-MM-DD / today/yesterday):",
            date_input,
            *focused_field == LogBodyweightField::Date,
        );

        let button_focus = match focused_field {
            LogBodyweightField::Confirm => Some(0),
            LogBodyweightField::Cancel => Some(1),
            _ => None,
        };
        render_button_pair(f, chunks[2], "OK", "Cancel", button_focus);

        let error_chunk_index = 3;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // --- Cursor Positioning ---
        position_cursor_for_input(
            f,
            focused_field,
            weight_input,
            &weight_text_area,
            date_input,
            &date_text_area,
        );
    }
}

/// Helper to position the cursor within the Log Bodyweight modal's fields.
fn position_cursor_for_input(
    f: &mut Frame,
    focused_field: &LogBodyweightField,
    weight_input: &str,
    weight_area: &Rect,
    date_input: &str,
    date_area: &Rect,
) {
    match focused_field {
        LogBodyweightField::Weight => {
            let cursor_x = (weight_area.x + weight_input.chars().count() as u16)
                .min(weight_area.right().saturating_sub(1));
            f.set_cursor(cursor_x, weight_area.y);
        }
        LogBodyweightField::Date => {
            let cursor_x = (date_area.x + date_input.chars().count() as u16)
                .min(date_area.right().saturating_sub(1));
            f.set_cursor(cursor_x, date_area.y);
        }
        _ => {} // No cursor for buttons
    }
}
