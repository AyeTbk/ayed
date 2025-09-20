use ayed_lsp_client::{
    LspClient,
    lsp::{
        ClientCapabilities, ClientCapabilitiesGeneral, InitializeParams, InitializeResult,
        PositionEncodingKind, Request, RequestParams, WorkspaceFolder,
    },
};

fn main() {
    let mut client = LspClient::new();

    let request = Request {
        jsonrpc: "2.0".into(),
        id: 77,
        method: "initialize".into(),
        params: Some(RequestParams::initialize(InitializeParams {
            process_id: None,
            capabilities: ClientCapabilities {
                general: Some(ClientCapabilitiesGeneral {
                    position_encodings: vec![PositionEncodingKind::Utf8],
                }),
            },
            workspace_folders: vec![WorkspaceFolder {
                uri: "file:///home/simon/workspaces/rust/ayed".into(),
                name: "ayed".into(),
            }],
        })),
    };
    client.request(&request);

    println!("==== begin ====");

    loop {
        for response in client.responses() {
            let value = response.result.unwrap();
            let initres: InitializeResult = serde_json::from_value(value).unwrap();
            println!("{:?}", initres);
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
