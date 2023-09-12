use crate::{app::{App, AppResult, Tab, MainTabWidget, InputMode}, printer::HeaterType};
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
                            app.selected_heater = None;
                        },
                        MainTabWidget::History => {
                            // Cancel printing
                            app.printer.will_print_file = None;
                        },
                    }
                },
                Tab::Console => {
                    if app.console_input.mode == InputMode::Editing {
                        app.console_input.mode = InputMode::Normal;
                    }
                },
                _ => {},
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
        KeyCode::Down => {
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
                Tab::Console => {
                    if app.console_input.mode == InputMode::Normal {
                        app.console_scroll= app.console_scroll.saturating_add(1);
                        app.console_scroll_state = app
                            .console_scroll_state
                            .position(app.console_scroll as u16);
                    }
                },
                _ => {},
            }
            ;
        },
        KeyCode::Up => {
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
                Tab::Console => {
                    if app.console_input.mode == InputMode::Normal {
                        app.console_scroll = app.console_scroll.saturating_sub(1);
                        app.console_scroll_state = app
                            .console_scroll_state
                            .position(app.console_scroll as u16);
                    }
                },
                _ => {},
            }
        },
        KeyCode::Tab => {
            match app.current_tab {
                Tab::Main => {
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
                Tab::Console => {
                    if app.console_input.mode == InputMode::Normal {
                        app.console_input.mode = InputMode::Editing;
                    } else {
                        app.console_input.mode = InputMode::Normal;
                    }
                },
                _ => {},
            }
        },
        KeyCode::Enter => {
            match app.current_tab {
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::Temperatures => {
                            if let Some(heater) = &app.selected_heater {
                                // save current heater temperature
                                //SET_TEMPERATURE_FAN_TARGET TEMPERATURE_FAN=exhaust_fan TARGET=30
                                //SET_HEATER_TEMPERATURE HEATER=heater_bed TARGET=25
                                if heater.heater_type == HeaterType::Heater {
                                    app.send_message(
                                        "printer.gcode.script".to_string(), 
                                        json!({"script": format!("SET_HEATER_TEMPERATURE HEATER={} TARGET={}", heater.name, app.temperature_input.value)})
                                    );
                                } else {
                                    app.send_message(
                                        "printer.gcode.script".to_string(), 
                                        json!({"script": format!("SET_TEMPERATURE_FAN_TARGET TEMPERATURE_FAN={} TARGET={}", heater.name.replace("temperature_fan ", ""), app.temperature_input.value)})
                                    );
                                }
                                
                                app.selected_heater = None;
                                app.temperature_input.value = "".to_string();
                                app.temperature_input.cursor_position = 1;
                            } else {
                                // Show a dialog to set the target temperature
                                if let Some(sel) = app.printer.status.heaters.state.selected() {
                                    app.selected_heater = Some(app.printer.status.heaters.items[sel].clone());
                                }
                            }
                        },
                        MainTabWidget::History => {
                            // If a history item is selected
                            if let Some(sel) = app.history.state.selected() {
                                // set the file to print
                                app.printer.will_print_file = Some(app.history.items[sel].clone());
                                // deselect history item
                                app.history.state.select(None);
                            } else {
                                // No history item is selected, which means we are on the confirmation dialog
                                let will_print_file = app.printer.will_print_file.clone();
                                if let Some(file) = will_print_file {
                                    app.send_message("printer.print.start".to_string(), json!({"filename": file.filename}));
                                    app.send_message("server.files.metadata".to_string(), json!({"filename": file.filename}));
                                    app.printer.will_print_file = None;
                                }
                            }
                        },
                    }
                },
                Tab::Console => {
                    if app.console_input.mode == InputMode::Editing {
                        app.send_message("printer.gcode.script".to_string(), json!({"script": app.console_input.value}));
                        app.console_input.value = "".to_string();
                        app.console_input.cursor_position = 0;
                    }
                },
                _ => {},
            }
            
        }

        KeyCode::Char(c) => {
            if c == 'c' && key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
            match app.current_tab {
                Tab::Console => {
                    if app.console_input.mode == InputMode::Editing {
                        app.console_input.value.push(c);
                        let cursor_moved_right = app.console_input.cursor_position.saturating_add(1);
                        app.console_input.cursor_position = cursor_moved_right.clamp(0, app.console_input.value.len() as u16);
                    } else {
                        if c == 'j' {
                            app.console_scroll= app.console_scroll.saturating_add(1);
                            app.console_scroll_state = app
                                .console_scroll_state
                                .position(app.console_scroll as u16);
                        } else if c == 'k' {
                            app.console_scroll= app.console_scroll.saturating_sub(1);
                            app.console_scroll_state = app
                                .console_scroll_state
                                .position(app.console_scroll as u16);
                        }
                    }
                },
                Tab::Main => {
                    match app.selected_widget {
                        MainTabWidget::Temperatures => {
                            if let Some(_) = &app.selected_heater {
                                app.temperature_input.value.push(c);
                                let cursor_moved_right = app.temperature_input.cursor_position.saturating_add(1);
                                app.temperature_input.cursor_position = cursor_moved_right.clamp(0, 1 + app.temperature_input.value.len() as u16);
                                
                            }
                        },
                        _ => {},
                    }
                }
                _ => {
                    
                }
            }
        },
        KeyCode::Backspace => {
            match app.current_tab {
                Tab::Console => {
                    let is_not_cursor_leftmost = app.console_input.cursor_position != 0;
                    if is_not_cursor_leftmost {
                        let current_index = app.console_input.cursor_position as usize;
                        let from_left_to_current_index = current_index - 1;
                        let before_char_to_delete = app.console_input.value.chars().take(from_left_to_current_index);
                        let after_char_to_delete = app.console_input.value.chars().skip(current_index);
                        app.console_input.value = before_char_to_delete.chain(after_char_to_delete).collect();
                        let cursor_moved_left = app.console_input.cursor_position.saturating_sub(1);

                        app.console_input.cursor_position = cursor_moved_left.clamp(0, app.console_input.value.len() as u16);
                    }
                },
                Tab::Main => {
                    if let Some(_) = app.selected_heater {
                        let is_not_cursor_leftmost = app.temperature_input.cursor_position != 1;
                        if is_not_cursor_leftmost {
                            let current_index = app.temperature_input.cursor_position as usize;
                            let from_left_to_current_index = current_index -2;
                            let before_char_to_delete = app.temperature_input.value.chars().take(from_left_to_current_index);
                            let after_char_to_delete = app.temperature_input.value.chars().skip(current_index);
                            app.temperature_input.value = before_char_to_delete.chain(after_char_to_delete).collect();
                            let cursor_moved_left = app.temperature_input.cursor_position.saturating_sub(1);

                            app.temperature_input.cursor_position = cursor_moved_left.clamp(1, 1 + app.temperature_input.value.len() as u16);
                        }
                    }
                }
                _ => {},
            }
        },
        
        _ => {}
    }
    Ok(())
}
