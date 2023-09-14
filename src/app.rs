use std::collections::HashMap;
use std::{error, fs};
use std::fs::File;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::time::Duration;
use curl::easy::Easy;
use tui::widgets::ScrollbarState;
use websocket::sync::Client;
use websocket::ws::dataframe::DataFrame;
use websocket::{ClientBuilder, url::Url};
use websocket::{OwnedMessage, WebSocketError, CloseData};
use flume::{Sender, Receiver};
use std::thread::{self, JoinHandle};
use rand::Rng;
use serde_json::{Value, json};
use chrono::prelude::*;
use std::io::Write;

use crate::printer::{Printer, Heater, PrintStats, FileMetadata};
use crate::ui::stateful_list::StatefulList;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Main,
    Help,
    Toolhead,
    ToolheadHelp,
    Extruder,
    ExtruderHelp,
    Console,
    ConsoleHelp,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    pub result: Value,
    id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct JsonRpcServerRequest {
    jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct JsonRpcClientRequest {
    jsonrpc: String,
    pub method: String,
    pub params: Value,
    id: Option<String>,
}

impl JsonRpcClientRequest {
    fn new() -> JsonRpcClientRequest {
        let mut rng = rand::thread_rng();
        let id = rng.gen::<u32>();
        JsonRpcClientRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(""),
            params: json!({}),
            id: Some(format!("{:x}", id)),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash)]
pub struct RpcRequest {
    pub method: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabsState {
    pub titles: Vec<String>,
    pub index: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryItem {
    pub filename: String,
    pub status: String,
    pub end_time : f64,
    pub filament_used: f64,
    pub estimated_time: f64,
    pub total_duration: f64
}
#[derive(Debug, Clone, PartialEq)]
pub enum MainTabWidget {
    History,
    Temperatures,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct InputState {
    pub mode: InputMode,
    pub value: String,
    pub cursor_position: u16,
}

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.

pub struct App {
    /// Is the application running?
    pub running: bool,
    pub starting: bool,
    pub data: String,
    pub printer: Printer,
    pub rx: Option<Receiver<OwnedMessage>>,
    pub tx: Option<Sender<OwnedMessage>>,
    pub sent_messages: HashMap<String, RpcRequest>,
    pub current_tab: Tab,
    pub history: StatefulList<HistoryItem>,
    pub ws_connected: bool,
    pub client: Option<Client<TcpStream>>,
    pub client_rx: Option<Receiver<Client<TcpStream>>>,
    pub selected_widget: MainTabWidget,
    pub console_scroll: u16,
    pub console_scroll_state: ScrollbarState,
    pub console_input: InputState,
    pub temperature_input: InputState,
    pub selected_heater: Option<Heater>,
    pub server_url: String,
}


impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            starting: true,
            data: String::from(""),
            printer: Printer::new(),
            tx: None,
            rx: None,
            sent_messages: HashMap::new(),
            current_tab: Tab::Main,
            history: StatefulList::with_items(vec![]),
            ws_connected: false,
            client: None,
            client_rx: None,
            selected_widget: MainTabWidget::History,
            console_scroll: 0,
            console_scroll_state: ScrollbarState::default(),
            console_input: InputState { mode: InputMode::Normal, value: "".to_string(), cursor_position: 0 },
            temperature_input: InputState { mode: InputMode::Editing, value: "".to_string(), cursor_position: 1 },
            selected_heater: None,
            server_url: "".to_string(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(server_url: String) -> Self {
        let mut app = Self::default();
        app.server_url = server_url;
        app
    }

    fn generate_client(&mut self, tx: Sender<Client<TcpStream>>) -> JoinHandle<()> {
        let u = self.server_url.clone();
        
        let get_client = thread::spawn(move || {
            log::debug!("Generating client");
            loop {
                let server = format!("ws://{}/websocket", u);
                let url = Url::parse(server.as_str()).unwrap();
                let mut builder = ClientBuilder::from_url(&url);
                match builder.connect_insecure() {
                    Ok(c) => {
                        // Send message to main thread
                        let _ = tx.send(c);
                        break;
                    },
                    Err(e) => {
                        log::error!("Client connect fail : {:?}", e);
                        continue;
                    },
                };
            }
        });
        get_client
    }

    pub fn start(&mut self, client: Client<TcpStream>) {
        let stream = client.stream_ref();
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
        let (mut receiver, mut sender) = client.split().unwrap();

        
        let (send_tx, send_rx) = flume::unbounded();
        let (rcv_tx, rcv_rx) =flume::unbounded();
        
    
        let tx_1 = send_tx.clone();
        self.tx = Some(send_tx);
        self.rx = Some(rcv_rx);
        self.ws_connected = true;

        let _send_loop = thread::spawn(move || {
            loop {
                // Send loop
                match send_rx.recv() {
                    Ok(m) => {
                        let mut d2: Vec<u8> = vec![0, 1];
                        d2.append(&mut "reset".to_string().into_bytes());
                        log::debug!("sending message {:?}", m);
                        if m.is_data() {
                            let _ = sender.send_message(&m);
                        } else if m.is_close() {
                            // Tell the server to close the connection
                            let _ = sender.send_message(&m);
                            let pl = m.take_payload();
                            if pl == d2 {
                                log::debug!("exiting sending thread {:?}", pl);
                                break;
                            }
                        }
                    },
                    Err(_e) => {}
                };
            }
        });


        let _receive_loop = thread::spawn(move || {
            // Receive loop
            let close_message = OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string())));
            for message in receiver.incoming_messages() {
                let message = match message {
                    Ok(m) => m,
                    Err(e) => {
                        match e {
                            WebSocketError::NoDataAvailable => {
                                log::error!("Websocket error {:?}", e);
                                // Kill sending thread
                                let _ = tx_1.send(close_message.clone());
                                // Tell main thread that the connection has closed
                                let _ = rcv_tx.send(close_message);
                                // exit loop and terminate thread
                                log::debug!("exiting receiving thread");
                                break;
                            },
                            WebSocketError::IoError(err) => {
                                log::error!("Websocket error {:?}", err);
                                if err.kind() == ErrorKind::ConnectionReset {
                                    // Kill sending thread
                                    let _ = tx_1.send(close_message.clone());
                                    // Tell main thread that the connection has closed
                                    let _ = rcv_tx.send(close_message);
                                    // exit loop and terminate thread
                                    log::debug!("exiting receiving thread");
                                    break;
                                } else if err.kind() == ErrorKind::WouldBlock {
                                    // Kill sending thread
                                    let _ = tx_1.send(close_message.clone());
                                    // Tell main thread that the connection has closed
                                    let _ = rcv_tx.send(close_message);
                                    // exit loop and terminate thread
                                    log::debug!("exiting receiving thread");
                                    break;
                                }
                            }
                            _ => {
                                log::error!("Websocket error {:?}", e);
                            }
                        };
                        
                        OwnedMessage::Close(None)
                    }
                };
                match message {
                    OwnedMessage::Text(_) => {
                        let _ = rcv_tx.send(message);
                    },
                    OwnedMessage::Ping(_) => {
                        let _ = tx_1.send(OwnedMessage::Pong(vec![]));
                    },
                    _ => {},
                }
            }
        });
        
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        // read incoming websockets messages
        if let Some(rx) = &self.rx {
            let message = match rx.try_recv() {
                Ok(m) => m,
                Err(_e) => {
                    log::debug!("Receive error {:?}", _e);
                    return;
                }
            };


            if message.is_close() {
                // todo make sure 
                // Closing message, means we lost connection and have to restart
                
                let d2 = OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string())));

                if message.clone() == d2{
                    self.ws_connected = false;
                    self.starting = true;
                    self.printer.connected = false;
                    // self.printer.status.state = "error".to_string();
                    log::debug!("Doing init because of close message reset");
                    self.init();
                    return;
                } 
            } else if message.is_data() {
                let v = message.take_payload();
                let s: String = match String::from_utf8(v) {
                    Ok(m) => m,
                    Err(_e) => {
                        log::debug!("decode error {:?}", _e);
                        return;
                    }
                };
    
                // try to parse as a response to one of our requests
                let response: Option<JsonRpcResponse> = match serde_json::from_str(s.as_str()) {
                    Ok(m) => Some(m),
                    Err(_e) => {
                        log::debug!("not a response error {:?}", _e);
                        None
                    }
                };
    
                if response.is_some() {
                    // This is a response to one of our requests, handle it
                    self.handle_response(response.unwrap());
                } else {
                    let request: Option<JsonRpcServerRequest> = match serde_json::from_str(s.as_str()) {
                        Ok(m) => Some(m),
                        Err(_e) => {
                            log::debug!("not a server request error {:?}", _e);
                            None
                        }
                    };
                    if request.is_some() {
                        self.handle_request(request.unwrap());
                    }
                }
            }
        } else {
            if self.starting == true {
                self.try_init();
                return;
            }
        }
        
        
        //println!("ws message: {:?}", message);
    }

    pub fn init(&mut self) {
        log::debug!("App init start");
        self.printer.connected = false;
        // self.printer.status.state = "error".to_string();

        self.rx = None;
        self.tx = None;

        let (client_tx, client_rx) = flume::unbounded();
        self.client_rx = Some(client_rx);
        // Connect in another thread
        let _ = self.generate_client(client_tx);
        // wait for client to be ready
        self.try_init();
    }

    fn try_init(&mut self) {
        match self.client_rx.as_ref().expect("No client").try_recv() {
            Ok(c) => {
                log::debug!("Client connected");
                self.start(c);

                self.send_start_messages();
                self.starting = false;
            },
            Err(e) => {
                log::debug!("Client connection error {:?}", e);
            },
        };
    }

    fn send_start_messages(&mut self) {
        self.send_message(String::from("server.connection.identify"), json!({
            "client_name": "Krui",
            "version":	"0.0.1",
            "type":	"desktop",
            "url":	"https://github.com/jfoucher/krui"
        }));
        self.send_message(String::from("server.info"), serde_json::Value::Object(serde_json::Map::new()));
        self.send_message(String::from("printer.objects.list"), serde_json::Value::Object(serde_json::Map::new()));
        self.send_message(String::from("server.history.list"), json!({
            "limit": 100,
            "start": 0,
            "since": 0,
            "order": "desc"
        }));
    }

    pub fn handle_response(&mut self, response: JsonRpcResponse) {
        // find the original method id from our hashmap
        
        let method = match self.sent_messages.get(&response.id) {
            Some(m) => {
                m.method.clone()
            }
            None => "".to_string()
        };
        
        if method.len() > 0 {
            // Remove from hashmap
            self.sent_messages.remove(&response.id);

            match method.as_str() {
                "server.info"=> {
                    log::debug!("server.info {:?}", response.result);
                    self.printer.connected = false;
                    if let Some(klc) = response.result.get("klippy_connected") {
                        if klc.as_bool().unwrap() {
                            if let Some(kls) = response.result.get("klippy_state") {
                                if kls.as_str().unwrap() == "ready" {
                                    self.printer.connected = true;
                                }
                            }
                        }
                    }
                }
                "server.files.metadata" => {
                    log::info!("server.files.metadata {}", serde_json::to_string(&response.result).unwrap());
                    let mut current_print = PrintStats::new();
                
                    if let Some(cp) = &self.printer.current_print {
                        current_print = cp.clone();
                    }
                    current_print.file = FileMetadata {
                        size: response.result["size"].as_u64().unwrap(),
                        slicer: response.result["slicer"].as_str().unwrap().to_string(),
                        layer_height: response.result["layer_height"].as_f64().unwrap(),
                        first_layer_height: response.result["first_layer_height"].as_f64().unwrap(),
                        object_height: response.result["object_height"].as_f64().unwrap(),
                        filament_total: response.result["filament_total"].as_f64().unwrap(),
                        estimated_time: response.result["estimated_time"].as_f64().unwrap(),
                    };
                    let mut easy = Easy::new();
                    let res = response.result;
                    let u = self.server_url.clone();
                    if let Some(thumbnails) = res.get("thumbnails") {
                        let ths = thumbnails.as_array().unwrap();
                        // Get last thumbnail as it seems to be the biggest one
                        if let Some(thumbnail) = thumbnails.get(ths.len()-1) {
                            log::info!("Thumbnail {:?}", thumbnail);
                            if let Some(path) = thumbnail.get("relative_path") {
                                let np = path.as_str().unwrap().clone();
                                log::info!("Downloading thumbnail {:?}", format!("http://{}/server/files/gcodes/{}", u, np.clone()).as_str());
                                easy.url(format!("http://{}/server/files/gcodes/{}", u, np.clone()).as_str()).unwrap();
                                let filename = np.clone().to_string();
                                let _ = fs::create_dir_all("cache/.thumbs/");
                                let filepath = format!("cache/{}", filename);
                                current_print.image = filepath.clone();
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

                                        handle.write_all(data).unwrap();
                                        
                                        Ok(data.len())
                                    }).unwrap();
                                    transfer.perform().unwrap();
                                }
                            }
                        }
                    }
                    
                    self.printer.current_print = Some(current_print);
                }
                "printer.objects.list" => {
                    let json_objects: Vec<String> = serde_json::from_value(response.result["objects"].clone()).unwrap();
                    let mut params = serde_json::Map::new();
                    let mut objects = serde_json::Map::new();
                    for ob in json_objects.iter() {
                        objects.insert(ob.clone(), serde_json::Value::Null);
                    }
                    params.insert("objects".to_string(), objects.into());

                    self.send_message("printer.objects.query".to_string(), serde_json::Value::Object(params.clone()));
                    self.send_message("printer.objects.subscribe".to_string(), serde_json::Value::Object(params));
                }
                "printer.emergency_stop" => {
                    // self.printer.connected = false;
                    // self.printer.stats.state = "shutdown".to_string();
                }
                "printer.objects.query" => {
                    let data = response.result.get("status").unwrap().clone();
                    self.printer.update(data.clone());

                    if let Some(print_stats) = data.get("print_stats") {
                        
                        if let Some(state) = print_stats.get("state") {
                            
                            if state.as_str().unwrap() == "printing" {
                                if let Some(filename) = print_stats.get("filename") {
                                    log::info!("filename {:?}", filename.as_str().unwrap());
                                    // If status is printing, get metadata
                                    self.send_message("server.files.metadata".to_string(), json!({"filename": filename.as_str().unwrap()}));
               
                                }
                            }
                        }
                    }
                },
                "server.history.list" => {
                    if let Some(jobs) = response.result.get("jobs") {

                        if jobs.is_array() {
                            for job in jobs.as_array().unwrap() {
                                // println!("{:?}", job);
                                self.add_job(job);
                            }
                            self.history.state.select(Some(0));
                        }
                    }
                },

                _ => {}
            }
        }
    }

    pub fn add_job(&mut self, job: &Value) {
        if let Some(filename) = job.get("filename") {
            let filename = filename.as_str().unwrap().to_string();
            if let Some(_) = self.history.items.iter().find(|i| {i.filename == filename}) {
                return;
            }
            let mut h = HistoryItem {
                filename: filename.clone(),
                status: "".to_string(),
                end_time: 0.0,
                estimated_time: 0.0,
                total_duration: 0.0,
                filament_used: 0.0,
            };
            if let Some(status) = job.get("status") {
                if let Some(s) = status.as_str() {
                    h.status = s.to_string();
                }
            }
            if let Some(end_time) = job.get("end_time") {
                if let Some(s) = end_time.as_f64() {
                    h.end_time = s;
                }
            }
            if let Some(total_duration) = job.get("total_duration") {
                if let Some(s) = total_duration.as_f64() {
                    h.total_duration = s;
                }
            }
            if let Some(filament_used) = job.get("filament_used") {
                if let Some(s) = filament_used.as_f64() {
                    h.filament_used = s;
                }
            }

            if let Some(metadata) = job.get("metadata") {
                if let Some(estimated_time) = metadata.get("estimated_time") {
                    if let Some(s) = estimated_time.as_f64() {
                        h.estimated_time = s;
                    }
                }
            }
            self.history.add(h);
        }
    }

    pub fn handle_request(&mut self, request: JsonRpcServerRequest) {
        let method = request.method.as_str();

        match method {
            "notify_klippy_shutdown" => {
                log::debug!("notify_klippy_shutdown");
                self.printer.connected = false;
            },
            "notify_history_changed" => {
                if let Some(params) = request.params {
                    if let Some(action) = params.get("action") {
                        if action.as_str().unwrap() == "added" {
                            if let Some(job) = params.get("job") {
                                self.add_job(job);
                                self.history.state.select(Some(0));
                            }
                        }
                    }
                }
            },
            "notify_klippy_ready" => {
                log::debug!("notify_klippy_ready");
                self.printer.connected = true;
                self.send_start_messages();
            },
            "notify_proc_stat_update" => {},
            "notify_status_update" => {
                if let Some(params) = request.params {
                    self.printer.update(params.get(0).unwrap().clone());
                }
            },
            "notify_gcode_response" => {
                // self.printer.status.gcodes.push(value);
                log::debug!("notify_gcode_response params {:?}", request.params);
                // self.quit();
                if let Some(p) = request.params {
                    if let Some(params) = p.as_array() {
                        for param in params {
                            if let Some(l) = param.as_str() {
                                self.printer.status.gcodes.push(
                                    crate::printer::GCodeLine {
                                        timestamp: Local::now(),
                                        content: l.to_string(),
                                    }
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                log::warn!("unknown request method: {} with params: {:?}", method, request.params);
            }
        }
    }

    pub fn emergency_stop(&mut self) {
        self.send_message("printer.emergency_stop".to_string(), serde_json::Value::Object(serde_json::Map::new()));
        self.printer.status.state = "error".to_string();
        self.printer.current_print = None;
        self.printer.connected = false;
        if let Some(tx) = &self.tx {
            let _ = tx.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
        }
    }

    pub fn send_message(&mut self, method: String, params: Value ) {
        let mut message = JsonRpcClientRequest::new();
        message.method = method.clone();
        message.params = params;
        let m = match serde_json::to_string(&message) {
            Ok(m) => m,
            Err(_e) => {
                return;
            }
        };


        if message.id.is_some() {
            let q: RpcRequest = RpcRequest { method };
            self.sent_messages.insert(message.id.unwrap(), q);
        }
        

        if let Some(tx) = &self.tx {
            let _ = tx.send(OwnedMessage::Text(m));
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn get_data(&mut self) -> &str {
        return self.data.as_str();
    }
}
