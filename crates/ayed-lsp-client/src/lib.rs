mod event;
pub use event::Event;

pub mod lsp;

mod request;
pub use request::{File, Position, Request};

mod transport;
use transport::SubprocessTransport;

use crate::request::{convert_request_to_lsp, make_lsp_notification, make_lsp_request};

const INITIALIZE_REQUEST_ID: i32 = 777;

pub struct LspClient {
    transport: SubprocessTransport,
    state: State,
    pending_requests: Vec<Request>,
    request_counter: i32,
}

impl LspClient {
    pub fn new() -> Self {
        let transport = SubprocessTransport::new("rust-analyzer");
        Self {
            transport,
            state: State::Offline,
            pending_requests: Vec::new(),
            request_counter: 0,
        }
    }

    pub fn initialize(&mut self) {
        assert!(self.state == State::Offline);

        self.send_request(&make_initialize_request());

        self.state = State::Initializing;
    }

    pub fn request(&mut self, req: Request) {
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
        for response in self.recv_responses() {
            match response {
                ServerMessage::Response(resp @ lsp::Response { id, .. })
                    if id == INITIALIZE_REQUEST_ID =>
                {
                    self.send_notification(&make_initialized_notification());
                    self.state = State::Online;
                }
                _ => {
                    // TODO some kind of proper logging...
                    panic!(
                        "lsp initializing: server sent a response to something else than the initialize request: {response:?}"
                    );
                }
            }
        }
    }

    fn tick_online(&mut self) {
        for request in std::mem::take(&mut self.pending_requests) {
            let mut lsp_request = convert_request_to_lsp(request);
            self.request_counter += 1;
            lsp_request.id = self.request_counter;
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

    pub fn recv_responses(&mut self) -> Vec<ServerMessage> {
        if let Ok(err) = self.transport.recv_err.try_recv() {
            eprintln!("lsp server err: {}", String::from_utf8_lossy(&err))
        }

        self.transport
            .recv()
            .map(|b| {
                let s = String::from_utf8(b).unwrap();
                let (_, content) = s.split_once("\r\n\r\n").unwrap();
                let message = serde_json::from_str::<lsp::Message>(content).unwrap();
                if message.id.is_some() {
                    let response = serde_json::from_str::<lsp::Response>(content).unwrap();
                    if response.id == 1 {
                        dbg!(&content);
                    }
                    ServerMessage::Response(response)
                } else {
                    let notification = serde_json::from_str::<lsp::Notification>(content).unwrap();
                    ServerMessage::Notification(notification)
                }
            })
            .collect()
    }
}

#[derive(Debug, PartialEq)]
pub enum State {
    Offline,
    Initializing,
    Online,
    ShuttingDown,
}

#[derive(Debug)]
pub enum ServerMessage {
    Response(lsp::Response),
    Notification(lsp::Notification),
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
