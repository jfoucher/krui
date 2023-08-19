use std::collections::HashMap;
use std::error;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::time::Duration;
use websocket::sync::Client;
use websocket::ws::dataframe::DataFrame;
use websocket::{ClientBuilder, url::Url};
use websocket::{OwnedMessage, WebSocketError, CloseData};
use flume::{Sender, Receiver};
use std::thread::{self, JoinHandle};
use rand::Rng;
use serde_json::{Value, json};


use crate::printer::Printer;


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum Tab {
    Main,
    Help,
    Toolhead,
    ToolheadHelp,
    Extruder,
    ExtruderHelp,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    pub result: Value,
    id: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash)]
pub struct TabsState {
    pub titles: Vec<String>,
    pub index: usize,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct HistoryItem {
    pub filename: String,
    pub status: String,
    pub end_time : f64,
    pub filament_used: f64,
    pub estimated_time: f64,
    pub total_duration: f64
}


/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
const CONNECTION: &'static str = "ws://192.168.1.11/websocket";
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
    pub history: HashMap<String, HistoryItem>,
    pub ws_connected: bool,
    pub client: Option<Client<TcpStream>>,
    pub client_rx: Option<Receiver<Client<TcpStream>>>,
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
            history: HashMap::new(),
            ws_connected: false,
            client: None,
            client_rx: None,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    fn generate_client(&mut self, tx: Sender<Client<TcpStream>>) -> JoinHandle<()> {
        let get_client = thread::spawn(move || {
            loop {
                let url = Url::parse(CONNECTION).unwrap();
                let mut builder = ClientBuilder::from_url(&url);
                match builder.connect_insecure() {
                    Ok(c) => {
                        // Send message to main thread
                        let _ = tx.send(c);
                        break;
                    },
                    Err(_) => {
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
                        log::debug!("sending message {:?}", m);
                        if m.is_data() {
                            let _ = sender.send_message(&m);
                        } else if m.is_close() {
                            // Tell the server to close the connection
                            let _ = sender.send_message(&m);
                            log::info!("exiting sending thread");
                            break;
                        }
                    },
                    Err(_e) => {}
                };
            }
        });


        let _receive_loop = thread::spawn(move || {
            // Receive loop
            for message in receiver.incoming_messages() {
                let message = match message {
                    Ok(m) => m,
                    Err(e) => {
                        match e {
                            WebSocketError::NoDataAvailable => {
                                log::error!("Websocket error {:?}", e);
                                // Kill sending thread
                                let _ = tx_1.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                // Tell main thread that the connection has closed
                                let _ = rcv_tx.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                // exit loop and terminate thread
                                log::info!("exiting receiving thread");
                                break;
                            },
                            WebSocketError::IoError(err) => {
                                log::error!("Websocket error {:?}", err);
                                if err.kind() == ErrorKind::ConnectionReset {
                                    // Kill sending thread
                                    let _ = tx_1.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                    // Tell main thread that the connection has closed
                                    let _ = rcv_tx.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                    // exit loop and terminate thread
                                    log::info!("exiting receiving thread");
                                    break;
                                } else if err.kind() == ErrorKind::WouldBlock {
                                    // Kill sending thread
                                    let _ = tx_1.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                    // Tell main thread that the connection has closed
                                    let _ = rcv_tx.send(OwnedMessage::Close(Some(CloseData::new(1, "reset".to_string()))));
                                    // exit loop and terminate thread
                                    log::info!("exiting receiving thread");
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
                // Closing message, means we lost connection and have to restart
                self.ws_connected = false;
                self.printer.connected = false;
                self.printer.stats.state = "error".to_string();
                self.init();
                return;
            }
    
            if message.is_data() {
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
        log::info!("App init start");
        self.printer.connected = false;
        self.printer.stats.state = "error".to_string();

        self.rx = None;
        self.tx = None;

        let (client_tx, client_rx) =flume::unbounded();
        self.client_rx = Some(client_rx);
        // Connect in another thread
        let _ = self.generate_client(client_tx);
        // wait for client to be ready
        self.try_init();
        

        log::info!("App init done");
    }

    fn try_init(&mut self) {
        match self.client_rx.as_ref().expect("No client").try_recv() {
            Ok(c) => {
                self.start(c);

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
                self.starting = false;
            },
            Err(_) => {},
        };
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
                    self.printer.update(response.result.get("status").unwrap().clone());
                }
                "server.history.list" => {
                    if let Some(jobs) = response.result.get("jobs") {

                        if jobs.is_array() {
                            for job in jobs.as_array().unwrap() {
                                // println!("{:?}", job);
                                if let Some(filename) = job.get("filename") {
                                    let filename = filename.as_str().unwrap().to_string();
                                    if let Some(_) = self.history.get(&filename) {
                                        continue;
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
                                        h.status = status.as_str().unwrap().to_string();
                                    }
                                    if let Some(end_time) = job.get("end_time") {
                                        h.end_time = end_time.as_f64().unwrap();
                                    }
                                    if let Some(total_duration) = job.get("total_duration") {
                                        h.total_duration = total_duration.as_f64().unwrap();
                                    }
                                    if let Some(filament_used) = job.get("filament_used") {
                                        h.filament_used = filament_used.as_f64().unwrap();
                                    }

                                    if let Some(metadata) = job.get("metadata") {
                                        if let Some(estimated_time) = metadata.get("estimated_time") {
                                            h.estimated_time = estimated_time.as_f64().unwrap();
                                        }
                                        
                                    }
                                    self.history.insert(filename, h);
                                }
                            }
                        }
                    }
                },

                _ => {}
            }
        }
    }

    pub fn handle_request(&mut self, request: JsonRpcServerRequest) {
        let method = request.method.as_str();

        match method {
            "notify_klippy_shutdown" => {
                log::debug!("notify_klippy_shutdown");
                self.printer.connected = false;
            },
            "notify_klippy_ready" => {
                log::debug!("notify_klippy_ready");
                self.printer.connected = true;
                self.init();
            },
            "notify_proc_stat_update" => {},
            "notify_status_update" => {
                if let Some(params) = request.params {
                    self.printer.update(params.get(0).unwrap().clone());
                }
                
                // TODO update PrinterStatus struct
                // let path = "data.json";
                // let mut output = File::create(path).unwrap();
                // let line = serde_json::to_string_pretty(&self.printer.status).unwrap();
                // write!(output, "{}", line);
               
            },
            "notify_gcode_response" => {
                self.data = format!("notify_gcode_response params {:?}", request.params);
                // self.quit();
            }
            _ => {}
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
