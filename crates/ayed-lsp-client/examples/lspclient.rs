use ayed_lsp_client::{File, LspClient, Position, Request, ServerMessage};
use serde_json::{json, value};

fn main() {
    let mut client = LspClient::new();

    client.initialize();

    while !client.is_online() {
        client.tick();
    }

    client.send_notification(&ayed_lsp_client::lsp::Notification {
        jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".into(),
        params: Some(ayed_lsp_client::lsp::NotificationParams::Other(json!({
            "textDocument": {
                "uri": "file:///home/simon/workspaces/rust/ayed/crates/ayed-tui/src/main.rs",
                "languageId": "rust",
                "version": 1,
                "text": include_str!("../../ayed-tui/src/main.rs"),
            }
        }))),
    });

    let mut i: usize = 0;
    loop {
        i += 1;
        if i % 120 == 0 {
            client.request(Request::Hover {
                file: File("/home/simon/workspaces/rust/ayed/crates/ayed-tui/src/main.rs".into()),
                position: Position(6, 15),
            });
        }

        client.tick();
        for response in client.recv_responses() {
            if let ServerMessage::Response(response) = response {
                println!("{:?}", response);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
