use tui::{widgets::{Borders, Paragraph, Block, Wrap, Table, Row}, prelude::*};

use crate::{ui::header, button::{Button, action_button}, markdown, app::App};

const TOOLHEAD_HELP_TEXT: &str = "
# Toolhead Help

This screen presents a list of all axis present on the printer.
Pressing X will home the X axis, pressing Y will home the Y axis, and pressing Z will home the Z axis.
Pressing A will home all axes.
Pressing Q will trigger a quad gantry leveling operation if available on your printer.

";


pub fn draw_toolhead_tab<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{

    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints(
        [
            Constraint::Min(6),     // Main content
            Constraint::Max(1),     // Tab Footer
        ]
        .as_ref(),
    )
    .split(area);
    let t_title = Span::styled(format!("{: ^width$}", "Toolhead", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
        
    let x_button = Button::new("Home".to_string(), Some("X".to_string()));
    let y_button = Button::new("Home".to_string(), Some("Y".to_string()));
    let z_button = Button::new("Home".to_string(), Some("Z".to_string()));
    let p = Table::new(vec![
        Row::new(vec![
            Line::from("Axis").alignment(Alignment::Center),
            Line::from("Position").alignment(Alignment::Center),
            Line::from("Homed").alignment(Alignment::Center),
            Line::from("").alignment(Alignment::Center),
        ]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),

        Row::new(vec![
            Line::from(Span::styled("X", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
            Line::from(format!("{:.2}mm", app.printer.toolhead.position.x)).alignment(Alignment::Center),
            Line::from(format!("{}", app.printer.toolhead.homed.x)).alignment(Alignment::Center),
            Line::from(action_button(x_button)).alignment(Alignment::Center),
        ]).style(Style::default()),
        Row::new(vec![
            Line::from(Span::styled("Y", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
            Line::from(format!("{:.2}mm", app.printer.toolhead.position.y)).alignment(Alignment::Center),
            Line::from(format!("{}", app.printer.toolhead.homed.y)).alignment(Alignment::Center),
            Line::from(action_button(y_button)).alignment(Alignment::Center),
        ]).style(Style::default()),
        Row::new(vec![
            Line::from(Span::styled("Z", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
            Line::from(format!("{:.3}mm", app.printer.toolhead.position.z)).alignment(Alignment::Center),
            Line::from(format!("{}", app.printer.toolhead.homed.z)).alignment(Alignment::Center),
            Line::from(action_button(z_button)).alignment(Alignment::Center),
        ]).style(Style::default()),
        
    ])
    .widths(&[
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),

    ])
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .title_style(Style::default().add_modifier(style::Modifier::BOLD).fg(Color::White).bg(Color::Magenta))
            .borders(Borders::NONE)
        )
        ;

    f.render_widget(p, chunks[0]);

    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Close".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new("Webcam".to_string(), Some("6".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[1], buttons);

}




pub fn draw_toolhead_help<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
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
    let t_title = Span::styled(format!("{: ^width$}", "Toolhead help", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(markdown::parse(TOOLHEAD_HELP_TEXT))
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
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new("Webcam".to_string(), Some("6".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[1], buttons);

}


