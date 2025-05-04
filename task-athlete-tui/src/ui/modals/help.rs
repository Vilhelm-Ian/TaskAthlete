use crate::ui::layout::centered_rect;
use ratatui::{
    layout::Margin,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(super) fn render_help_modal(f: &mut Frame) {
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
        Line::from(" a: Add New Workout Entry (for viewed day)"),
        Line::from(" c: Create New Exercise Definition"),
        Line::from(" e / Enter: Edit Selected Set/Entry (in Sets Table)"),
        Line::from(" d / Delete: Delete Selected Set/Entry (in Sets Table)"),
        Line::from(" g: Go to Graphs for Selected Exercise (TODO)"),
        Line::from(""),
        Line::from("--- Exercises Tab (F2) ---").style(Style::new().bold().underlined()),
        Line::from(" k/j / ↑/↓: Navigate List"),
        Line::from(" Enter: View/Edit Selected Exercise (TODO)"),
        Line::from(" d / Delete: Delete Selected Exercise (TODO)"),
        Line::from(" c: Create New Exercise Definition"),
        Line::from(" /: Start Search/Filter"),
        Line::from(" Esc: Clear Search/Filter"),
        Line::from(""),
        Line::from("--- Planning Tab (F3) ---").style(Style::new().bold().underlined()),
        Line::from(" (TODO)"),
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
        area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}
