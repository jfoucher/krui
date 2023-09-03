use tui::{Frame, prelude::*, widgets::{Paragraph, Block, Borders, Wrap, ListItem, List, Clear, BorderType, Padding}};

use crate::{app::{App, InputMode}, button::Button, printer::{Heater, HeaterType}};
use crate::markdown;
use crate::ui::header;

use super::modal;

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



pub fn draw_main_help<'a, B>(f: &mut Frame<B>, _: &mut App, area: Rect)
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
    let temp_size = 2 * (app.printer.status.heaters.items.len()) as u16 ;
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Min(6),     // Job history
                Constraint::Length(1),     // title
                Constraint::Length(temp_size),
                Constraint::Max(1),     // Tab Footer
            ]
            .as_ref(),
        )
        .split(area);

    let mut v = app.history.items.clone();
    v.sort_by(|a, b| { b.end_time.total_cmp(&a.end_time)});

    // let v = vec![1,2,3];
    let history:Vec<ListItem> = v.iter().enumerate().map(|(i, item)| { 
        let status = match item.status.as_str() {
            "cancelled" | "klippy_shutdown" => " ✕ ",
            "completed" => " ✔ ",
            _ => " ? ",
        };
        let mut selected = false;
        if let Some(sel) = app.history.state.selected() {
            selected = sel == i;
        }
        let status_bg = match item.status.as_str() {
            "cancelled" => Color::Red,
            "completed" => Color::Green,
            _ => Color::Gray,
        };
        let (hours, remainder) = (item.total_duration.round() as i64 / (60*60), item.total_duration.round() as i64 % (60*60));
        let (minutes, seconds) = (remainder / 60, remainder % 60);
        let mut time_str = format!("{}m{}s", minutes, seconds);

        if hours > 0 {
            time_str = format!("{}h{}", hours, time_str);
        }
        let fg = if selected {Color::DarkGray} else {Color::Gray};
        let bg = if selected {Color::Gray} else {Color::DarkGray};
        ListItem::new(
            vec![
                Line::from(
                    vec![
                        Span::styled(format!("{}", item.filename), Style::default().add_modifier(Modifier::BOLD).fg(fg).bg(bg)),
                        Span::styled(format!("{}", " ".repeat(area.width as usize - 3 - item.filename.len())), Style::default().fg(fg).bg(bg)),
                        Span::styled(format!("{}", status), Style::default().bg(status_bg).fg(Color::White)),
                    ]
                ),
                Line::from(
                    vec![
                        Span::styled(format!("Filament: {:.0}mm Duration: {} {w: >w$}", item.filament_used, time_str, w = f.size().width as usize), Style::default().fg(fg).bg(bg))
                    ]
                ),
            ]
        )
    }).collect();



    let t_title = Span::styled(format!("{: ^width$}", "Job history", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));

    let p = List::new(history)
        .block(Block::default()
            .title(t_title)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
        )
        ;

    f.render_stateful_widget(p, chunks[0], &mut app.history.state);


    let t_title = Span::styled(format!("{: ^width$}", "Temperatures", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));

    f.render_widget(Paragraph::new(t_title), chunks[1]);

    let heaters_text: Vec<ListItem> = app.printer.status.heaters.items.iter().enumerate().map(|(i, h)| {
        let mut selected = false;
        if let Some(sel) = app.printer.status.heaters.state.selected() {
            selected = sel == i;
        }
        heater_text(h.clone(), selected, f.size().width)
    }).collect();

    let bottom = Layout::default()
    .direction(Direction::Horizontal)
    .margin(0)
    .constraints(
        [
            Constraint::Length((chunks[1].width-42)/2),     // spacer
            Constraint::Length(42),
            Constraint::Length((chunks[1].width-42)/2),     // Spacer
        ]
        .as_ref(),
    )
    .split(chunks[2]);

    let p = List::new(heaters_text)
        .block(Block::default()
            .borders(Borders::NONE)
        )
        ;

    f.render_stateful_widget(p, bottom[1], &mut app.printer.status.heaters.state);

    // Changing heater temperature
    if let Some(heater) = &app.selected_heater {
        // sl is a paragraph with an input allowing to set the heater temperature
        let hn = heater.name.replace("temperature_fan ", "").replace("_", " ");
        let title = Paragraph::new(
            Line::from(vec![
                Span::styled("Set ", Style::default()),
                Span::styled(&hn, Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" temperature", Style::default()),
            ]).alignment(Alignment::Center)
            // format!("{: ^50}\n{}", format!("Set {} temperature", hn), 
            //     format!("Enter the new temperature for the {} heater", hn)
        ).block(Block::default()
        ).wrap(Wrap {trim: false});
        
        let text = Paragraph::new(
            format!("Enter the new temperature for the {} heater", hn)).block(Block::default()
        );


        let input = Paragraph::new(app.temperature_input.value.as_str())
        .style(match app.console_input.mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .blue()
        .block(Block::default().borders(Borders::ALL).title("Temperature"));
        let btn = Paragraph::new(
            Line::from(vec![
                Span::styled(format!("     {: <19}", "Enter<OK>"), Style::default().bg(Color::White).fg(Color::Black)),
                Span::styled(format!("{: >19}     ", "Esc<Cancel>"), Style::default().bg(Color::White).fg(Color::Black)),
            ])
        );

        let chunks = modal(f, title, text, btn, Some(input));
        
        match app.temperature_input.mode {
            InputMode::Normal => {}
            InputMode::Editing => {
                f.set_cursor(
                    chunks[2].x + app.temperature_input.cursor_position as u16 + 1,
                    chunks[2].y + 1,
                )
            }
        }
    }


    // Starting a print
    if app.printer.printing_file != None && app.printer.status.print_state != "printing".to_string() {
        let title = Paragraph::new(
            Line::from(vec![
                Span::styled("Confirm print start", Style::default().add_modifier(Modifier::BOLD))
            ]).alignment(Alignment::Center)
        );

        let text = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("{: <48}", "This will start a print of "), Style::default()),
            ]),
            Line::from(vec![
                Span::styled(format!("{: ^48}", app.printer.printing_file.clone().unwrap()), Style::default().add_modifier(Modifier::BOLD)),
            ]),
        ]
        );
        
        let buttons = Paragraph::new(
            Line::from(vec![
                Span::styled(format!("     {: <19}", "Enter<OK>"), Style::default().bg(Color::White).fg(Color::Black)),
                Span::styled(format!("{: >19}     ", "Esc<Cancel>"), Style::default().bg(Color::White).fg(Color::Black)),
            ]),
        );

        modal(f, title, text, buttons, None);
    }

    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[3], buttons);
    

}

fn heater_text<'a>(heater: Heater, selected: bool, width: u16) -> ListItem<'a> {
    let pow = (heater.power * 40.0) as usize;

    let fg = if selected {Color::DarkGray} else {Color::Gray};
    let bg = if selected {Color::Gray} else {Color::DarkGray};

    let text = ListItem::new(vec![
        Line::from(vec![
            Span::styled(format!("{: <15}", heater.name.replace("temperature_fan ", "").replace("_", " ")), Style::default().add_modifier(Modifier::BOLD).fg(if heater.heater_type == HeaterType::Heater { Color::Magenta } else { Color::Red }).bg(bg)),
            Span::styled(format!("{:3.2}°C ", heater.temperature), Style::default().fg(fg).bg(bg)),
            Span::styled(" Target: ", Style::default().fg(Color::Cyan).bg(bg)),
            Span::styled(format!("{:3.0}°C {w: >w$}", heater.target, w=width as usize), Style::default().fg(fg).bg(bg)),
        ]).alignment(Alignment::Center),
        Line::from(vec![
            Span::styled("[", Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <15.15}", "|".repeat(pow)), Style::default().add_modifier(Modifier::BOLD).fg(Color::Indexed(40)).bg(bg)),
            Span::styled(format!("{: <15.15}", "|".repeat(if pow > 15 {pow-15} else {0})), Style::default().fg(Color::Indexed(214)).bg(bg).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{: <10.10}", "|".repeat(if pow > 30 {pow-30} else {0})), Style::default().fg(Color::Indexed(196)).bg(bg).add_modifier(Modifier::BOLD)),
            Span::styled("]", Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)),
        ]).alignment(Alignment::Center),
    ]);
    text
}
