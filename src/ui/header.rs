

use tui::{Frame, prelude::*, widgets::{Paragraph, Block, Borders}};

use crate::{app::App, button::Button};
use crate::button::footer_button;

pub fn draw_header<'a, B>(frame: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{

    let header = Layout::default()
    .direction(Direction::Horizontal)
    .margin(0)
    .constraints(
        [
            Constraint::Min(20),
            Constraint::Max(6)
        ].as_ref()
    )
    .split(area);

    let mut c = " ✕ ";
    let mut bg = Color::Red;
    if app.printer.connected {
    c = " ✔ ";
    bg = Color::Green;
    }
    let mut state = format!(" {} ", app.printer.status.state);
    if app.printer.status.print_state == "printing" {
        state = "printing".to_string();
    }

    let (state_bg, state_fg) = match app.printer.status.state.as_str() {
    "standby" => (Color::Gray, Color::LightGreen),
    "ready" => (Color::Gray, Color::LightGreen),
    "printing" => (Color::Green, Color::White),
    "paused"   => (Color::Gray, Color::LightCyan),
    "complete"  => (Color::LightGreen, Color::Black),
    "cancelled" => (Color::Gray, Color::Black),
    "starting" => (Color::Gray, Color::Black),
    "error"     => (Color::Red, Color::White),
    "shutdown"     => (Color::Red, Color::White),
    _ => (Color::Black, Color::White),
    };
    let h = app.printer.toolhead.homed.x && app.printer.toolhead.homed.y && app.printer.toolhead.homed.z;
    let qgl = app.printer.toolhead.homed.qgl;
    let fan = app.printer.toolhead.fan.speed;
    let text = vec![
        Line::from(vec![
            Span::styled(c, Style::default().bg(bg).fg(Color::White)),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled(format!(" {} ", state), Style::default().bg(state_bg).fg(state_fg)),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("Home", Style::default().fg(Color::White).bg(if h {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("QGL", Style::default().fg(Color::White).bg(if qgl {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("Step", Style::default().fg(Color::White).bg(if app.printer.status.stepper_enable {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled("Fil", Style::default().fg(Color::White).bg(if app.printer.status.filament_switch {Color::Green} else {Color::Red})),
            Span::styled(" ", Style::default().bg(Color::Black)),
            Span::styled(format!("Fan {:.0}", fan*100.0), Style::default().fg(Color::White).bg(if fan < 0.3 {Color::Green} else if fan < 0.6 {Color::LightRed } else {Color::Red})),

        ]),
    ];

    let p = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::NONE)
        );

    frame.render_widget(p, header[0]);

    let s = vec![
        Line::from(vec![
            Span::styled(format!(" {:.2} ", app.printer.sysload), Style::default().bg(if app.printer.sysload < 0.3 {Color::Green} else if app.printer.sysload < 0.6 {Color::LightRed } else {Color::Red}).fg(Color::White)),
        ]),
    ];

    let sl = Paragraph::new(s)
        .block(Block::default()
            //.title("")
            .borders(Borders::NONE)
        );

    frame.render_widget(sl, header[1]);


}

pub fn draw_footer<'a, B>(f: &mut Frame<B>, area: Rect, buttons: Vec<Button>)
where
    B: Backend,
{

    let block = Block::new()
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::LightBlue))
        ;

    f.render_widget(block, area);

    let constraints: Vec<Constraint> = buttons.iter().map(|_| Constraint::Ratio(1, buttons.len() as u32)).collect();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints(
            constraints.as_ref(),
        )
        .split(area);

    for (i, button) in buttons.iter().enumerate() {
        let footer = Paragraph::new(footer_button(button.clone()))
        .block(Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::LightBlue))
        );
        f.render_widget(footer, chunks[i]);
    }
    
}