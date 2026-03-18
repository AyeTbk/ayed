mod event;
use std::{collections::HashMap, str::FromStr};

pub use event::Event;

pub mod lsp;

pub mod types;

mod notification;
pub use notification::Notification;

mod request;
pub use request::Request;

mod response;
pub use response::Response;

mod transport;
use serde_json::Value;
use transport::SubprocessTransport;

use crate::{
    notification::convert_notification_to_json,
    request::{RequestType, convert_request_to_json},
};

const INITIALIZE_REQUEST_ID: i32 = 1;

pub struct LspClient {
    transport: SubprocessTransport,
    state: State,
    pending_requests: Vec<Request>,
    pending_notifications: Vec<Notification>,
    request_counter: i32,
    request_type_per_id: HashMap<i32, RequestType>,
}

impl LspClient {
    pub fn new() -> Self {
        let transport = SubprocessTransport::new("rust-analyzer");
        Self {
            transport,
            state: State::Offline,
            pending_requests: Vec::new(),
            pending_notifications: Vec::new(),
            request_counter: INITIALIZE_REQUEST_ID,
            request_type_per_id: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) {
        assert!(self.state == State::Offline);

        self.send_request(Request::Initialize);

        self.state = State::Initializing;
    }

    pub fn shutdown(self) {
        self.transport.shutdown();
    }

    pub fn queue_request(&mut self, req: Request) {
        self.pending_requests.push(req);
    }

    pub fn queue_notification(&mut self, notif: Notification) {
        self.pending_notifications.push(notif);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        todo!()
    }

    pub fn is_online(&self) -> bool {
        self.state == State::Online
    }

    pub fn is_just_initialized(&self) -> bool {
        self.state == State::Initialized
    }

    pub fn tick(&mut self) {
        match self.state {
            State::Initializing => self.tick_initializing(),
            State::Online | State::Initialized => {
                self.state = State::Online;
                self.tick_online();
            }
            _ => todo!(),
        }
    }

    fn tick_initializing(&mut self) {
        let (resps, notifs) = self.recv_server_messages();
        for response in resps {
            match response {
                lsp::Response { id, .. } if id == INITIALIZE_REQUEST_ID => {
                    self.send_notification(Notification::Initialized);
                    self.state = State::Initialized;
                }
                _ => {
                    panic!(
                        "lsp initializing: server sent a response to something else than the initialize request: {response:?}"
                    );
                }
            }
        }
        for notif in notifs {
            panic!("lsp initializing: server sent an unexpected notification: {notif:?}");
        }
    }

    fn tick_online(&mut self) {
        for notif in std::mem::take(&mut self.pending_notifications) {
            self.send_notification(notif);
        }
        for req in std::mem::take(&mut self.pending_requests) {
            self.send_request(req);
        }
    }

    fn send_json(&mut self, content: &Value) {
        let json_string = serde_json::to_string(content).unwrap();
        self.send_message(json_string);
    }

    fn send_request(&mut self, req: Request) {
        dbg!(&req);
        let request_type = req.typ();
        let id = self.request_counter;
        self.request_counter += 1;
        let lsp_request = convert_request_to_json(req, id);
        self.request_type_per_id.insert(id, request_type);
        let req_string = serde_json::to_string(&lsp_request).unwrap();
        self.send_message(req_string);
    }

    fn send_notification(&mut self, notif: Notification) {
        dbg!(&notif);
        let notif_json = convert_notification_to_json(notif);
        self.send_json(&notif_json);
    }

    fn send_message(&mut self, content: String) {
        let full_message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
        self.transport.send(full_message.into_bytes())
    }

    // FIXME make not pub (currently pub for the example)
    pub fn recv_server_messages(&mut self) -> (Vec<lsp::Response>, Vec<Value>) {
        if let Ok(err) = self.transport.recv_server_err.try_recv() {
            eprintln!("lsp server err: {}", String::from_utf8_lossy(&err))
        }

        let mut responses = Vec::new();
        let mut notifications = Vec::new();

        for b in self.transport.recv() {
            let content = String::from_utf8(b).unwrap();
            let message = serde_json::from_str::<lsp::Message>(&content).unwrap();
            if message.id.is_some() {
                let response = serde_json::from_str::<lsp::Response>(&content).unwrap();
                responses.push(response)
            } else {
                let notification = serde_json::Value::from_str(&content).unwrap();
                notifications.push(notification)
            }
        }

        (responses, notifications)
    }

    pub fn receive_responses(&mut self) -> Vec<Response> {
        let (resps, notifs) = self.recv_server_messages();

        for notif in notifs {
            dbg!(notif);
        }

        let mut responses = Vec::new();
        for resp in resps {
            let Some(mut resp_result) = resp.result else {
                eprintln!("server response (id: {}) is malformed", resp.id);
                continue;
            };

            use serde_json::Value;

            let get_hover_result = |result: &mut Value| -> Option<String> {
                if let Value::String(text) = result.pointer_mut("/contents/value")?.take() {
                    Some(text)
                } else {
                    None
                }
            };

            let Some(request_type) = self.request_type_per_id.remove(&resp.id) else {
                eprintln!("lsp response without associated request. id {}", resp.id);
                continue;
            };
            match request_type {
                RequestType::Initialize => unimplemented!(),
                RequestType::SuggestCompletion => unimplemented!(),
                RequestType::Hover => {
                    if let Some(text) = get_hover_result(&mut resp_result) {
                        responses.push(Response::HoverInfo { text });
                    } else {
                        unimplemented!("{resp_result:?}");
                    }
                }
            }
        }

        responses
    }
}

#[derive(Debug, PartialEq)]
pub enum State {
    Offline,
    Initializing,
    Initialized,
    Online,
    ShuttingDown,
}
