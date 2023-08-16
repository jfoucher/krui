use tui::{
    backend::Backend,
    layout::{Alignment, Rect, Layout, Direction, Constraint},
    style::{Color, Style, Modifier},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, text::{Line, Span}, prelude::Corner,
};

use crate::app::App;

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui-org/ratatui/tree/master/examples
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Max(1),
                Constraint::Min(20),
                Constraint::Max(1)
            ].as_ref()
        )
        .split(frame.size());

    let mut c = " ✕ ";
    let mut bg = Color::Red;
    if app.printer.connected {
        c = " ✔ ";
        bg = Color::Green;
    }
    let mut st = format!("{:?}", app.printer.status.heaters);
    if let Some(extruder) = app.printer.status.heaters.get("extruder") {
        st = format!("{} {}", extruder.target, extruder.temperature);
    }
    


    let text = vec![
        Line::from(vec![
            Span::styled(c, Style::default().bg(bg).fg(Color::White)),
            Span::styled("line", Style::default().add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD).bg(Color::Gray).fg(Color::DarkGray)),
            Span::styled(format!("{}", st), Style::default().fg(Color::Red).bg(Color::Blue)),
        ]),
    ];
    let p = Paragraph::new(text)
            .block(Block::default()
            //.title("")
            .borders(Borders::NONE)
        );

    frame.render_widget(p, chunks[0]);
    let block = Block::default()
         .title("Content")
         .borders(Borders::NONE);
    let p = Paragraph::new(app.get_data())
        .block(Block::default()
        .title("JSON")
        .borders(Borders::ALL)
    );
    frame.render_widget(p, chunks[1]);

    let block = Block::default()
         .title("Footer")
         .borders(Borders::NONE);
    frame.render_widget(block, chunks[2]);
}
