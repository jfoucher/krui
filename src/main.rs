

use krui::app::{App, AppResult};
use krui::event::{Event, EventHandler};
use krui::handler::handle_key_events;
use krui::tui::Tui;


use std::{io, env};

use tui::backend::CrosstermBackend;
use tui::Terminal;

use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

fn main() -> AppResult<()> {
    let args: Vec<String> = env::args().collect();
    // Create an application.
    if args.len() < 2 {
        println!("Usage: krui <server url>");
        return Ok(());
    }
    let mut app = App::new(args[1].clone());

    // Initialize log writer
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("output.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder()
                   .appender("logfile")
                   .build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(100);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    app.init();
    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;

    Ok(())
}
