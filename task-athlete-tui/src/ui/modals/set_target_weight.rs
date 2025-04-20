use super::helpers::{render_error_message, render_input_field};
use crate::{
    app::{
        state::{ActiveModal, SetTargetWeightField},
        App,
    },
    ui::layout::centered_rect,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use task_athlete_lib::Units;

pub(super) fn render_set_target_weight_modal(f: &mut Frame, app: &App) {
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

        let has_error = error_message.is_some();
        let height = 5 + u16::from(has_error);
        let area = centered_rect(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Target field
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
            &format!("Target Weight ({weight_unit}):"),
            weight_input,
            *focused_field == SetTargetWeightField::Weight,
        );

        render_target_weight_buttons(f, chunks[1], focused_field);

        let error_chunk_index = 2;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // --- Cursor Positioning ---
        if focused_field == &SetTargetWeightField::Weight {
            let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                .min(weight_text_area.right().saturating_sub(1));
            f.set_cursor(cursor_x, weight_text_area.y);
        }
    }
}

/// Renders the three buttons (Set, Clear, Cancel) for the target weight modal.
fn render_target_weight_buttons(f: &mut Frame, area: Rect, focused_field: &SetTargetWeightField) {
    let button_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let base_style = Style::default().fg(Color::White);
    let focused_style = base_style.reversed();

    f.render_widget(
        Paragraph::new(" Set ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Set {
                focused_style
            } else {
                base_style
            }),
        button_layout[0],
    );
    f.render_widget(
        Paragraph::new(" Clear Target ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Clear {
                focused_style
            } else {
                base_style
            }),
        button_layout[1],
    );
    f.render_widget(
        Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Cancel {
                focused_style
            } else {
                base_style
            }),
        button_layout[2],
    );
}
