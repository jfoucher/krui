pub mod header;
pub mod toolhead;
pub mod main;

pub mod console;
pub mod webcam;
pub mod stateful_list;
use std::rc::Rc;

use tui::{
    backend::Backend,
    layout::{Alignment, Rect, Layout, Direction, Constraint},
    style::{Color, Style, Modifier, Stylize},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap, Clear, Padding, Table, Row},
    Frame, text::{Line, Span},
};

use crate::{app::App, button::{Button, action_button}, printer::Webcam};
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
        Tab::Webcam => webcam::draw_tab(frame, app, chunks[1]),
        Tab::WebcamHelp => webcam::draw_help(frame, app, chunks[1]),
        _ => {}
    }

    if app.printer.connected == false && app.printer.status.state == "shutdown".to_string() {
        let title = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("Klipper reports: {}", app.printer.status.state), Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)),
            ]).alignment(Alignment::Center),
        ]);
        let text = Paragraph::new(app.printer.status.state_message.clone())
        .wrap(Wrap {trim: false});

        let restart: Button = Button::new("Restart".to_string(), Some("10".to_string()));
        let btn = Table::new(vec![
            Row::new(vec![
                Line::from(action_button(restart)).alignment(Alignment::Center),
            ])
        ])
        .widths(&[
            Constraint::Percentage(100),
        ])
        .block(Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(2))
        )
        ;

        modal(frame, title, text, btn, None);
    }

    
    

}

pub fn modal<'a, B>(f: &mut Frame<B>, title: Paragraph, text: Paragraph, buttons: Table, input: Option<Paragraph>) -> Rc<[Rect]>
where
    B: Backend,
{
    let area = Rect::new((f.size().width - 50) / 2, (f.size().height - 12) / 2, 50, 12);
    f.render_widget(Clear, area);
    f.render_widget(Block::default()
        .style(Style::default().reset().bg(Color::White).fg(Color::Black))
        .borders(Borders::ALL)
        .border_type(BorderType::Thick), area);

    let mut constraints = [
        Constraint::Length(2),     // title
        Constraint::Min(3),         // text
        Constraint::Length(3),         // Input
        Constraint::Length(2),     // Buttons
    ];
    if input.is_none() {
        constraints[2] = Constraint::Length(0);
    }

    let chunks =  Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            constraints
            .as_ref(),
        )
        .split(area);

    f.render_widget(title, chunks[0]);
    f.render_widget(text.block(Block::default().padding(Padding::horizontal(1)))
    .wrap(Wrap {trim: false}), chunks[1]);
    if let Some(input_field) = input {
        f.render_widget(input_field, Rect::new(chunks[2].x + 1, chunks[2].y, chunks[2].width - 2, chunks[2].height));
        
    }
    f.render_widget(buttons, Rect::new(chunks[3].x + 1, chunks[3].y, chunks[3].width - 2, chunks[3].height));

    return chunks;
}