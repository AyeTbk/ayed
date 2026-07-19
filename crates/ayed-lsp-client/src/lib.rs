use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

pub mod types;

mod completion;
pub use completion::Completion;

mod notification;
pub use notification::Notification;

mod request;

mod response;
pub use response::Response;

mod transport;
use transport::SubprocessTransport;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    notification::convert_notification_to_json,
    request::{
        PendingRequest, RequestType, build_definition_request_json, build_hover_request_json,
        build_initialize_request_json, build_resolve_completion_request_json,
        build_signature_help_request_json, build_suggest_completion_request_json,
    },
    types::{CompletionItem, CompletionItemId, Location, Position, SignatureHelp, TextDocumentIdentifier},
};

const INITIALIZE_REQUEST_ID: i32 = 1;

pub struct LspClient {
    transport: SubprocessTransport,
    state: State,

    pending_requests: Vec<PendingRequest>,
    pending_notifications: Vec<Notification>,

    request_counter: i32,
    request_metadata: HashMap<i32, RequestMetadata>,

    completion: Completion,
}

impl LspClient {
    pub fn new(server_command: &str, notify_async_done: Arc<AtomicBool>) -> Self {
        let transport = SubprocessTransport::new(server_command, notify_async_done);
        Self {
            transport,
            state: State::Offline,
            pending_requests: Vec::new(),
            pending_notifications: Vec::new(),
            request_counter: INITIALIZE_REQUEST_ID,
            request_metadata: HashMap::new(),
            completion: Default::default(),
        }
    }

    pub fn initialize(&mut self) {
        assert!(self.state == State::Offline);

        self.queue_initialize_request();
        self.send_messages();

        self.state = State::Initializing;
    }

    pub fn shutdown(self) {
        self.transport.shutdown();
    }

    pub fn queue_notification(&mut self, notif: Notification) {
        self.pending_notifications.push(notif);
    }

    pub fn queue_initialize_request(&mut self) {
        let id = self.take_request_id();
        let json = build_initialize_request_json(id);
        self.queue_request(PendingRequest {
            id,
            typ: RequestType::Initialize,
            json,
        });
    }

    pub fn queue_suggest_completion_request(
        &mut self,
        text_document: TextDocumentIdentifier,
        position: Position,
    ) {
        let id = self.take_request_id();
        let json = build_suggest_completion_request_json(id, text_document, position);
        self.queue_request(PendingRequest {
            id,
            typ: RequestType::SuggestCompletion,
            json,
        });
    }

    // NOTE: for internal use only
    fn queue_resolve_completion_request(&mut self, completion_item_idx: u32) {
        let completion_item_id = CompletionItemId {
            idx: completion_item_idx as u32,
            generation: self.completion.generation(),
        };
        let id = self.take_request_id();
        let maybe_item = self.completion.items().get(completion_item_id.idx as usize);
        if let Some(item) = maybe_item
            && completion_item_id.generation == self.completion.generation()
        {
            let json = build_resolve_completion_request_json(id, item);
            self.queue_request(PendingRequest {
                id,
                typ: RequestType::ResolveCompletion,
                json,
            });

            self.request_metadata_mut(id).completion_item_id = completion_item_id;
        } else {
            log::warn!("ignoring stale completion item resolve request");
        }
    }

    pub fn queue_signature_help_request(
        &mut self,
        text_document: TextDocumentIdentifier,
        position: Position,
    ) {
        let id = self.take_request_id();
        let json = build_signature_help_request_json(id, text_document, position);
        self.queue_request(PendingRequest {
            id,
            typ: RequestType::SignatureHelp,
            json,
        });
    }

    pub fn queue_hover_request(
        &mut self,
        text_document: TextDocumentIdentifier,
        position: Position,
    ) {
        let id = self.take_request_id();
        let json = build_hover_request_json(id, text_document, position);
        self.queue_request(PendingRequest {
            id,
            typ: RequestType::Hover,
            json,
        });
    }

    pub fn queue_definition_request(
        &mut self,
        text_document: TextDocumentIdentifier,
        position: Position,
    ) {
        let id = self.take_request_id();
        let json = build_definition_request_json(id, text_document, position);
        self.queue_request(PendingRequest {
            id,
            typ: RequestType::Definition,
            json,
        });
    }

    fn take_request_id(&mut self) -> i32 {
        let id = self.request_counter;
        self.request_counter += 1;
        id
    }

    fn queue_request(&mut self, req: PendingRequest) {
        self.request_metadata_mut(req.id).typ = req.typ;
        self.pending_requests.push(req);
    }

    pub fn is_online(&self) -> bool {
        self.state == State::Online
    }

    pub fn is_just_initialized(&self) -> bool {
        self.state == State::Initialized
    }

    pub fn completion_items(&self) -> &[CompletionItem] {
        self.completion.items()
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
                ServerResponse { id, .. } if id == INITIALIZE_REQUEST_ID => {
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
        self.send_messages()
    }

    fn send_messages(&mut self) {
        for notif in std::mem::take(&mut self.pending_notifications) {
            self.send_notification(notif);
        }
        for req in std::mem::take(&mut self.pending_requests) {
            self.send_request(req);
        }
    }

    fn send_request(&mut self, req: PendingRequest) {
        let req_string = serde_json::to_string(&req.json).unwrap();
        self.send_message(req_string);
    }

    fn send_notification(&mut self, notif: Notification) {
        let notif_json = convert_notification_to_json(notif);
        self.send_json(&notif_json);
    }

    fn send_json(&mut self, content: &Value) {
        let json_string = serde_json::to_string(content).unwrap();
        self.send_message(json_string);
    }

    fn send_message(&mut self, content: String) {
        let full_message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
        self.transport.send(full_message.into_bytes())
    }

    fn recv_server_messages(&mut self) -> (Vec<ServerResponse>, Vec<Value>) {
        if let Ok(err) = self.transport.recv_server_err.try_recv() {
            log::debug!("lsp server err: {}", String::from_utf8_lossy(&err))
        }

        let mut responses = Vec::new();
        let mut notifications = Vec::new();

        for b in self.transport.recv() {
            let content = String::from_utf8(b).unwrap();
            let message = serde_json::from_str::<ServerMessage>(&content).unwrap();
            if message.id.is_some() {
                let response = serde_json::from_str::<ServerResponse>(&content).unwrap();
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
            log::debug!("{:?}", notif);
        }

        let mut responses = Vec::new();
        for resp in resps {
            // log::debug!("{resp:?}"); // DEBUG
            // FIXME parse all those json values directly with serde instead of hand wrangling some shit.
            // FIXME and some of those responses can be null while others cant so the code right below is a problem.
            let Some(mut resp_result) = resp.result else {
                log::debug!("server response (id: {}) is malformed", resp.id);
                continue;
            };
            if let Some(resp_error) = resp.error {
                log::debug!(
                    "server responded with an error (id: {}): {:?}",
                    resp.id,
                    resp_error
                );
                continue;
            }

            use serde_json::Value;

            let get_completion_result = |result: &mut Value| -> Option<Vec<CompletionItem>> {
                let items = result.pointer_mut("/items")?.take();
                let Value::Array(items_arr) = items else {
                    return None;
                };
                let items = items_arr
                    .into_iter()
                    .filter_map(|item| {
                        if let Some(p) = item.pointer("/additionalTextEdits") {
                            log::debug!("{:?}", p);
                        }
                        let item: CompletionItem = serde_json::from_value(item).ok()?;
                        Some(item)
                    })
                    .collect();
                Some(items)
            };
            let get_completion_resolve_result = |result: &mut Value| -> Option<CompletionItem> {
                let item: CompletionItem = serde_json::from_value(result.take()).unwrap(); //.ok()?;
                Some(item)
            };
            let get_hover_result = |result: &mut Value| -> Option<String> {
                if let Value::String(text) = result.pointer_mut("/contents/value")?.take() {
                    Some(text)
                } else {
                    None
                }
            };
            let get_definition_result = |result: &mut Value| -> Option<Vec<Location>> {
                let result = result.take();
                let results = match result {
                    Value::Array(results) => results,
                    Value::Object(_) => vec![result],
                    Value::Null => vec![],
                    _ => unimplemented!("bad goto location: {result:?}"),
                };
                let loc: Vec<Location> = serde_json::from_value(Value::Array(results)).ok()?;
                Some(loc)
            };

            let Some(request_metadata) = self.request_metadata.remove(&resp.id) else {
                log::debug!("lsp response without associated request. id {}", resp.id);
                continue;
            };
            match request_metadata.typ {
                RequestType::Initialize => unimplemented!("not supposed to happen"),
                RequestType::SuggestCompletion => {
                    if let Some(items) = get_completion_result(&mut resp_result) {
                        // Cool, we got the completion items, but they are incomplete.
                        // Let's resolve the remaining information before indicating that
                        // the items are ready.
                        self.completion.set_items(items);
                        for i in 0..self.completion.items().len() {
                            self.queue_resolve_completion_request(i as u32);
                        }
                    } else {
                        unimplemented!("{resp_result:?}");
                    }
                }
                RequestType::ResolveCompletion => {
                    if let Some(item) = get_completion_resolve_result(&mut resp_result) {
                        let idx = request_metadata.completion_item_id.idx;
                        let generation = request_metadata.completion_item_id.generation;
                        if self.completion.resolve_item(idx, generation, item) {
                            if self.completion.all_items_are_resolved() {
                                responses.push(Response::CompletionSuggestionsAvailable);
                            }
                        }
                    } else if resp_result == Value::Null {
                        log::warn!("completion item resolve got null response.");
                        continue;
                    } else {
                        unimplemented!("{resp_result:?}");
                    }
                }
                RequestType::SignatureHelp => {
                    let Ok(sighelp) = serde_json::from_value::<SignatureHelp>(resp_result) else {
                        log::debug!("lsp signature help: received bad json");
                        continue;
                    };
                    let Some(active_signature) = sighelp.active_signature else {
                        log::debug!("lsp signature help: no active signature");
                        continue;
                    };
                    let text = sighelp.signatures[active_signature as usize].label.to_string();
                    responses.push(Response::SignatureHelp { text });
                }
                RequestType::Hover => {
                    if let Some(text) = get_hover_result(&mut resp_result) {
                        responses.push(Response::HoverInfo { text });
                    } else {
                        unimplemented!("{resp_result:?}");
                    }
                }
                RequestType::Definition => {
                    if let Some(locations) = get_definition_result(&mut resp_result) {
                        responses.push(Response::GoToDefinitionInfo { locations });
                    } else {
                        unimplemented!("{resp_result:?}");
                    }
                }
            }
        }

        responses
    }

    fn request_metadata_mut(&mut self, request_id: i32) -> &mut RequestMetadata {
        self.request_metadata.entry(request_id).or_default()
    }
}

#[derive(Default)]
struct RequestMetadata {
    typ: RequestType,
    completion_item_id: CompletionItemId,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Offline,
    Initializing,
    Initialized,
    Online,
    ShuttingDown,
}

#[derive(Serialize, Deserialize)]
struct ServerMessage {
    pub id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerResponse {
    pub id: i32,
    pub result: Option<serde_json::Value>,
    pub error: Option<ServerResponseError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
