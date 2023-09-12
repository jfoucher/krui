
use std::{collections::HashMap, time::SystemTime};

use chrono::DateTime;
use itertools::Step;
use log4rs::append::rolling_file::LogFile;

use crate::{ui::stateful_list::StatefulList, app::HistoryItem};


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Toolhead {
    pub position: Position,
    pub homed: Homed,
    pub fan: Fan,
    pub speed: f64,
    pub extruder_velocity: f64,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Homed {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub qgl: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct FileMetadata {
    pub size: u64,
    pub slicer: String,
    pub layer_height: f64,
    pub first_layer_height: f64,
    pub object_height: f64,
    pub filament_total: f64,
    pub estimated_time: f64,

}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PrintStats {
    pub filename: String,
    pub total_duration: f64,
    pub print_duration: f64,
    pub filament_used: f64,
    pub total_layers: i64,
    pub current_layer: i64,
    pub progress: f64,
    pub file: FileMetadata,
}

impl PrintStats {
    pub fn new() -> PrintStats {
        PrintStats {
            filename: "".to_string(),
            total_duration: 0.0,
            print_duration: 0.0,
            filament_used: 0.0,
            total_layers: 0,
            current_layer: 0,
            progress: 0.0,
            file: FileMetadata {
                size: 0,
                slicer: "".to_string(),
                layer_height: 0.0,
                first_layer_height: 0.0,
                object_height: 0.0,
                filament_total: 0.0,
                estimated_time: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Fan {
    #[serde(default = "default_float")]
    pub speed: f64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum HeaterType {
    Heater,
    TemperatureFan,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Heater {
    pub name: String,
    #[serde(default = "default_float")]
    pub temperature: f64,
    #[serde(default = "default_float")]
    pub target: f64,
    #[serde(default = "default_float")]
    pub power: f64,
    pub heater_type: HeaterType,
}

pub fn default_float() -> f64 {
    return 0.0;
}

#[derive(Debug, Clone)]
pub struct GCodeLine {
    pub timestamp: DateTime<chrono::Local>,
    pub content: String,
}
#[derive(Debug, Clone)]
pub struct PrinterStatus {
    pub heaters: StatefulList<Heater>,
    pub state: String,
    pub print_state: String,
    pub state_message: String,
    pub stepper_enable: bool,
    pub filament_switch: bool,
    pub gcodes: Vec<GCodeLine>,
}

#[derive(Debug, Clone)]
pub struct Printer {
    pub connected: bool,
    pub status: PrinterStatus,
    pub toolhead: Toolhead,
    pub sysload: f64,
    pub will_print_file: Option<HistoryItem>,
    pub current_print: Option<PrintStats>,
}

impl Printer {
    pub fn new() -> Printer {
        Printer {
            connected: false,
            status: PrinterStatus {
                heaters: StatefulList::with_items(vec![]),
                state: String::from("unknown"),
                print_state: String::from("unknown"),
                state_message: "".to_string(),
                stepper_enable: false,
                filament_switch: false,
                gcodes: vec![],
            },
            toolhead: Toolhead {
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }, 
                homed: Homed { x: false, y: false, z: false, qgl: false },
                fan: Fan { speed: 0.0 },
                speed: 0.0,
                extruder_velocity: 0.0,
            },
            sysload: 0.0,
            will_print_file: None,
            current_print: None,
        }
    }

    pub fn update(&mut self, data: serde_json::Value) {
        if let Some(motion) = data.get("motion_report") {
            log::info!("motion: {:?}", motion);
            if let Some(position) = motion.get("live_position") {
                if let Some(x) = position.get(0) {
                    self.toolhead.position.x = x.as_f64().unwrap();
                }
                if let Some(y) = position.get(1) {
                    self.toolhead.position.y = y.as_f64().unwrap();
                }
                if let Some(z) = position.get(2) {
                    self.toolhead.position.z = z.as_f64().unwrap();
                }
            }
            if let Some(speed) = motion.get("live_velocity") {
                if let Some(s) = speed.as_f64() {
                    self.toolhead.speed = s;
                }
            }

            if let Some(speed) = motion.get("live_extruder_velocity") {
                if let Some(s) = speed.as_f64() {
                    self.toolhead.extruder_velocity = s;
                }
            }
        }

        if let Some(heaters) = data.get("heaters") {
            if let Some(available_heaters) = heaters.get("available_heaters") {
                // add heater to HashMap of heaters, with blank heaters
                for h in available_heaters.as_array().unwrap() {
                    let heater_name: String = serde_json::from_value(h.clone()).unwrap();
                    if let None = self.status.heaters.items.iter().find(|h| {h.name == heater_name}) {
                        self.status.heaters.add(Heater { name: heater_name, temperature: 0.0, target: 0.0, power: 0.0, heater_type: HeaterType::Heater });
                    }
                }
            }
            if let Some(available_sensors) = heaters.get("available_sensors") {
                // add temperature fans to HashMap of temperature fans, with blank data
                for h in available_sensors.as_array().unwrap() {
                    if let Some(s) = h.as_str() {
                        if !String::from(s).contains("temperature_fan") {
                            continue;
                        }
                    }
                    let heater_name: String = serde_json::from_value(h.clone()).unwrap();
                    if let None = self.status.heaters.items.iter().find(|h| {h.name == heater_name}) {
                        self.status.heaters.add(Heater { name:heater_name, temperature: 0.0, target: 0.0, power: 0.0, heater_type: HeaterType::TemperatureFan });
                    }
                }
            }
        }
        let mut new_heaters = self.status.heaters.items.clone();
        // For each heater, check if we have data to set their values
        for (i, heater) in self.status.heaters.items.iter().enumerate() {
            // try to get current heater from data
            let mut nh = heater.clone();
            if let Some(heater_data) = data.get(nh.name.clone()) {
                if let Some(temp) = heater_data.get("temperature") {
                    nh.temperature = temp.as_f64().unwrap();
                }
                if let Some(pow) = heater_data.get("power") {
                    nh.power = pow.as_f64().unwrap();
                }
                if let Some(pow) = heater_data.get("speed") {
                    nh.power = pow.as_f64().unwrap();
                }
                if let Some(t) = heater_data.get("target") {
                    nh.target = serde_json::from_value(t.clone()).unwrap();
                }
                new_heaters[i] = nh;
            }
        }
        self.status.heaters.items = new_heaters;



        let mut status = self.status.clone();
        if let Some(print_stats) = data.get("webhooks") {
            if let Some(state) = print_stats.get("state") {
                if let Some(s) = state.as_str() {
                    status.state = s.to_string();
                }
            }
            if let Some(state_msg) = print_stats.get("state_message") {
                if let Some(s) = state_msg.as_str() {
                    status.state_message = s.to_string();
                }
            }
        }
        if let Some(sdcard) = data.get("virtual_sdcard") {
            if let Some(state) = sdcard.get("progress") {
                if let Some(s) = state.as_f64() {

                    if let Some(mut cp) = self.current_print.clone() {
                        cp.progress = s;
                        self.current_print = Some(cp);
                    }
                }
            }
        }
        if let Some(print_stats) = data.get("print_stats") {
            if let Some(state) = print_stats.get("state") {
                if let Some(s) = state.as_str() {
                    status.print_state = s.to_string();
                }
            }
            if status.print_state == "printing" {
                let mut current_print = PrintStats::new();
                
                if let Some(cp) = &self.current_print {
                    current_print = cp.clone();
                }
                
                // Set file data in main view
                if let Some(filename) = print_stats.get("filename") {
                    if let Some(f) = filename.as_str() {
                        current_print.filename = f.to_string();
                    }
                }
                if let Some(total_duration) = print_stats.get("total_duration") {
                    if let Some(f) = total_duration.as_f64() {
                        current_print.total_duration = f;
                    }
                }
                if let Some(print_duration) = print_stats.get("print_duration") {
                    if let Some(f) = print_duration.as_f64() {
                        current_print.print_duration = f;
                    }
                }
                if let Some(filament_used) = print_stats.get("filament_used") {
                    if let Some(f) = filament_used.as_f64() {
                        current_print.filament_used = f;
                    }
                }
                if let Some(info) = print_stats.get("info") {
                    log::info!("info: {:?}", info);
                    if let Some(total_layers) = info.get("total_layer") {
                        if let Some(f) = total_layers.as_i64() {
                            current_print.total_layers = f;
                        }
                    }
                    if let Some(current_layer) = info.get("current_layer") {
                        if let Some(f) = current_layer.as_i64() {
                            current_print.current_layer = f;
                        }
                    }
                }


                self.current_print = Some(current_print);
            } else {
                self.current_print = None;
            }
        }
        // TODO make this optional
        if let Some(ks) = data.as_object() {
            for (k, v) in ks {
                if k.contains("filament_switch_sensor") {
                    if let Some(fs) = v.get("filament_detected") {
                        if let Some(s) = fs.as_bool() {
                            status.filament_switch = s;
                        }
                    }
                }
            }
        }
        
        if let Some(step) = data.get("stepper_enable") {
            if let Some(steppers) = step.get("steppers") {
                if let Some(ks) = steppers.as_object() {
                    let mut e = false;
                    for (_, v) in ks {
                        if let Some(s) = v.as_bool() {
                            if s {
                                e = true;
                                break;
                            }
                        }
                    }
                    status.stepper_enable = e;
                }
            }
        }
        self.status = status;
        // Update homed axes
        let mut homed = self.toolhead.homed.clone();
        if let Some(toolhead) = data.get("toolhead") {
            if let Some(homes) = toolhead.get("homed_axes") {
                log::info!("homed_axes: {:?}", homes);
                if let Some(axes) = homes.as_str() {
                    let ax = String::from(axes);
                    if ax.contains("x") {
                        homed.x = true;
                    } else {
                        homed.x = false;
                    }
                    if ax.contains("y") {
                        homed.y = true;
                    } else {
                        homed.y = false;
                    }
                    if ax.contains("z") {
                        homed.z = true;
                    } else {
                        homed.z = false;
                    }
                }
            }
        }
        if let Some(qgl) = data.get("quad_gantry_level") {
            if let Some(a) = qgl.get("applied") {
                homed.qgl = a.as_bool().unwrap();
            }
        }
        self.toolhead.homed = homed;

        // Update part fan speed
        let mut fan = self.toolhead.fan.clone();
        if let Some(f) = data.get("fan") {
            if let Some(s) = f.get("speed") {
                if let Some(speed) = s.as_f64() {
                    fan.speed = speed;
                }
            }
        }
        self.toolhead.fan = fan;
        // Update sys load

        if let Some(f) = data.get("system_stats") {
            if let Some(s) = f.get("sysload") {
                if let Some(load) = s.as_f64() {
                    self.sysload = load;
                }
            }
        }
        if let Some(c) = data.get("connected") {
            self.connected = c.as_bool().unwrap();
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_fan_speed() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "fan": {
                "speed": 0.5
            }
        });
        p.update(data);
        assert_eq!(p.toolhead.fan.speed, 0.5);
        data = serde_json::json!({
            "fan": {
                "speed": 0.75
            }
        });
        p.update(data);
        assert_eq!(p.toolhead.fan.speed, 0.75);
    }

    #[test]
    fn test_update_fan_speed_does_not_change_other_data() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "fan": {
                "speed": 0.5
            }
        });
        p.update(data);
        assert_eq!(p.toolhead.fan.speed, 0.5);
        data = serde_json::json!({
            "fan": {
                "speed": 0.75
            }
        });
        p.update(data);
        assert_eq!(p.toolhead.fan.speed, 0.75);
        assert_eq!(p.toolhead.homed.x, false);
        assert_eq!(p.status.state, "unknown");
    }

    #[test]
    fn test_adding_new_heater_adds_heater(){
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "heaters": {
                "available_heaters": ["heater_bed", "extruder"]
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 2);
        assert_eq!(p.status.heaters.items[0].name, "heater_bed");
        assert_eq!(p.status.heaters.items[1].name, "extruder");
    }
    #[test]
    fn test_updating_heater_data_does_not_add_new_heaters() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "heaters": {
                "available_heaters": ["heater_bed", "extruder"]
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 2);
        assert_eq!(p.status.heaters.items[0].name, "heater_bed");
        assert_eq!(p.status.heaters.items[1].name, "extruder");
        data = serde_json::json!({
            "heater_bed": {
                "temperature": 50.0,
                "power": 0.5,
                "target": 60.0
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 2);
        assert_eq!(p.status.heaters.items[0].name, "heater_bed");
        assert_eq!(p.status.heaters.items[1].name, "extruder");
    }

    #[test]
    fn test_add_temperature_fan_adds_temperature_fan() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "heaters": {
                "available_sensors": ["temperature_fan test_fan"]
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 1);
        assert_eq!(p.status.heaters.items[0].name, "temperature_fan test_fan");
    }

    #[test]
    fn test_update_temperature_fan_data_does_not_add_new_heaters() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "heaters": {
                "available_sensors": ["temperature_fan test_fan"]
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 1);
        assert_eq!(p.status.heaters.items[0].name, "temperature_fan test_fan");
        data = serde_json::json!({
            "temperature_fan test_fan": {
                "temperature": 50.0,
                "power": 0.5,
                "target": 60.0
            }
        });
        p.update(data);
        assert_eq!(p.status.heaters.items.len(), 1);
        assert_eq!(p.status.heaters.items[0].name, "temperature_fan test_fan");
        assert_eq!(p.status.heaters.items[0].heater_type, HeaterType::TemperatureFan);
        assert_eq!(p.status.heaters.items[0].temperature, 50.0);
        assert_eq!(p.status.heaters.items[0].power, 0.5);
    }

    #[test]
    fn test_updating_filament_sensor_sets_filament_switch() {
        let mut p = Printer::new();
        let mut data = serde_json::json!({
            "filament_switch_sensor test_sensor": {
                "filament_detected": true
            }
        });
        p.update(data);
        assert_eq!(p.status.filament_switch, true);
    }
}