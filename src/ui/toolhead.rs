use tui::{widgets::{Borders, Paragraph, Block, Wrap}, prelude::*};

use crate::{ui::header, button::Button, markdown, app::App};

const TOOLHEAD_HELP_TEXT: &str = "
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


pub fn draw_toolhead_tab<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
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
    let t_title = Span::styled(format!("{: ^width$}", "Toolhead", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new("TOOLHEAD TAB HERE")
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
        );
    f.render_widget(p, chunks[0]);    

    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Close".to_string(), Some("3".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, app, chunks[1], buttons);

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
    ];
    header::draw_footer(f, app, chunks[1], buttons);

}


