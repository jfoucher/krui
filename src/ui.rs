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
    let state = app.printer.stats.state.clone();

    let (state_bg, state_fg) = match state.as_str() {
        "standby" => (Color::Gray, Color::LightGreen),
        "printing" => (Color::Green, Color::White),
        "paused"   => (Color::Gray, Color::LightCyan),
        "complete"  => (Color::LightGreen, Color::Black),
        "cancelled" => (Color::Gray, Color::White),
        "error"     => (Color::Red, Color::White),
        _ => (Color::Black, Color::White),
    };
    let h = app.printer.toolhead.homed.x && app.printer.toolhead.homed.y && app.printer.toolhead.homed.z;
    let qgl = app.printer.toolhead.homed.qgl;
    let fan = app.printer.toolhead.fan.speed;
    let text = vec![
        Line::from(vec![
            Span::styled(c, Style::default().bg(bg).fg(Color::White)),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled(state, Style::default().bg(state_bg).fg(state_fg)),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("Home", Style::default().fg(Color::White).bg(if h {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("QGL", Style::default().fg(Color::White).bg(if qgl {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled(format!("Fan {:.0}", fan*100.0), Style::default().fg(Color::White).bg(if fan < 0.3 {Color::Green} else if fan < 0.6 {Color::LightRed } else {Color::Red})),

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
