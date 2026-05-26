use docolint_server::run;
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, PublishDiagnosticsParams,
};
use serde_json::json;
use std::time::Duration;
use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_did_open_diagnostic_flow() {
    use lsp_types::DiagnosticSeverity;

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": [
                {
                    "message": "Possible spelling mistake found.",
                    "shortMessage": "Spelling mistake",
                    "offset": 14,
                    "length": 5,
                    "replacements": [{ "value": "test" }],
                    "rule": { "id": "MORFOLOGIK_RULE_EN_US", "description": "Possible spelling mistake" },
                    "context": { "text": "This is a testt.", "offset": 14, "length": 5 }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let (server_conn, client_conn) = Connection::memory();

    let params = InitializeParams {
        initialization_options: Some(json!({
            "endpoint": mock_server.uri()
        })),
        ..Default::default()
    };

    let server_handle = tokio::spawn(async move {
        run(server_conn, params).await
    });

    let uri = "file:///test.rs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "rust".to_string(),
                version: 1,
                text: "/// This is a testt.".to_string(),
            },
        },
    );
    client_conn.sender.send(Message::Notification(did_open)).unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut found_diagnostic = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn.receiver.recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
            let diag = &params.diagnostics[0];
            assert_eq!(diag.severity, Some(DiagnosticSeverity::INFORMATION));
            assert_eq!(diag.source.as_deref(), Some("docolint"));
            assert!(diag.message.contains("spelling"), "message: {}", diag.message);
            if let Some(lsp_types::NumberOrString::String(rule_id)) = &diag.code {
                assert_eq!(rule_id, "MORFOLOGIK_RULE_EN_US");
            } else {
                panic!("Expected string rule ID");
            }
            assert_eq!(diag.range.start.line, 0);
            assert!(diag.range.start.character > 0);
            let data = diag.data.as_ref().expect("diagnostic data is missing");
            assert_eq!(data["rule_id"].as_str(), Some("MORFOLOGIK_RULE_EN_US"));
            let replacements = data["replacements"].as_array().expect("replacements should be an array");
            assert!(!replacements.is_empty(), "replacements should not be empty");
            assert_eq!(replacements[0].as_str(), Some("test"));
            found_diagnostic = true;
            break;
        }
    }

    assert!(found_diagnostic, "Did not receive publishDiagnostics within timeout");

    // Clean shutdown
    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}
