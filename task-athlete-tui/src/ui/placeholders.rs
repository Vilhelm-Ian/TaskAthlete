use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, title: &str, area: Rect) {
    let placeholder_text = Paragraph::new(format!("{} - Implementation Pending", title))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    f.render_widget(placeholder_text, area);
}
