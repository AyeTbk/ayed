mod event;
use std::collections::HashMap;

pub use event::Event;

pub mod lsp;

mod request;
pub use request::{File, Position, Request};

mod response;
pub use response::Response;

mod transport;
use transport::SubprocessTransport;

use crate::request::{
    RequestType, convert_request_to_lsp, make_lsp_notification, make_lsp_request,
};

const INITIALIZE_REQUEST_ID: i32 = 1;

pub struct LspClient {
    transport: SubprocessTransport,
    state: State,
    pending_requests: Vec<Request>,
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
            request_counter: INITIALIZE_REQUEST_ID,
            request_type_per_id: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) {
        assert!(self.state == State::Offline);

        self.send_request(&make_initialize_request());

        self.state = State::Initializing;
    }

    pub fn shutdown(self) {
        self.transport.shutdown();
    }

    pub fn queue_request(&mut self, req: Request) {
        self.pending_requests.push(req);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        todo!()
    }

    pub fn is_online(&self) -> bool {
        self.state == State::Online
    }

    pub fn tick(&mut self) {
        match self.state {
            State::Initializing => self.tick_initializing(),
            State::Online => self.tick_online(),
            _ => todo!(),
        }
    }

    fn tick_initializing(&mut self) {
        let (resps, notifs) = self.recv_server_messages();
        for response in resps {
            match response {
                lsp::Response { id, .. } if id == INITIALIZE_REQUEST_ID => {
                    self.send_notification(&make_initialized_notification());
                    self.state = State::Online;
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
        for request in std::mem::take(&mut self.pending_requests) {
            let request_type = request.typ();
            let mut lsp_request = convert_request_to_lsp(request);
            self.request_counter += 1;
            lsp_request.id = self.request_counter;
            self.request_type_per_id
                .insert(lsp_request.id, request_type);

            self.send_request(&lsp_request);
        }
    }

    fn send_request(&mut self, request: &lsp::Request) {
        let content = serde_json::to_string(request).unwrap();
        self.send_message(content)
    }

    pub fn send_notification(&mut self, notification: &lsp::Notification) {
        let content = serde_json::to_string(notification).unwrap();
        self.send_message(content)
    }

    fn send_message(&mut self, content: String) {
        let full_message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
        self.transport.send(full_message.into_bytes())
    }

    // FIXME make not pub (currently pub for the example)
    pub fn recv_server_messages(&mut self) -> (Vec<lsp::Response>, Vec<lsp::Notification>) {
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
                let notification = serde_json::from_str::<lsp::Notification>(&content).unwrap();
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
    Online,
    ShuttingDown,
}

fn make_initialize_request() -> lsp::Request {
    make_lsp_request(
        INITIALIZE_REQUEST_ID,
        "initialize",
        Some(lsp::RequestParams::Initialize(lsp::InitializeParams {
            process_id: None,
            capabilities: lsp::ClientCapabilities {
                general: None,
                workspace: None,
                text_document: None,
            },
            root_uri: None, //"file:///home/simon/workspaces/rust/ayed".into(),
            workspace_folders: vec![
                // lsp::WorkspaceFolder {
                //     name: "ayed".into(),
                //     uri: "file:///home/simon/workspaces/rust/ayed".into(),
                // }
            ],
        })),
    )
}

fn make_initialized_notification() -> lsp::Notification {
    make_lsp_notification("initialized", Some(lsp::NotificationParams::Initialized {}))
}
