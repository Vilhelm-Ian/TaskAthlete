use super::helpers::{render_button_pair, render_error_message, render_input_field};
use crate::{
    app::{
        state::{ActiveModal, AddExerciseField},
        App,
    },
    ui::layout::centered_rect,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use task_athlete_lib::ExerciseType;

pub(super) fn render_create_exercise_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
        focused_field,
        error_message,
        log_weight,
        log_reps,
        log_duration,
        log_distance,
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Create New Exercise")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        let has_error = error_message.is_some();
        // Calculate required height:
        // 2 lines for Name input field (label + box)
        // 2 lines for Muscles input field (label + box)
        // 1 line for Type label
        // 1 line for Type options
        // 1 line for Logging label
        // 1 line for Logging checkboxes
        // 1 line spacer
        // 1 line for buttons
        // Total content lines = 10 + 1 (optional error)
        // Add 2 lines for modal top/bottom borders = 12 or 13 total height
        let height = 12 + u16::from(has_error);
        let area = centered_rect(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Name field
            Constraint::Length(2), // Muscles field
            Constraint::Length(1), // Type label
            Constraint::Length(1), // Type options
            Constraint::Length(1), // Logging label
            Constraint::Length(1), // Logging checkboxes
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if has_error {
            constraints.push(Constraint::Length(1)); // Error
        }
        constraints.push(Constraint::Min(0)); // Fill remainder

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let name_text_area = render_input_field(
            f,
            chunks[0],
            "Name:",
            name_input,
            *focused_field == AddExerciseField::Name,
        );
        let muscles_text_area = render_input_field(
            f,
            chunks[1],
            "Muscles (comma-separated):",
            muscles_input,
            *focused_field == AddExerciseField::Muscles,
        );

        f.render_widget(Paragraph::new("Type:"), chunks[2]);
        render_exercise_type_options(f, chunks[3], selected_type, focused_field);

        f.render_widget(Paragraph::new("Logging:"), chunks[4]);

        render_log_flag_checkboxes(
            f,
            chunks[5], // Chunk for checkboxes
            *log_weight,
            *log_reps,
            *log_duration,
            *log_distance,
            focused_field,
        );

        let button_focus = match focused_field {
            AddExerciseField::Confirm => Some(0),
            AddExerciseField::Cancel => Some(1),
            _ => None,
        };
        render_button_pair(f, chunks[7], "OK", "Cancel", button_focus); // Buttons in chunk 5 (after spacer)

        let error_chunk_index = 8;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // Cursor Positioning
        position_cursor_for_create_exercise(
            f,
            focused_field,
            name_input,
            &name_text_area,
            muscles_input,
            &muscles_text_area,
        );
    }
}

/// Renders the exercise type selection buttons.
fn render_exercise_type_options(
    f: &mut Frame,
    area: Rect,
    selected_type: &ExerciseType,
    focused_field: &AddExerciseField,
) {
    let type_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let base_style = Style::default().fg(Color::White);
    let types = [
        ExerciseType::Resistance,
        ExerciseType::Cardio,
        ExerciseType::BodyWeight,
    ];
    let focus_fields = [
        AddExerciseField::TypeResistance,
        AddExerciseField::TypeCardio,
        AddExerciseField::TypeBodyweight,
    ];
    let labels = [" Resistance ", " Cardio ", " BodyWeight "];

    for i in 0..3 {
        let is_selected = *selected_type == types[i];
        let is_focused = *focused_field == focus_fields[i];

        let mut style = if is_selected {
            base_style.bg(Color::DarkGray)
        } else {
            base_style
        };
        if is_focused {
            style = style.add_modifier(Modifier::REVERSED);
        }

        f.render_widget(
            Paragraph::new(labels[i])
                .alignment(ratatui::layout::Alignment::Center)
                .style(style),
            type_layout[i],
        );
    }
}

/// Helper to position the cursor within the Create Exercise modal's fields.
fn position_cursor_for_create_exercise(
    f: &mut Frame,
    focused_field: &AddExerciseField,
    name_input: &str,
    name_area: &Rect,
    muscles_input: &str,
    muscles_area: &Rect,
) {
    match focused_field {
        AddExerciseField::Name => {
            let cursor_x = (name_area.x + name_input.chars().count() as u16)
                .min(name_area.right().saturating_sub(1));
            f.set_cursor(cursor_x, name_area.y);
        }
        AddExerciseField::Muscles => {
            let cursor_x = (muscles_area.x + muscles_input.chars().count() as u16)
                .min(muscles_area.right().saturating_sub(1));
            f.set_cursor(cursor_x, muscles_area.y);
        }
        _ => {} // No cursor for type selection or buttons or checkboxes
    }
}

/// Renders the logging flag checkboxes.
fn render_log_flag_checkboxes(
    f: &mut Frame,
    area: Rect,
    log_weight: bool,
    log_reps: bool,
    log_duration: bool,
    log_distance: bool,
    focused_field: &AddExerciseField,
) {
    let checkboxes_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let fields = [
        AddExerciseField::LogWeight,
        AddExerciseField::LogReps,
        AddExerciseField::LogDuration,
        AddExerciseField::LogDistance,
    ];
    let states = [log_weight, log_reps, log_duration, log_distance];
    let labels = ["Weight", "Reps", "Duration", "Distance"];

    for i in 0..4 {
        let is_focused = *focused_field == fields[i];
        let state_char = if states[i] { "[x]" } else { "[ ]" };
        let text = format!("{} {}", state_char, labels[i]);
        let style = Style::default()
            .fg(Color::White)
            .add_modifier(if is_focused {
                Modifier::REVERSED
            } else {
                Modifier::empty()
            });
        f.render_widget(Paragraph::new(text).style(style), checkboxes_layout[i]);
    }
}
