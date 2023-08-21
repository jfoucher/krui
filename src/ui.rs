pub mod header;
pub mod toolhead;
pub mod main;
pub mod console;
pub mod stateful_list;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect, Layout, Direction, Constraint},
    style::{Color, Style, Modifier, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph, Padding, Wrap},
    Frame, text::{Line, Span}, prelude::Corner,
};
use crate::{printer::{Heater}, button::{footer_button, Button, self}, markdown, app::HistoryItem};
use crate::app::App;
use crate::app::Tab;



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
                Constraint::Min(8),
            ].as_ref()
        )
        .split(frame.size());

    header::draw_header(frame, app, chunks[0]);

    match app.current_tab {
        Tab::Main => main::draw_main_tab(frame, app, chunks[1]),
        Tab::Help => main::draw_main_help(frame, app, chunks[1]),
        Tab::Toolhead => toolhead::draw_toolhead_tab(frame, app, chunks[1]),
        Tab::ToolheadHelp => toolhead::draw_toolhead_help(frame, app, chunks[1]),
        Tab::Console => console::draw_tab(frame, app, chunks[1]),
        Tab::ConsoleHelp => console::draw_help(frame, app, chunks[1]),
        _ => {}
    }

    if app.printer.connected == false && app.printer.status.state == "shutdown".to_string() {
        let sl = Paragraph::new(format!("{: ^50}{: <450}", format!("Klipper reports: {}", app.printer.status.state), format!("{}Press F10 to restart the firmware", app.printer.status.state_message)))
        .block(Block::default()
            .style(Style::default().bg(Color::White).fg(Color::Black))
            
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
        )
        .wrap(Wrap {trim: false})
        ;

        frame.render_widget(sl, Rect::new((frame.size().width - 50) / 2, (frame.size().height - 12) / 2, 50, 12));
    }

    if app.printer.printing_file != None && app.printer.status.print_state != "printing".to_string() {
        let text = vec![
            Line::from(vec![
                Span::styled(format!("{: ^48}", "Confirm print start"), Style::default().add_modifier(Modifier::BOLD))
            ]).alignment(Alignment::Center),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![
                Span::styled(format!("{: <48}", "This will start a print of "), Style::default()),
            ]),
            Line::from(vec![
                Span::styled(format!("{: ^48}", app.printer.printing_file.clone().unwrap()), Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![Span::styled(format!("{: ^48}", " "), Style::default())]),
            Line::from(vec![
                Span::styled(format!("     {: <19}", "Enter<OK>"), Style::default().bg(Color::White).fg(Color::Black)),
                Span::styled(format!("{: >19}     ", "Esc<Cancel>"), Style::default().bg(Color::White).fg(Color::Black)),
            ]),
        ];
        let sl = Paragraph::new(text)
        .block(Block::default()
            .style(Style::default().reset().bg(Color::White).fg(Color::Black))
            
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
        )

        ;

        frame.render_widget(sl, Rect::new((frame.size().width - 50) / 2, (frame.size().height - 12) / 2, 50, 12));
    }
    
    

}

