use crate::app::{state::LogFocus, App}; // Use App from crate::app
use chrono::{Duration, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use task_athlete_lib::{Units, Workout}; // Import Units

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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

#[allow(clippy::struct_excessive_bools)]
struct ColumnVisibility {
    has_reps: bool,
    has_weight: bool,
    has_duration: bool,
    has_distance: bool,
    has_notes: bool,
}

// --- Helper Functions ---

/// Determines which optional columns have data based on the provided sets.
fn determine_column_visibility(sets: &[Workout]) -> ColumnVisibility {
    ColumnVisibility {
        has_reps: sets.iter().any(|w| w.reps.is_some()),
        has_weight: sets
            .iter()
            .any(|w| w.calculate_effective_weight().is_some()),
        has_duration: sets.iter().any(|w| w.duration_minutes.is_some()),
        has_distance: sets.iter().any(|w| w.distance.is_some()),
        // Consider empty strings as "no data" for notes if desired:
        // has_notes: sets.iter().any(|w| w.notes.as_ref().map_or(false, |s| !s.is_empty())),
        has_notes: sets.iter().any(|w| w.notes.is_some()), // Original check
    }
}

/// Creates the styled block for the table.
fn create_table_block(title: String, is_focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
}

/// Creates the header row based on visible columns and units.
fn create_table_header<'a>(visibility: &ColumnVisibility, units: Units) -> Row<'a> {
    let mut header_cells = vec![Cell::from("Set").style(Style::default().fg(Color::LightBlue))];

    if visibility.has_reps {
        header_cells.push(Cell::from("Reps").style(Style::default().fg(Color::LightBlue)));
    }
    if visibility.has_weight {
        let weight_unit = match units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        header_cells.push(
            Cell::from(format!("Weight ({})", weight_unit))
                .style(Style::default().fg(Color::LightBlue)),
        );
    }
    if visibility.has_duration {
        header_cells.push(Cell::from("Duration").style(Style::default().fg(Color::LightBlue)));
    }
    if visibility.has_distance {
        let dist_unit = match units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        header_cells.push(
            Cell::from(format!("Distance ({})", dist_unit))
                .style(Style::default().fg(Color::LightBlue)),
        );
    }
    if visibility.has_notes {
        header_cells.push(Cell::from("Notes").style(Style::default().fg(Color::LightBlue)));
    }

    Row::new(header_cells).height(1).bottom_margin(1)
}

/// Calculates the column widths based on visible columns.
fn calculate_table_widths(visibility: &ColumnVisibility) -> Vec<Constraint> {
    let mut widths = vec![Constraint::Length(5)]; // "Set" column

    if visibility.has_reps {
        widths.push(Constraint::Length(6));
    }
    if visibility.has_weight {
        widths.push(Constraint::Length(8));
    }
    if visibility.has_duration {
        widths.push(Constraint::Length(10));
    }
    if visibility.has_distance {
        widths.push(Constraint::Length(10));
    }

    if visibility.has_notes {
        widths.push(Constraint::Min(10)); // Notes column expands
    } else {
        // Make the last *visible* column expand instead
        if let Some(last_width) = widths.last_mut() {
            match last_width {
                Constraint::Length(l) => *last_width = Constraint::Min(*l),
                _ => {} // Leave Min/Max/Percentage/Ratio as is
            }
        }
        // Handle edge case: only "Set" column is visible
        if widths.len() == 1 {
            widths[0] = Constraint::Min(5);
        }
    }
    widths
}

/// Creates the data rows for the table based on visible columns and units.
fn create_table_rows<'a>(
    sets: &'a [Workout],
    visibility: ColumnVisibility,
    units: Units,
) -> Vec<Row<'a>> {
    sets.iter()
        .enumerate()
        .map(|(i, w)| {
            let mut row_cells = vec![Cell::from(format!("{}", i + 1))]; // "Set" number cell

            if visibility.has_reps {
                row_cells.push(Cell::from(
                    w.reps.map_or("-".to_string(), |v| v.to_string()),
                ));
            }
            if visibility.has_weight {
                let weight_display = match units {
                    Units::Metric => w.calculate_effective_weight(),
                    Units::Imperial => w.calculate_effective_weight().map(|kg| kg * 2.20462),
                };
                let weight_str = weight_display.map_or("-".to_string(), |v| format!("{:.1}", v));
                row_cells.push(Cell::from(weight_str));
            }
            if visibility.has_duration {
                row_cells.push(Cell::from(
                    w.duration_minutes
                        .map_or("-".to_string(), |v| format!("{} min", v)),
                ));
            }
            if visibility.has_distance {
                let dist_val = match units {
                    Units::Metric => w.distance,
                    Units::Imperial => w.distance.map(|km| km * 0.621_371),
                };
                let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1}", v));
                row_cells.push(Cell::from(dist_str));
            }
            if visibility.has_notes {
                row_cells.push(Cell::from(w.notes.as_deref().unwrap_or("-"))); // Use as_deref for efficiency
            }

            Row::new(row_cells)
        })
        .collect()
}

// --- Main Rendering Function (Now Cleaner) ---

fn render_log_set_list(f: &mut Frame, app: &mut App, area: Rect) {
    // 1. Determine Title and Focus
    let selected_exercise_name = app
        .log_exercise_list_state
        .selected()
        .and_then(|i| app.log_exercises_today.get(i));

    let title = selected_exercise_name
        .map(|name| format!("Sets for: {}", name))
        .unwrap_or_else(|| "Select an Exercise".to_string());

    let is_focused = app.log_focus == LogFocus::SetList;

    // 2. Get necessary data
    let sets = &app.log_sets_for_selected_exercise;
    let units = app.service.config.units;

    // 3. Check column visibility
    let visibility = determine_column_visibility(sets);

    // 4. Create reusable table components using helpers
    let table_block = create_table_block(title, is_focused);
    let header = create_table_header(&visibility, units);
    let widths = calculate_table_widths(&visibility);
    let rows = create_table_rows(sets, visibility, units);

    // 5. Build the final table widget
    let table = Table::new(rows, &widths) // Pass widths as a slice
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // 6. Render the stateful widget
    f.render_stateful_widget(table, area, &mut app.log_set_table_state);
}
