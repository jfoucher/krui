use crate::app::{App, AppResult, Tab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        // Counter handlers
        KeyCode::F(1) => {
            app.current_tab = match app.current_tab {
                Tab::Main => Tab::Help,
                Tab::Help => Tab::Main,
                Tab::Toolhead => Tab::ToolheadHelp,
                Tab::ToolheadHelp => Tab::Toolhead,
                Tab::Extruder => Tab::ExtruderHelp,
                Tab::ExtruderHelp => Tab::Extruder,
                _ => Tab::Main
            }
        }
        KeyCode::F(2) => {
            app.quit();
        }
        KeyCode::F(3) => {
            app.current_tab = match app.current_tab {
                Tab::Main => Tab::Toolhead,
                _ => Tab::Main
            }
        }

        KeyCode::F(4) => {
            app.current_tab = match app.current_tab {
                Tab::Main => Tab::Extruder,
                _ => Tab::Main
            }
        }
        KeyCode::F(10) => {
            if app.printer.connected {
                app.send_message("printer.emergency_stop".to_string(), serde_json::Value::Object(serde_json::Map::new()))
            } else {
                app.send_message("printer.gcode.script".to_string(), json!({"script": "FIRMWARE_RESTART"}));
                app.printer.stats.state = "starting".to_string();
            }
            
        }
        
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
