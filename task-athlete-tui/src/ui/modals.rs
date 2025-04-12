// task-athlete-tui/src/ui/modals.rs
use crate::{
    app::{
        state::{ActiveModal, LogBodyweightField, SetTargetWeightField},
        App,
    }, // Use App from crate::app
    ui::layout::centered_rect, // Use centered_rect from layout
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f), // Don't need app state for help text
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
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
