use chrono::Timelike;
use tui::{widgets::{Borders, Paragraph, Block, Wrap, Scrollbar, ScrollbarOrientation}, prelude::*};

use crate::{ui::header, button::Button, markdown, app::{App, InputMode}, printer::GCodeLine};

const CONSOLE_HELP_TEXT: &str = "
# Toolhead Help

This screen presents a list of all axis present on the printer.

Pressing the up and down arrow keys will select one of the axes.

- Press `F1` for this help
- Press `F2` to home the selected axis
- Press `F3` to return to the main screen
- Press `F4` to home all axes
- Press `F5` to set the position of the selected axis
- Press `F6` to move the selected axis
- Press `F7` to start the quad gantry leveling (if supported by your printer)
- Press `F8` to trigger an emergency shutdown
";


pub fn draw_tab<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{


    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints(
        [
            Constraint::Min(6),     // Tab content
            Constraint::Max(1),     // Tab Footer
        ]
        .as_ref(),
    )
    .split(area);

    let div = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints(
        [
            Constraint::Max(1),     // Title
            Constraint::Length(3),     // console input
            Constraint::Min(6),     // Console content
        ]
        .as_ref(),
    )
    .split(chunks[0]);

    let t_title = Span::styled(
        format!("{: ^width$}", "Console", width = f.size().width as usize),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White)
            .bg(Color::Magenta)
    );

    let title = Block::default()
        .title(t_title)
        .title_alignment(Alignment::Center)
        .borders(Borders::NONE);
    f.render_widget(title, div[0]);   
    // Reverse the order of the lines
    let lines: Vec<&GCodeLine> = app.printer.status.gcodes.iter().rev().collect();

    let input = Paragraph::new(app.console_input.value.as_str())
        .style(match app.console_input.mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Send GCODE command"));

    f.render_widget(input, div[1]);

    match app.console_input.mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            f.set_cursor(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                div[1].x + app.console_input.cursor_position as u16 + 1,
                // Move one line down, from the border to the input line
                div[1].y + 1,
            )
        }
    }

    let text: Vec<Line> = lines.iter().map(|gcode| {
        let mut content = gcode.content.as_str().to_string();
        let mut fg = Color::White;
        let mut modif = Modifier::empty();
        if content.starts_with("//") {
            content = content.replace("//", "  ");
            fg = Color::Reset;
        }
        if content.starts_with("!!") {
            content = content.replace("!!", "  ");
            fg = Color::Red;
            modif = Modifier::BOLD;
        }
        Line::from(vec![
            Span::styled(format!(" {:0<2}:{:0<2} ", gcode.timestamp.hour(), gcode.timestamp.minute()), Style::default().fg(Color::Reset).bg(Color::White)),
            Span::styled(content, Style::default().fg(fg).add_modifier(modif)),
        ])
    }).collect();
    app.console_scroll_state = app.console_scroll_state.content_length(text.len() as u16);


    let p = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::NONE)
        )
        .wrap(Wrap { trim: true })
        .scroll((app.console_scroll, 0))
    ;

    f.render_widget(p, div[2]);   

    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        div[2],
        &mut app.console_scroll_state,
    );


    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Close".to_string(), Some("5".to_string())),
        
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[1], buttons);

}




pub fn draw_help<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{

    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints(
        [
            Constraint::Min(6),     // Help text
            Constraint::Max(1),     // Tab Footer
        ]
        .as_ref(),
    )
    .split(area);
    let t_title = Span::styled(format!("{: ^width$}", "Console help", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(markdown::parse(CONSOLE_HELP_TEXT))
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
        )
        .wrap(Wrap { trim: false })
        ;
    f.render_widget(p, chunks[0]);    

    let buttons = vec![
        Button::new("Close".to_string(), Some("1".to_string())),
    ];
    header::draw_footer(f, chunks[1], buttons);

}


