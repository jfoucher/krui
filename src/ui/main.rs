use std::time::SystemTime;

use chrono::{DateTime, Local};

use tui::{Frame, prelude::*, widgets::{Paragraph, Block, Borders, Wrap, ListItem, List, Table, Row, Padding}};


use crate::{app::{App, InputMode, HistoryItem}, button::{Button, action_button}, printer::{Heater, HeaterType}};
use crate::markdown;
use crate::ui::header;
use viuer::{print_from_file, Config};
use super::modal;

const MAIN_HELP_TEXT: &str = "
This is the main tab. When the printer is idle, the top panel displays the history of past prints.
When the printer is printing, it displays the current print status.

The panel below that show the heater temperatures as well as the temperature fans temperatures if some are defined in your configuration.

## Print history
Each item shows the filename, the filament used and the duration of the last print for that file. The end status of the print is also shown at the right. ✔ means the print suceeded. ✕ means the print failed or was cancelled.
You can select an item by using the up and down arrow keys. Pressing enter will open a confirmation dialog to start a print for that file.

## Temperatures
Use the <TAB> key to move between the history panel and the temperatures panel. The temperatures panel shows the current temperature of each heater as well as the target temperature. The bar below shows the power of the heater. The color of the title indicates the type of heater. Magenta for a heater, red for a temperature fan.
Arrow keys can be used to select a heater.
Pressing <Enter> on a selected heater will open a dialog to set the target temperature for that heater.

## Printing
The panel displayed while printing, shows the name of the current file, the print progress and an estimation of the ETA for the print.
The current layer, the toolhead speed, the filament used and the flow are also displayed.
You will also be able to see the print preview if one is available for the file being printed.

You can press F10 at any time to trigger an emergency stop. This will stop the print and disconnect from the printer.
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
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new("Webcam".to_string(), Some("6".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
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

    // only show history if not printing
    if app.printer.status.print_state != "printing" {
        let mut v = app.history.items.clone();
        v.sort_by(|a, b| { b.end_time.total_cmp(&a.end_time)});

        // TODO add item to history after print ends
        let history:Vec<ListItem> = v.iter().enumerate().map(|(i, item)| { 
            render_history(i, item, chunks[0], app)
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
    } else {
        let fl = if app.printer.toolhead.extruder_velocity > 0.0 { app.printer.toolhead.extruder_velocity } else { 0.0 };
        let flow = fl * (1.75/2.0)*(1.75/2.0) * 3.14159;
        let mut layer = 0;
        let mut total_layers = 0;


        let mut total_duration = 0.0;
        let mut filament_used = 0.0;
        let mut filename = "Unknown".to_string();
        let mut progress = 0.0;
        let mut estimate: f64 = 0.0;
        let mut slicer_estimate = 0.0;
        let mut eta = SystemTime::now();
        let mut speed = 0.0;

        // TODO handle print paused
        if let Some(current_print) = &app.printer.current_print {
            layer = if current_print.current_layer > 0 { current_print.current_layer } else { 
                ((app.printer.toolhead.position.z - current_print.file.first_layer_height as f64) / current_print.file.layer_height as f64 + 1.0).ceil() as i64
            };
            total_layers = if current_print.total_layers > 0 { current_print.total_layers } else { 
                ((current_print.file.object_height - current_print.file.first_layer_height as f64) / current_print.file.layer_height as f64 + 1.0).ceil() as i64
            };

            speed = app.printer.toolhead.speed;

            let print_duration = current_print.print_duration;
            total_duration = current_print.total_duration;
            filament_used = if current_print.filament_used > 0.0 { current_print.filament_used } else { 0.0 };
            filename = current_print.filename.clone();
            progress = if current_print.progress > 0.0 { current_print.progress } else { 0.0 };
            slicer_estimate = current_print.file.estimated_time - print_duration;
            // state.print_stats.print_duration / getters.getPrintPercent - state.print_stats.print_duration
            estimate = if progress > 0.0 { print_duration / progress - print_duration } else { 0.0 };
            eta = SystemTime::now() + std::time::Duration::from_secs(slicer_estimate.round() as u64);

            let max_height = chunks[0].height as u32 - 5;
            let w = max_height * 4 / 3;
            let conf = Config {
                height: Some(max_height),
                x: ((chunks[0].width as i32 ) / 2 - w as i32) as u16,
                y: 6,
                transparent: false,
                ..Default::default()
            };
        
        
            match print_from_file(current_print.image.clone(), &conf) {
                Ok(_) => {},
                Err(_) => {},
            };
        }
        let datetime: DateTime<Local> = eta.into();


        let t_title = Span::styled(format!("{: ^width$}", format!("Printing {} ({:.0}%)", filename, progress*100.0), width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));


        
        let p = Table::new(vec![
            Row::new(vec![
                Line::from("Layer").alignment(Alignment::Center),
                Line::from("Speed").alignment(Alignment::Center),
                Line::from("Filament").alignment(Alignment::Center),
                Line::from("Flow").alignment(Alignment::Center),
            ]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Row::new(vec![
                Line::from(format!("{}/{} ", layer, total_layers)).alignment(Alignment::Center),
                Line::from(format!("{:.0}mm/s", speed)).alignment(Alignment::Center),
                Line::from(format!("{:.0}mm", filament_used)).alignment(Alignment::Center),
                Line::from(format!("{:.1}mm3/s", flow)).alignment(Alignment::Center),
            ]),

            Row::new(vec![
                Line::from("Estimate").alignment(Alignment::Center),
                Line::from("Slicer").alignment(Alignment::Center),
                Line::from("Total").alignment(Alignment::Center),
                Line::from("ETA").alignment(Alignment::Center),
            ]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Row::new(vec![
                Line::from(format!("{}", time_string_from_seconds(estimate.round() as i64))).alignment(Alignment::Center),
                Line::from(format!("{}", time_string_from_seconds(slicer_estimate.round() as i64))).alignment(Alignment::Center),
                Line::from(format!("{}", time_string_from_seconds(total_duration.round() as i64))).alignment(Alignment::Center),
                Line::from(format!("{}", datetime.format("%H:%M"))).alignment(Alignment::Center),
            ]),
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
    }

    

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

        let ok: Button = Button::new("OK".to_string(), Some("󰌑 ".to_string()));
        let cancel = Button::new("Cancel".to_string(), Some("󱊷 ".to_string()));
        let btn = Table::new(vec![
            Row::new(vec![
                Line::from(action_button(ok)).alignment(Alignment::Left),
                Line::from(action_button(cancel)).alignment(Alignment::Right),
            ])
        ])
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .block(Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(2))
        )
        ;

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
    if app.printer.will_print_file != None && app.printer.status.print_state != "printing".to_string() {
        let title = Paragraph::new(
            Line::from(vec![
                Span::styled("Confirm print start", Style::default().add_modifier(Modifier::BOLD))
            ]).alignment(Alignment::Center)
        );

        let text = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("This will start a print of ", Style::default()),
                Span::styled(app.printer.will_print_file.clone().unwrap().filename, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                
            ]),
            Line::from(vec![
                Span::styled("Estimated time is ", Style::default()),
                Span::styled(format!("{}", time_string_from_seconds(app.printer.will_print_file.clone().unwrap().estimated_time.round() as i64)), Style::default().add_modifier(Modifier::BOLD)),
            ]),
        ]
        );
        
        let ok: Button = Button::new("OK".to_string(), Some("󰌑 ".to_string()));
        let cancel = Button::new("Cancel".to_string(), Some("󱊷 ".to_string()));
        let btn = Table::new(vec![
            Row::new(vec![
                Line::from(action_button(ok)).alignment(Alignment::Left),
                Line::from(action_button(cancel)).alignment(Alignment::Right),
            ])
        ])
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .block(Block::default()
            .borders(Borders::NONE)
            .padding(Padding::horizontal(2))
        )
        ;

        modal(f, title, text, btn, None);
    }

    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new("Webcam".to_string(), Some("6".to_string())),
        Button::new(if app.printer.connected {"STOP".to_string()} else {"Restart".to_string()}, Some("10".to_string())),
    ];
    header::draw_footer(f, chunks[3], buttons);
    

}

fn render_history<'a>(i: usize, item: &HistoryItem, area: Rect, app: &mut App) -> ListItem<'a> {
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
        "klippy_shutdown" => Color::Red,
        "cancelled" => Color::Yellow,
        "completed" => Color::Green,
        _ => Color::Gray,
    };

    let time_str = time_string_from_seconds(item.total_duration.round() as i64);

    let fg = if selected {Color::DarkGray} else {Color::Gray};
    let bg = if selected {Color::Gray} else {Color::DarkGray};
    ListItem::new(
        vec![
            Line::from(
                vec![
                    Span::styled(format!("{}", item.filename), Style::default().add_modifier(Modifier::BOLD).fg(fg).bg(bg)),
                    Span::styled(" ".repeat(area.width as usize - 3 - item.filename.len()), Style::default().fg(fg).bg(bg)),
                    Span::styled(format!("{}", status), Style::default().bg(status_bg).fg(Color::White)),
                ]
            ),
            Line::from(
                vec![
                    Span::styled(format!("Filament: {:.0}mm Duration: {} {w: >w$}", item.filament_used, time_str, w = area.width as usize), Style::default().fg(fg).bg(bg))
                ]
            ),
        ]
    )
}

fn time_string_from_seconds(seconds: i64) -> String {
    let (hours, remainder) = (seconds / (60*60), seconds % (60*60));
    let (minutes, seconds) = (remainder / 60, remainder % 60);
    let mut time_str = format!("{}m{:0>2}s", minutes, seconds);

    if hours > 0 {
        time_str = format!("{}h{}", hours, time_str);
    }
    time_str
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
