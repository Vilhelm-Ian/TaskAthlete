//src/ui/calendar_tab.rs
use crate::app::{
    state::{CalendarFocus, CalendarView},
    App,
};
use crate::ui::modals::helpers::render_input_field; // Use helper for input field
use crate::ui::placeholders::render_placeholder; // Import placeholder renderer
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_calendar_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)]) // Date/View switch line, Content
        .split(area);

    render_calendar_view_switcher(f, app, chunks[0]);

    match app.calendar_view {
        CalendarView::ListView => render_calendar_list_view(f, app, chunks[1]),
        CalendarView::CalendarView => {
            render_placeholder(f, "Calendar View (Not Yet Implemented)", chunks[1])
        }
    }
}

fn render_calendar_view_switcher(f: &mut Frame, app: &App, area: Rect) {
    let view_text = match app.calendar_view {
        CalendarView::ListView => "--- Calendar View: List ([v] Calendar) ---",
        CalendarView::CalendarView => "--- Calendar View: Calendar ([v] List) ---",
    };
    let paragraph = Paragraph::new(view_text).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(paragraph, area);
}

fn render_calendar_list_view(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter Input
            Constraint::Min(0),    // Workout List
        ])
        .split(area);

    render_calendar_filter_input(f, app, chunks[0]);
    render_calendar_workout_list(f, app, chunks[1]);

    // Manual cursor positioning for the filter input field
    if app.calendar_focus == CalendarFocus::FilterInput {
        let cursor_area = chunks[0].inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }); // Area *inside* the render_input_field block
        let cursor_x = (cursor_area.x + app.calendar_filter_input.chars().count() as u16)
            .min(cursor_area.right().saturating_sub(1));
        f.set_cursor(cursor_x, cursor_area.y);
    }
}

fn render_calendar_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let filter_title = if app.calendar_filter_applied.is_empty() {
        "Filter (Exercise or Muscle):"
    } else {
        // Show applied filter
        &format!("Filter (Applied: '{}'):", app.calendar_filter_applied)
    };

    let block_style = if app.calendar_focus == CalendarFocus::FilterInput {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_area = render_input_field(
        f,
        area,
        "calendar",
        app.calendar_filter_input.as_str(),
        app.calendar_focus == CalendarFocus::FilterInput,
    );

    // Draw the label and surrounding block manually
    let block = Block::default()
        .borders(Borders::ALL)
        .title(filter_title)
        .border_style(block_style);

    f.render_widget(block, area);

    // The input field itself was rendered inside the block area by render_input_field
}

fn render_calendar_workout_list(f: &mut Frame, app: &mut App, area: Rect) {
    let table_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Workouts ({})",
            app.calendar_workouts_filtered.len()
        )) // Show count
        .border_style(if app.calendar_focus == CalendarFocus::WorkoutList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let dist_unit = match app.service.config.units {
        Units::Metric => "km",
        Units::Imperial => "mi",
    };
    let weight_cell = format!("Weight ({})", weight_unit);
    let distance_cell = format!("Distance ({})", dist_unit);

    let header_cells = [
        "Date",
        "Exercise",
        "Sets",
        "Reps",
        &weight_cell,
        "Duration",
        &distance_cell,
        "Notes",
    ]
    .into_iter()
    .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.calendar_workouts_filtered.iter().map(|w| {
        let weight_display = match app.service.config.units {
            Units::Metric => w.weight,
            Units::Imperial => w.weight.map(|kg| kg * 2.20462),
        };
        let weight_str = weight_display.map_or("-".to_string(), |v| format!("{:.1}", v));

        let dist_val = match app.service.config.units {
            Units::Metric => w.distance,
            Units::Imperial => w.distance.map(|km| km * 0.621_371),
        };
        let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1}", v));

        Row::new(vec![
            Cell::from(w.timestamp.format("%Y-%m-%d").to_string()),
            Cell::from(w.exercise_name.clone()),
            Cell::from(w.sets.map_or("-".to_string(), |v| v.to_string())),
            Cell::from(w.reps.map_or("-".to_string(), |v| v.to_string())),
            Cell::from(weight_str),
            Cell::from(
                w.duration_minutes
                    .map_or("-".to_string(), |v| format!("{} min", v)),
            ),
            Cell::from(dist_str),
            Cell::from(w.notes.clone().unwrap_or_else(|| "-".to_string())),
        ])
    });

    let widths = [
        Constraint::Length(12),     // Date
        Constraint::Percentage(20), // Exercise
        Constraint::Length(6),      // Sets
        Constraint::Length(6),      // Reps
        Constraint::Length(8),      // Weight
        Constraint::Length(10),     // Duration
        Constraint::Length(10),     // Distance
        Constraint::Min(10),        // Notes
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.calendar_list_state);
}
