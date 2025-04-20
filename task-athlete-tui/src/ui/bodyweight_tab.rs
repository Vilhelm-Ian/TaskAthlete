use crate::app::{state::BodyweightFocus, App}; // Use App from crate::app
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, LegendPosition, Paragraph, Row,
        Table, Wrap,
    },
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_bodyweight_graph(f, app, chunks[0]);
    render_bodyweight_bottom(f, app, chunks[1]);
}

pub fn render_bodyweight_graph(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let target_data;
    let mut datasets = vec![];

    let data_points: Vec<(f64, f64)> = app
        .bw_graph_data
        .iter()
        .map(|(x, y)| {
            let display_weight = match app.service.config.units {
                Units::Metric => *y,
                Units::Imperial => *y * 2.20462,
            };
            (*x, display_weight)
        })
        .collect();

    datasets.push(
        Dataset::default()
            .name("Bodyweight")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points),
    );

    if let Some(target_raw) = app.bw_target {
        let target_display = match app.service.config.units {
            Units::Metric => target_raw,
            Units::Imperial => target_raw * 2.20462,
        };
        if app.bw_graph_x_bounds[0] <= app.bw_graph_x_bounds[1] {
            target_data = vec![
                (app.bw_graph_x_bounds[0], target_display),
                (app.bw_graph_x_bounds[1], target_display),
            ];
            datasets.push(
                Dataset::default()
                    .name("Target")
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::ITALIC),
                    )
                    .data(&target_data),
            );
        }
    }

    let display_y_bounds = match app.service.config.units {
        Units::Metric => app.bw_graph_y_bounds,
        Units::Imperial => [
            app.bw_graph_y_bounds[0] * 2.20462,
            app.bw_graph_y_bounds[1] * 2.20462,
        ],
    };

    let range_label = match app.bw_graph_range_months {
        1 => "1M",
        3 => "3M",
        6 => "6M",
        12 => "1Y",
        _ => "All",
    };
    let chart_title = format!("Bodyweight Trend ({})", range_label);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(chart_title)
                .border_style(if app.bw_focus == BodyweightFocus::Graph {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .x_axis(
            Axis::default()
                .title("Date".italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(app.bw_graph_x_bounds)
                .labels(vec![]),
        )
        .y_axis(
            Axis::default()
                .title(format!("Weight ({})", weight_unit).italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(display_y_bounds)
                .labels({
                    let min_label = display_y_bounds[0].ceil() as i32;
                    let max_label = display_y_bounds[1].floor() as i32;
                    let range = (max_label - min_label).max(1);
                    let step = (range / 5).max(1);
                    (min_label..=max_label)
                        .step_by(step as usize)
                        .map(|w| Span::from(format!("{:.0}", w)))
                        .collect()
                }),
        )
        .legend_position(Some(LegendPosition::TopLeft));

    f.render_widget(chart, area);
}

fn render_bodyweight_bottom(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_bodyweight_status(f, app, chunks[0]);
    render_bodyweight_history(f, app, chunks[1]);
}

fn render_bodyweight_status(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let (latest_weight_str, latest_date_str) = match app.bw_history.first() {
        Some((_, date, w)) => {
            let display_w = match app.service.config.units {
                Units::Metric => *w,
                Units::Imperial => *w * 2.20462,
            };
            (
                format!("{:.1} {}", display_w, weight_unit),
                format!("(on {})", date.format("%Y-%m-%d")),
            )
        }
        None => ("N/A".to_string(), "".to_string()),
    };
    let target_weight_str = match app.bw_target {
        Some(w) => {
            let display_w = match app.service.config.units {
                Units::Metric => w,
                Units::Imperial => w * 2.20462,
            };
            format!("{:.1} {}", display_w, weight_unit)
        }
        None => "Not Set".to_string(),
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Latest: ", Style::default().bold()),
            Span::raw(latest_weight_str),
            Span::styled(
                format!(" {}", latest_date_str),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().bold()),
            Span::raw(target_weight_str),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " [L]og New ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [T]arget Weight ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [R]ange Cycle ",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status & Actions")
                .border_style(if app.bw_focus == BodyweightFocus::Actions {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_bodyweight_history(f: &mut Frame, app: &mut App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let table_block = Block::default()
        .borders(Borders::ALL)
        .title("History")
        .border_style(if app.bw_focus == BodyweightFocus::History {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let weight_cell_header = format!("Weight ({})", weight_unit);
    let header_cells = ["Date", weight_cell_header.as_str()]
        .into_iter()
        .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.bw_history.iter().map(|(_, date, weight_kg)| {
        let display_weight = match app.service.config.units {
            Units::Metric => *weight_kg,
            Units::Imperial => *weight_kg * 2.20462,
        };
        Row::new(vec![
            Cell::from(date.format("%Y-%m-%d").to_string()),
            Cell::from(format!("{:.1}", display_weight)),
        ])
    });

    let widths = [Constraint::Length(12), Constraint::Min(10)];
    let table = Table::new(rows, widths)
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.bw_history_state);
}
