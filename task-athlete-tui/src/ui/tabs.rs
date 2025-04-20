use crate::app::{ActiveTab, App}; // Use App from crate::app
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ["Log (F1)", "History (F2)", "Graphs (F3)", "Bodyweight (F4)"]
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::Gray))))
        .collect();

    let selected_tab_index = match app.active_tab {
        ActiveTab::Log => 0,
        ActiveTab::History => 1,
        ActiveTab::Graphs => 2,
        ActiveTab::Bodyweight => 3,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(selected_tab_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}
