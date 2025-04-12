// task-athlete-tui/src/ui/log_tab.rs
use crate::app::{state::LogFocus, App}; // Use App from crate::app
use chrono::{Duration, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render_log_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let today_str = Utc::now().date_naive();
    let date_header_str = if app.log_viewed_date == today_str {
        format!("--- Today ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else if app.log_viewed_date == today_str - Duration::days(1) {
        format!("--- Yesterday ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else {
        format!("--- {} ---", app.log_viewed_date.format("%Y-%m-%d"))
    };

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let date_header = Paragraph::new(date_header_str)
        .alignment(ratatui::layout::Alignment::Center);
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

     let weight_unit = match app.service.config.units { Units::Metric => "kg", Units::Imperial => "lbs" };
     let dist_unit = match app.service.config.units { Units::Metric => "km", Units::Imperial => "mi" };
     let weight_cell = format!("Weight ({})", weight_unit);
     let distance_cell = format!("Distance ({})", dist_unit);
     let header_cells = ["Set", "Reps", &weight_cell, "Duration", &distance_cell, "Notes"]
         .into_iter()
         .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
     let header = Row::new(header_cells).height(1).bottom_margin(1);

     let rows = app.log_sets_for_selected_exercise.iter().enumerate().map(|(i, w)| {
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
             Cell::from(format!("{}", i + 1)),
             Cell::from(w.reps.map_or("-".to_string(), |v| v.to_string())),
             Cell::from(weight_str),
             Cell::from(w.duration_minutes.map_or("-".to_string(), |v| format!("{} min", v))),
             Cell::from(dist_str),
             Cell::from(w.notes.clone().unwrap_or_else(|| "-".to_string())),
         ])
     });

     let widths = [
         Constraint::Length(5), Constraint::Length(6), Constraint::Length(8),
         Constraint::Length(10), Constraint::Length(10), Constraint::Min(10),
     ];

     let table = Table::new(rows, widths)
         .header(header)
         .block(table_block)
         .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
         .highlight_symbol(">> ");

     f.render_stateful_widget(table, area, &mut app.log_set_table_state);
}
