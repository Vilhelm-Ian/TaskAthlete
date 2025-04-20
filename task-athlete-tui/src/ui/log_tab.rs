use crate::app::{state::LogFocus, App}; // Use App from crate::app
use chrono::{Duration, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_log_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let today_str = Utc::now().date_naive();
    let date_header_str = if app.log_viewed_date == today_str {
        format!("--- Today ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else if app.log_viewed_date == today_str - Duration::days(1) {
        format!(
            "--- Yesterday ({}) ---",
            app.log_viewed_date.format("%Y-%m-%d")
        )
    } else {
        format!("--- {} ---", app.log_viewed_date.format("%Y-%m-%d"))
    };

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let date_header = Paragraph::new(date_header_str).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(date_header, outer_chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer_chunks[1]);

    render_log_exercise_list(f, app, chunks[0]);
    render_log_set_list(f, app, chunks[1]);
}

fn render_log_exercise_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .log_exercises_today
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title("Exercises Logged")
        .border_style(if app.log_focus == LogFocus::ExerciseList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.log_exercise_list_state);
}

fn render_log_set_list(f: &mut Frame, app: &mut App, area: Rect) {
    let selected_exercise_name = app
        .log_exercise_list_state
        .selected()
        .and_then(|i| app.log_exercises_today.get(i));

    let title = match selected_exercise_name {
        Some(name) => format!("Sets for: {}", name),
        None => "Select an Exercise".to_string(),
    };

    let table_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if app.log_focus == LogFocus::SetList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    // --- Determine which columns have data ---
    let sets = &app.log_sets_for_selected_exercise; // Borrow for multiple iterations

    // Check if *any* set has a value for the optional fields
    let has_reps = sets.iter().any(|w| w.reps.is_some());
    let has_weight = sets.iter().any(|w| w.weight.is_some());
    let has_duration = sets.iter().any(|w| w.duration_minutes.is_some());
    let has_distance = sets.iter().any(|w| w.distance.is_some());
    // Consider empty strings as "no data" for notes if desired:
    // let has_notes = sets.iter().any(|w| w.notes.as_ref().map_or(false, |s| !s.is_empty()));
    let has_notes = sets.iter().any(|w| w.notes.is_some()); // Original check based on Option

    // --- Prepare dynamic header, rows, and widths ---

    let mut header_cells_vec = vec![Cell::from("Set").style(Style::default().fg(Color::LightBlue))];
    let mut widths_vec = vec![Constraint::Length(5)]; // "Set" column

    // Define unit strings regardless of whether columns are shown
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let dist_unit = match app.service.config.units {
        Units::Metric => "km",
        Units::Imperial => "mi",
    };
    let weight_cell_text = format!("Weight ({})", weight_unit);
    let distance_cell_text = format!("Distance ({})", dist_unit);

    if has_reps {
        header_cells_vec.push(Cell::from("Reps").style(Style::default().fg(Color::LightBlue)));
        widths_vec.push(Constraint::Length(6));
    }
    if has_weight {
        header_cells_vec.push(
            Cell::from(weight_cell_text.as_str()).style(Style::default().fg(Color::LightBlue)),
        );
        widths_vec.push(Constraint::Length(8));
    }
    if has_duration {
        header_cells_vec.push(Cell::from("Duration").style(Style::default().fg(Color::LightBlue)));
        widths_vec.push(Constraint::Length(10));
    }
    if has_distance {
        header_cells_vec.push(
            Cell::from(distance_cell_text.as_str()).style(Style::default().fg(Color::LightBlue)),
        );
        widths_vec.push(Constraint::Length(10));
    }
    if has_notes {
        header_cells_vec.push(Cell::from("Notes").style(Style::default().fg(Color::LightBlue)));
        widths_vec.push(Constraint::Min(10)); // Notes column expands
    } else {
        // If Notes column is hidden, make the *last visible* column expand instead
        if let Some(last_width) = widths_vec.last_mut() {
            // Change Length constraint to Min to make it expand
            match last_width {
                Constraint::Length(l) => *last_width = Constraint::Min(*l),
                // If it was already Min/Max/Percentage/Ratio, leave it as is
                _ => {}
            }
        }
        // Handle edge case: only "Set" column is visible
        if widths_vec.len() == 1 {
            widths_vec[0] = Constraint::Min(5);
        }
    }

    let header = Row::new(header_cells_vec).height(1).bottom_margin(1);

    let rows: Vec<Row> = sets // Need to collect into Vec<Row> for Table::new
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let mut row_cells = vec![Cell::from(format!("{}", i + 1))]; // "Set" number cell

            if has_reps {
                row_cells.push(Cell::from(
                    w.reps.map_or("-".to_string(), |v| v.to_string()),
                ));
            }
            if has_weight {
                let weight_display = match app.service.config.units {
                    Units::Metric => w.weight,
                    Units::Imperial => w.weight.map(|kg| kg * 2.20462),
                };
                let weight_str = weight_display.map_or("-".to_string(), |v| format!("{:.1}", v));
                row_cells.push(Cell::from(weight_str));
            }
            if has_duration {
                row_cells.push(Cell::from(
                    w.duration_minutes
                        .map_or("-".to_string(), |v| format!("{} min", v)),
                ));
            }
            if has_distance {
                let dist_val = match app.service.config.units {
                    Units::Metric => w.distance,
                    Units::Imperial => w.distance.map(|km| km * 0.621_371),
                };
                let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1}", v));
                row_cells.push(Cell::from(dist_str));
            }
            if has_notes {
                row_cells.push(Cell::from(
                    w.notes.clone().unwrap_or_else(|| "-".to_string()),
                ));
            }

            Row::new(row_cells) // Create the row from the dynamic cell list
        })
        .collect(); // Collect the iterator into a Vec<Row>

    // Use the dynamically generated widths
    let table = Table::new(rows, &widths_vec) // Pass the Vec or a slice reference
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.log_set_table_state);
}
