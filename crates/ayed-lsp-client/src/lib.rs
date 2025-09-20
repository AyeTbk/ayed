mod event;
pub use event::Event;

mod lsp;

mod request;
pub use request::Request;

mod transport;
use transport::SubprocessTransport;

pub struct LspClient {
    transport: SubprocessTransport,
    state: State,
    pending_requests: Vec<Request>,
}

impl LspClient {
    pub fn new() -> Self {
        let transport = SubprocessTransport::new("rust-analyzer");
        Self {
            transport,
            state: State::Offline,
            pending_requests: Vec::new(),
        }
    }

    pub fn initialize(&mut self) {
        assert!(self.state == State::Offline);

        self.state = State::Initializing;

        // TODO whatever else is needed to begin initializing (send LSP Initialize Reqest)
    }

    pub fn tick(&mut self) {
        match self.state {
            _ => todo!(),
        }
    }

    pub fn request(&mut self, req: Request) {
        todo!()
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        todo!()
    }

    pub fn _old_request(&mut self, request: &lsp::Request) {
        let content = serde_json::to_string(request).unwrap();
        let request = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
        self.transport.send(request.into_bytes())
    }

    pub fn _old_responses(&mut self) -> Vec<lsp::Response> {
        if let Ok(err) = self.transport.recv_err.try_recv() {
            eprintln!("lsp server err: {}", String::from_utf8_lossy(&err))
        }

        self.transport
            .recv()
            .map(|b| {
                let s = String::from_utf8(b).unwrap();
                let (_, content) = s.split_once("\r\n\r\n").unwrap();
                serde_json::from_str::<lsp::Response>(content).unwrap()
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
