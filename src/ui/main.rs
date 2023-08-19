use tui::{Frame, prelude::*, widgets::{Paragraph, Block, Borders, Wrap}};

use crate::{app::{App, HistoryItem}, button::Button, printer::{Heater, TemperatureFan}};
use crate::markdown;
use crate::ui::header;

const MAIN_HELP_TEXT: &str = "# KLUI Help

KLUI is a simple controller for a klipper-enabled 3D printer. It requires the Moonraker server as well.

## Features

- View all the reported temperatures and change their targets, for both heaters and temperature fans.
- Home X, Y and Z axes (I do not have another type of printer to test)
- View the position of X Y and Z axes
- Do a quad gantry leveling procedure if your printer supports it
- Show the status of the printer (printing or not, homed, QGL, filament sensors, steppers active, system load)
- View this help text

## Shortcuts
Some shortcuts have the first letter highlighted, representing the key to be pressed to trigger the action.

Most shortcuts are displayed in the app footer, the number represents the function key to press to launch the action.
For example, press the `F2` key to exit the app. The will popup a confirmation dialog. press `q` to quit, or `c` to cancel.
If you have a mouse, you can also click on the shortcuts as if they were buttons to trigger the action.

You can also press `escape` to exit any modal window that may be open to return to the main screen.

When you are in another screen, regular shortcuts are disabled. The only one that will always function is the `F8` key that will trigger an emergency stop. 
";


pub fn draw_main_help<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
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
    let t_title = Span::styled(format!("{: ^width$}", "Main help", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(markdown::parse(MAIN_HELP_TEXT))
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


pub fn draw_main_tab<'a, B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let temp_size = 2 * (app.printer.status.heaters.len() + app.printer.status.temperature_fans.len()) as u16 + 1;
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Min(6),     // Job history
                Constraint::Length(temp_size),
                Constraint::Max(1),     // Tab Footer
            ]
            .as_ref(),
        )
        .split(area);

    let mut v: Vec<(&String, &HistoryItem)> = app.history.iter().collect();
    v.sort_by(|a, b| { b.1.end_time.total_cmp(&a.1.end_time)});
    let history:Vec<Line> = v.iter().map(|i| { 
        let status = match i.1.status.as_str() {
            "cancelled" => " ✕ ",
            "completed" => " ✔ ",
            _ => " ? ",
        };
        let status_bg = match i.1.status.as_str() {
            "cancelled" => Color::Red,
            "completed" => Color::Green,
            _ => Color::Gray,
        };
        let (hours, remainder) = (i.1.total_duration.round() as i64 / (60*60), i.1.total_duration.round() as i64 % (60*60));
        let (minutes, seconds) = (remainder / 60, remainder % 60);
        let mut time_str = format!("{}m{}s", minutes, seconds);

        if hours > 0 {
            time_str = format!("{}h{}", hours, time_str);
        }
        vec![
            Line::from(
                vec![
                    Span::styled(format!("{}", i.1.filename), Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
                    Span::styled(format!("{}", " ".repeat(area.width as usize - 3 - i.1.filename.len())), Style::default()),
                    Span::styled(format!("{}", status), Style::default().bg(status_bg).fg(Color::White)),
                ]
            ),
            Line::from(format!("Filament: {:.0}mm Duration: {}", i.1.filament_used, time_str)),
        ]
    }).flatten().collect();

    let t_title = Span::styled(format!("{: ^width$}", "Job history", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(history)
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
        );
    f.render_widget(p, chunks[0]);

    let mut h: Vec<Line<'a>> = vec![];

    for heater in app.printer.status.heaters.clone() {
        let mut l = heater_text(heater);
        h.append(&mut l);
    }
    for heater in app.printer.status.temperature_fans.clone() {
        let mut l = temp_fan_text(heater);
        h.append(&mut l);
    }
    let t_title = Span::styled(format!("{: ^width$}", "Temperatures", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(h)
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
        );
    f.render_widget(p, chunks[1]);



    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[2], buttons);
    

}

fn heater_text<'a>(heater: (String, Heater)) -> Vec<Line<'a>> {
    let pow = (heater.1.power * 40.0) as usize;

    let text = vec![
        Line::from(vec![
            Span::styled(format!("{: <15}", heater.0.replace("_", " ")), Style::default().add_modifier(Modifier::BOLD).fg(Color::Magenta)),
            Span::styled(format!("{:3.2}°C", heater.1.temperature), Style::default().fg(Color::White)),
            Span::styled(" Target: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:3.0}°C", heater.1.target), Style::default().fg(Color::White)),
        ]).alignment(Alignment::Center),
        Line::from(vec![
            Span::styled("[", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <15.15}", "|".repeat(pow)), Style::default().add_modifier(Modifier::BOLD).fg(Color::Indexed(40))),
            Span::styled(format!("{: <15.15}", "|".repeat(if pow > 15 {pow-15} else {0})), Style::default().fg(Color::Indexed(214)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <10.10}", "|".repeat(if pow > 30 {pow-30} else {0})), Style::default().fg(Color::Indexed(196)).add_modifier(Modifier::BOLD)),
            Span::styled("]", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]).alignment(Alignment::Center),
    ];
    text
}

fn temp_fan_text<'a>(temp_fan: (String, TemperatureFan)) -> Vec<Line<'a>> {
    let pow = (temp_fan.1.speed * 40.0) as usize;

    let text = vec![
        Line::from(vec![
            Span::styled(
                format!("{: <15}", temp_fan.0.replace("temperature_fan ", "").replace("_", " ")), 
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Indexed(208))
            ),
            Span::styled(format!("{:3.2}°C", temp_fan.1.temperature), Style::default().fg(Color::White)),
            Span::styled(" Target: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:3.0}°C", temp_fan.1.target), Style::default().fg(Color::White)),
        ]).alignment(Alignment::Center),
        Line::from(vec![
            Span::styled("[", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <15.15}", "|".repeat(pow)), Style::default().add_modifier(Modifier::BOLD).fg(Color::Indexed(40))),
            Span::styled(format!("{: <15.15}", "|".repeat(if pow > 15 {pow-15} else {0})), Style::default().fg(Color::Indexed(214)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <10.10}", "|".repeat(if pow > 30 {pow-30} else {0})), Style::default().fg(Color::Indexed(196)).add_modifier(Modifier::BOLD)),
            Span::styled("]", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]).alignment(Alignment::Center),
    ];
    text
}

