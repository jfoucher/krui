
use std::collections::HashMap;

use log4rs::append::rolling_file::LogFile;


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Toolhead {
    pub position: Position,
    pub homed: Homed,
    pub fan: Fan,
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
pub struct Fan {
    #[serde(default = "default_float")]
    pub speed: f64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Heater {
    #[serde(default = "default_float")]
    pub temperature: f64,
    #[serde(default = "default_float")]
    pub target: f64,
    #[serde(default = "default_float")]
    pub power: f64,
}

pub fn default_float() -> f64 {
    return 0.0;
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TemperatureFan {
    #[serde(default = "default_float")]
    pub temperature: f64,
    #[serde(default = "default_float")]
    pub target: f64,
    #[serde(default = "default_float")]
    pub speed: f64,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PrinterStatus {
    pub heaters: HashMap<String, Heater>,
    pub temperature_fans: HashMap<String, TemperatureFan>,
    pub state: String,
    pub state_message: String,
    pub stepper_enable: bool,
    pub filament_switch: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Printer {
    pub connected: bool,
    pub status: PrinterStatus,
    pub toolhead: Toolhead,
    pub sysload: f64,
}

impl Printer {
    pub fn new() -> Printer {
        Printer {
            connected: false,
            status: PrinterStatus {
                heaters: HashMap::new(),
                temperature_fans: HashMap::new(),
                state: String::from("unknown"),
                state_message: "".to_string(),
                stepper_enable: false,
                filament_switch: false,
            },
            toolhead: Toolhead {
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }, 
                homed: Homed { x: false, y: false, z: false, qgl: false },
                fan: Fan { speed: 0.0 }
            },
            sysload: 0.0,
        }
    }

    pub fn update(&mut self, data: serde_json::Value) {
        if let Some(heaters) = data.get("heaters") {
            if let Some(available_heaters) = heaters.get("available_heaters") {
                // add heater to HashMap of heaters, with blank heaters
                for h in available_heaters.as_array().unwrap() {
                    let heater_name: String = serde_json::from_value(h.clone()).unwrap();
                    if let None = self.status.heaters.get(&heater_name) {
                        self.status.heaters.insert(serde_json::from_value(h.clone()).unwrap(), Heater { temperature: 0.0, target: 0.0, power: 0.0 });
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
                    if let None = self.status.temperature_fans.get(&heater_name) {
                        self.status.temperature_fans.insert(serde_json::from_value(h.clone()).unwrap(), TemperatureFan { temperature: 0.0, target: 0.0, speed: 0.0 });
                    }
                }
            }
        }
        // For each heater, check if we have data to set their values
        let mut new_heaters = self.status.heaters.clone();
        for (k, heater) in &self.status.heaters {
            // try to get current heater from data
            if let Some(heater_data) = data.get(k) {
                let mut nh = heater.clone();
                if let Some(temp) = heater_data.get("temperature") {
                    nh.temperature = serde_json::from_value(temp.clone()).unwrap();
                }
                if let Some(pow) = heater_data.get("power") {
                    nh.power = serde_json::from_value(pow.clone()).unwrap();
                }
                if let Some(t) = heater_data.get("target") {
                    nh.target = serde_json::from_value(t.clone()).unwrap();
                }
                new_heaters.insert(k.to_string(), nh);
            }
        }
        self.status.heaters = new_heaters;

        // For each temp fan, check if we have data to set their values
        let mut new_heaters = self.status.temperature_fans.clone();
        for (k, heater) in &self.status.temperature_fans {
            // try to get current fan from data
            if let Some(heater_data) = data.get(k) {
                let mut nh = heater.clone();
                if let Some(temp) = heater_data.get("temperature") {
                    nh.temperature = serde_json::from_value(temp.clone()).unwrap();
                }
                if let Some(pow) = heater_data.get("speed") {
                    nh.speed = serde_json::from_value(pow.clone()).unwrap();
                }
                if let Some(t) = heater_data.get("target") {
                    nh.target = serde_json::from_value(t.clone()).unwrap();
                }
                new_heaters.insert(k.to_string(), nh);
            }
        }
        self.status.temperature_fans = new_heaters;

        let mut status = self.status.clone();
        if let Some(print_stats) = data.get("webhooks") {
            log::info!("{:?}", print_stats);
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
                if let Some(axes) = homes.as_str() {
                    let ax = String::from(axes);
                    if ax.contains("x") {
                        homed.x = true;
                    }
                    if ax.contains("z") {
                        homed.z = true;
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

