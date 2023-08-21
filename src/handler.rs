use crate::app::{App, AppResult, Tab, MainTabWidget};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        KeyCode::Esc => {
            match app.current_tab {
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::Temperatures => {
                        },
                        MainTabWidget::History => {
                            // Cancel printing
                            app.printer.printing_file = None;
                        },
                    }
                },
                _ => {},
            }
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
                Tab::Console => Tab::ConsoleHelp,
                Tab::ConsoleHelp => Tab::Console,
            }
        }
        KeyCode::F(2) => {
            app.quit();
        }
        KeyCode::F(3) => {
            app.current_tab = match app.current_tab {
                Tab::Toolhead => Tab::Main,
                _ => Tab::Toolhead,
            }
        }

        KeyCode::F(4) => {
            app.current_tab = match app.current_tab {
                Tab::Extruder => Tab::Main,
                _ => Tab::Extruder,
            }
        }

        KeyCode::F(5) => {
            app.current_tab = match app.current_tab {
                Tab::Console => Tab::Main,
                _ => Tab::Console,
            }
        }
        KeyCode::F(10) => {
            if app.printer.connected {
                app.emergency_stop();
            } else {
                app.send_message("printer.gcode.script".to_string(), json!({"script": "FIRMWARE_RESTART"}));
                app.printer.status.state = "starting".to_string();
            }   
        },
        KeyCode::Down | KeyCode::Char('j') => {
            match app.current_tab {
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::History => {
                            app.history.next();
                        },
                        MainTabWidget::Temperatures => {
                            app.printer.status.heaters.next();
                        },
                    }
                },
                _ => {},
            }
            ;
        },
        KeyCode::Up | KeyCode::Char('k') => {
            match app.current_tab {
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::History => {
                            app.history.previous();
                        },
                        MainTabWidget::Temperatures => {
                            app.printer.status.heaters.previous();
                        },
                    }
                },
                _ => {},
            }
        },
        KeyCode::Tab => {
            app.selected_widget = match app.selected_widget {
                MainTabWidget::History => MainTabWidget::Temperatures,
                MainTabWidget::Temperatures => MainTabWidget::History,
            };

            log::info!("selected_widget {:?}", app.selected_widget);

            match app.selected_widget {
                MainTabWidget::Temperatures => {
                    app.history.unselect();
                    app.printer.status.heaters.state.select(Some(0));
                },
                MainTabWidget::History => {
                    app.printer.status.heaters.unselect();
                    app.history.state.select(Some(0));
                },
            }
        },
        KeyCode::Enter => {
            match app.current_tab {
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::Temperatures => {
                            // TODO set target temperature
                        },
                        MainTabWidget::History => {
                            // If a history item is selected
                            if let Some(sel) = app.history.state.selected() {
                                // set the file to print
                                app.printer.printing_file = Some(app.history.items[sel].clone().filename);
                                // deselect history item
                                app.history.state.select(None);
                            } else {
                                // No history item is selected, which means we are on the confirmation dialog
                                if let Some(file) = &app.printer.printing_file {
                                    app.send_message("printer.print.start".to_string(), json!({"filename": file}));
                                    app.printer.printing_file = None;
                                }
                                
                                
                            }
                        },
                    }
                },
                _ => {},
            }
            
        }
        
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
