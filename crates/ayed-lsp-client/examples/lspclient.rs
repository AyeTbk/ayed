use ayed_lsp_client::{File, LspClient, Position, Request};
use serde_json::json;

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
            client.queue_request(Request::Hover {
                file: File("/home/simon/workspaces/rust/ayed/crates/ayed-tui/src/main.rs".into()),
                position: Position(6, 15),
            });
        }

        client.tick();
        let (resps, notifs) = client.recv_server_messages();
        for response in resps {
            println!("response: {:?}", response);
        }
        for notification in notifs {
            println!("notification: {:?}", notification);
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
