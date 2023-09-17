use std::{fs::{self, File}, io::Write};

use chrono::Timelike;
use curl::easy::Easy;
use tui::{widgets::{Borders, Paragraph, Block, Wrap, Scrollbar, ScrollbarOrientation}, prelude::*};
use viuer::{print_from_file, Config};

use crate::{ui::header, button::Button, markdown, app::{App, InputMode}, printer::GCodeLine};

const TAB_HELP_TEXT: &str = "
# Webcam Help

Using this tab, you can send any GCODE command to the printer and see the response.
Messages from the printer are displayed in a scrollable text view. Scroll the view using the arrow keys, while the GCODE input field is deselected.
Toggle the GCODE input field using the Tab key.
Press Enter to send the command to the printer.
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


    let t_title = Span::styled(
        format!("{: ^width$}", "Webcam", width = f.size().width as usize),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White)
            .bg(Color::Magenta)
    );


    let p = Paragraph::new("text")
        .block(Block::default()
            .title(t_title)
            .borders(Borders::NONE)
        )
    ;

    f.render_widget(p, chunks[0]);
    // Download and render stream snapshot
    let mut easy = Easy::new();
    //log::info!("webcams {:?}", app.printer.webcams);
    if let Some(w) = app.printer.webcams.get(0) {
        let mut wc = w.clone();
        if wc.render % 10 == 0 {
            easy.url(format!("http://{}{}", app.server_url.clone(), wc.snapshot_url.clone()).as_str()).unwrap();

            let _ = fs::create_dir_all("cache/stream/");
            let filepath = "cache/stream/1.jpg";
            let file = match File::create(filepath.clone()) {
                Ok(f) => Some(f),
                Err(e) => {
                    log::error!("Error creating file {:?}", e);
                    None
                }
            };
            if let Some(mut handle) = file {
                let mut transfer = easy.transfer();
                transfer.write_function(|data| {
                    // write data to a temporary file
                    //log::info!("Writing {} bytes to file {:?}", data.len(), filepath);
                    handle.write_all(data).unwrap();
                    
                    Ok(data.len())
                }).unwrap();
                transfer.perform().unwrap();
            }
            let max_height = chunks[0].height as u32 - 5;
            let w = max_height * 4 / 3;
            let conf = Config {
                height: Some(max_height),
                x: ((chunks[0].width as i32 ) / 2 - w as i32) as u16,
                y: 2,
                transparent: false,

                ..Default::default()
            };
        
            match print_from_file(filepath, &conf) {
                Ok(_) => {},
                Err(_) => {},
            };
        }

        wc.render = wc.render.wrapping_add(1);

        app.printer.webcams[0] = wc;
    }







    let buttons = vec![
        Button::new("Help".to_string(), Some("1".to_string())),
        Button::new("Quit".to_string(), Some("2".to_string())),
        Button::new("Toolhead".to_string(), Some("3".to_string())),
        Button::new("Extruder".to_string(), Some("4".to_string())),
        Button::new("Console".to_string(), Some("5".to_string())),
        Button::new("Close".to_string(), Some("6".to_string())),
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
    let t_title = Span::styled(format!("{: ^width$}", "Webcam help", width = f.size().width as usize), Style::default().add_modifier(Modifier::BOLD).fg(Color::White).bg(Color::Magenta));
    let p = Paragraph::new(markdown::parse(TAB_HELP_TEXT))
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


