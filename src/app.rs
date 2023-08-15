use std::collections::HashMap;
use std::error;
use std::fs::File;
use std::io::Write;
use websocket::ws::dataframe::DataFrame;
use websocket::{ClientBuilder, url::Url};
use websocket::{Message, OwnedMessage};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use rand::Rng;
use serde_json::{Value, json, Map,};

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Heater {
    temperature: f64,
    target: f64,
    power: f64,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TemperatureFan {
    temperature: f64,
    target: f64,
    speed: f64,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PrinterStatus {
    heaters: Vec<Heater>,
    temperatures_fans: Vec<TemperatureFan>
}


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct Printer {
    pub connected: bool,
    pub status: PrinterStatus,
}

impl Printer {
    fn new() -> Printer {
        Printer {
            connected: false,
            status: PrinterStatus { heaters: vec![], temperatures_fans: vec![] }
        }
    }

    pub fn merge(&mut self, o: serde_json::Value, n: serde_json::Value) -> serde_json::Value {
        let new = n.as_object().unwrap();
        let old = o.as_object();
        let mut ret = match old {
            Some(old_object) => {
                old_object.clone()
            },
            None => {
                Map::new()
            }
        };
        for (k, v) in new {
            if v.is_object() {
                if ret.contains_key(k) {
                    let re = ret.get(k).unwrap().clone();
                    ret.insert(k.to_string(), serde_json::Value::Object(self.merge(re, v.clone()).as_object().unwrap().clone()));
                } else {
                    ret.insert(k.to_string(), v.clone());
                }
            } else {
                ret.insert(k.to_string(), v.clone());
            }
        }
        serde_json::Value::Object(ret)
    }

    pub fn update(&mut self, data: serde_json::Value) {
        let st = self.status.clone();
        self.status = self.merge(st, data);
    }
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
    pub objects: Vec<String>,
    pub printer: Printer,
    pub rx: Receiver<OwnedMessage>,
    pub tx: Sender<OwnedMessage>,
    pub sent_messages: HashMap<String, RpcRequest>
}

impl Default for App {
    fn default() -> Self {
        let url = Url::parse(CONNECTION).unwrap();
        let mut builder = ClientBuilder::from_url(&url);
        let client = builder.connect_insecure().unwrap();
        
        let (mut receiver, mut sender) = client.split().unwrap();

        let (send_tx, send_rx) = channel();
        let (rcv_tx, rcv_rx) = channel();

        let tx_1 = send_tx.clone();

        let send_loop = thread::spawn(move || {
            loop {
                // Send loop
                let message = match send_rx.recv() {
                    Ok(m) => m,
                    Err(e) => {
                        // println!("Send Loop: {:?}", e);
                        return;
                    }
                };
                match message {
                    OwnedMessage::Close(_) => {
                        let _ = sender.send_message(&message);
                        // If it's a close message, just send it and then return.
                        return;
                    }
                    _ => (),
                }
                // Send the message
                match sender.send_message(&message) {
                    Ok(()) => (),
                    Err(e) => {
                        // println!("Send Loop: {:?}", e);
                        let _ = sender.send_message(&Message::close());
                        return;
                    }
                }
            }
        });

        let receive_loop = thread::spawn(move || {
            // Receive loop
            for message in receiver.incoming_messages() {
                let message = match message {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx_1.send(OwnedMessage::Close(None));
                        return;
                    }
                };
                match message {
                    OwnedMessage::Close(_) => {
                        // Got a close message, so send a close message and return
                        let _ = tx_1.send(OwnedMessage::Close(None));
                        // Let app know that the connection was closed
                        let _ = rcv_tx.send(OwnedMessage::Close(None));
                        return;
                    }
                    OwnedMessage::Ping(data) => {
                        match tx_1.send(OwnedMessage::Pong(data)) {
                            // Send a pong in response
                            Ok(()) => (),
                            Err(e) => {
                                // println!("Receive Loop ping err: {:?}", e);
                                return;
                            }
                        }
                    }
                    // Say what we received
                    _ => {
                        let _ = rcv_tx.send(message);
                    },
                }
            }
        });

        Self {
            running: true,
            data: String::from(""),
            objects: vec![],

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
                    self.objects = serde_json::from_value(response.result["objects"].clone()).unwrap();
                    let mut params = serde_json::Map::new();
                    let mut objects = serde_json::Map::new();
                    for ob in self.objects.iter() {
                        objects.insert(ob.clone(), serde_json::Value::Null);
                    }
                    params.insert("objects".to_string(), objects.into());

                    self.send_message("printer.objects.query".to_string(), serde_json::Value::Object(params.clone()));
                    self.send_message("printer.objects.subscribe".to_string(), serde_json::Value::Object(params));
                },
                "printer.objects.query" => {
                    self.printer.status = response.result.get("status").unwrap().clone();
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
                let ex = self.printer.status.get("extruder");
                // let path = "data.json";
                // let mut output = File::create(path).unwrap();
                // let line = serde_json::to_string_pretty(&self.printer.status).unwrap();
                // write!(output, "{}", line);

                match ex {
                    Some(e) => {
                        self.data = format!("{}", e.get("temperature").unwrap().to_string())
                    },
                    None => {}
                }
               
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
