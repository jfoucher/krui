use std::collections::HashMap;
use std::error;
use std::fs::File;
use std::io::Write;
use websocket::ws::dataframe::DataFrame;
use websocket::{ClientBuilder, url::Url};
use websocket::{Message, OwnedMessage};
use flume::{Sender, Receiver};
use std::thread;
use rand::Rng;
use serde_json::{Value, json, Map,};
use std::time::{SystemTime, Duration};
mod printer;
use printer::{Printer};


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
    pub params: Value,
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


/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
const CONNECTION: &'static str = "ws://voron.ink/websocket";
/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    pub data: String,
    pub printer: Printer,
    pub rx: Receiver<OwnedMessage>,
    pub tx: Sender<OwnedMessage>,
    pub sent_messages: HashMap<String, RpcRequest>,
}

impl Default for App {
    fn default() -> Self {
        let url = Url::parse(CONNECTION).unwrap();
        let mut builder = ClientBuilder::from_url(&url);
        let client = builder.connect_insecure().unwrap();
        
        let (mut receiver, mut sender) = client.split().unwrap();


        let (send_tx, send_rx) = flume::unbounded();
        let (rcv_tx, rcv_rx) =flume::unbounded();

        let tx_1 = send_tx.clone();


        // New thread to update data;

        let update_loop = thread::spawn(move || {
            
        });


        let _send_loop = thread::spawn(move || {
            loop {
                // Send loop
                match send_rx.try_recv() {
                    Ok(m) => {
                        let _ = sender.send_message(&m);
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
                    Err(_e) => {
                        OwnedMessage::Binary(vec![])
                    }
                };
                match message {
                    OwnedMessage::Text(_) => {
                        let _ = rcv_tx.send(message);
                    },
                    OwnedMessage::Ping(_) => {
                        let _ = tx_1.send(OwnedMessage::Pong(vec![]));
                    },
                    _ => {
                        println!("not a text message {:?}", message);
                    },
                }
            }
        });

        Self {
            running: true,
            data: String::from(""),
            printer: Printer::new(),
            tx: send_tx,
            rx: rcv_rx,
            sent_messages: HashMap::new(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        // read incoming websockets messages
        let message = match self.rx.try_recv() {
            Ok(m) => m,
            Err(_e) => {
                return;
            }
        };

        if message.is_data() {
            let v = message.take_payload();
            let s: String = match String::from_utf8(v) {
                Ok(m) => m,
                Err(_e) => {
                    return;
                }
            };

            // try to parse as a response to one of our requests
            let response: Option<JsonRpcResponse> = match serde_json::from_str(s.as_str()) {
                Ok(m) => Some(m),
                Err(_e) => {
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
                        None
                    }
                };
                if request.is_some() {
                    self.handle_request(request.unwrap());
                }
            }
            
            
        }
        
        //println!("ws message: {:?}", message);
    }

    pub fn init(&mut self) {
        self.send_message(String::from("server.info"), serde_json::Value::Object(serde_json::Map::new()));
        self.send_message(String::from("printer.objects.list"), serde_json::Value::Object(serde_json::Map::new()));
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
                    //self.data = format!("{}\n{:?}",self.data, response.result)
                    self.printer.connected = serde_json::from_value(response.result["klippy_connected"].clone()).unwrap();
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
                },
                "printer.objects.query" => {
                    self.printer.update(response.result.get("status").unwrap().clone());
                }
                _ => {
                    //println!("not implemented")
                }
            }
        }
    }

    pub fn handle_request(&mut self, request: JsonRpcServerRequest) {
        let method = request.method.as_str();

        match method {
            "notify_klippy_shutdown" => self.printer.connected = false,
            "notify_klippy_ready" => self.printer.connected = true,
            "notify_status_update" => {
                //println!("{:?}", request.params);
                self.printer.update(request.params.get(0).unwrap().clone());
                // TODO update PrinterStatus struct
                // let path = "data.json";
                // let mut output = File::create(path).unwrap();
                // let line = serde_json::to_string_pretty(&self.printer.status).unwrap();
                // write!(output, "{}", line);
               
            }
            _ => {
                //println!("Method {} not implement", method)
            }
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
        

        //println!("sending json message {:?}", m);
        //self.data = m.clone();
        let _ = self.tx.send(OwnedMessage::Text(m));


        
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        
    }

    pub fn decrement_counter(&mut self) {
        
    }
    pub fn get_data(&mut self) -> &str {
        return self.data.as_str();
    }
}
