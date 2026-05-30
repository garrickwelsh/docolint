use docolint_server::run;
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::{DidOpenTextDocumentParams, InitializeParams, PublishDiagnosticsParams};
use serde_json::json;
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

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
            "endpoint": mock_server.uri(),
            "language": "en-AU",
            "disableSpellCheck": true
        })),
        ..Default::default()
    };

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

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
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut found_diagnostic = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert_eq!(params.diagnostics.len(), 1);
            let diag = &params.diagnostics[0];
            assert_eq!(diag.severity, Some(DiagnosticSeverity::INFORMATION));
            assert_eq!(diag.source.as_deref(), Some("docolint"));
            assert!(
                diag.message.contains("spelling"),
                "message: {}",
                diag.message
            );
            if let Some(lsp_types::NumberOrString::String(rule_id)) = &diag.code {
                assert_eq!(rule_id, "MORFOLOGIK_RULE_EN_US");
            } else {
                panic!("Expected string rule ID");
            }
            assert_eq!(diag.range.start.line, 0);
            assert!(diag.range.start.character > 0);
            let data = diag.data.as_ref().expect("diagnostic data is missing");
            assert_eq!(data["rule_id"].as_str(), Some("MORFOLOGIK_RULE_EN_US"));
            let replacements = data["replacements"]
                .as_array()
                .expect("replacements should be an array");
            assert!(!replacements.is_empty(), "replacements should not be empty");
            assert_eq!(replacements[0].as_str(), Some("test"));
            found_diagnostic = true;
            break;
        }
    }

    assert!(
        found_diagnostic,
        "Did not receive publishDiagnostics within timeout"
    );

    // Clean shutdown
    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_stacked_inline_comments_check_as_one_unit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
        })))
        .mount(&mock_server)
        .await;

    let (server_conn, client_conn) = Connection::memory();

    let params = InitializeParams {
        initialization_options: Some(json!({
            "endpoint": mock_server.uri(),
            "includeInlineComments": true
        })),
        ..Default::default()
    };

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///test.rs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "rust".to_string(),
                version: 1,
                text: "// This is the start of a sentence,\n// this continues the sentence.\nconst S: &str = \"some string\";".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected one LT request, got {requests:#?}"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("text=This+is+the+start+of+a+sentence%2C+this+continues+the+sentence."),
        "unexpected LT request body: {body}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_stacked_rust_doc_comments_check_as_one_unit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///test.rs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "rust".to_string(),
                version: 1,
                text: "/// This is the start of a sentence,\n/// this continues the sentence.\nfn foo() {}".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected one LT request, got {requests:#?}"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("text=This+is+the+start+of+a+sentence%2C+this+continues+the+sentence."),
        "unexpected LT request body: {body}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_stacked_csharp_doc_comments_check_as_one_unit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///test.cs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "csharp".to_string(),
                version: 1,
                text: "/// This is the start of a sentence,\n/// this continues the sentence.\npublic class Foo {}".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected one LT request, got {requests:#?}"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("text=This+is+the+start+of+a+sentence%2C+this+continues+the+sentence."),
        "unexpected LT request body: {body}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rust_doc_paragraph_continuation_stays_in_one_unit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///test.rs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "rust".to_string(),
                version: 1,
                text: "/// Represents a single grammar or spelling error returned by LanguageTool.\n///\n/// This struct maps directly from the LanguageTool API response and is used\n/// internally to track error location, message, and suggested replacements.\npub struct GrammarError;".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        2,
        "expected two LT requests, got {requests:#?}"
    );
    let bodies = requests
        .iter()
        .map(|request| String::from_utf8_lossy(&request.body).into_owned())
        .collect::<Vec<_>>();
    assert!(
        bodies.iter().any(|body| body.contains(
            "text=Represents+a+single+grammar+or+spelling+error+returned+by+LanguageTool."
        )),
        "missing first paragraph request: {bodies:#?}"
    );
    assert!(
        bodies.iter().any(|body| body.contains("text=This+struct+maps+directly+from+the+LanguageTool+API+response+and+is+used+internally+to+track+error+location%2C+message%2C+and+suggested+replacements.")),
        "missing joined continuation request: {bodies:#?}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rust_doc_diagnostic_starts_at_retained_prose_after_blank_doc_line() {
    use lsp_types::DiagnosticSeverity;

    let mock_server = MockServer::start().await;
    let expected_plain_text =
        "Allows clients to configure the LanguageTool endpoint and parser behavior.";
    let allows_offset = expected_plain_text.find("Allows").unwrap();

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": [
                {
                    "message": "Grammar issue.",
                    "shortMessage": "Grammar",
                    "offset": allows_offset,
                    "length": 6,
                    "replacements": [],
                    "rule": { "id": "RULE", "description": "Grammar issue" },
                    "context": { "text": expected_plain_text, "offset": allows_offset, "length": 6 }
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///test.rs";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "rust".to_string(),
                version: 1,
                text: "/// Deserialized from `InitializeParams.initialization_options`.\n///\n/// Allows clients to configure the LanguageTool endpoint and parser behavior.\npub struct InitializationOptions;".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut found_diagnostic = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            if params.diagnostics.is_empty() {
                continue;
            }

            if let Some(diagnostic) = params.diagnostics.iter().find(|diagnostic| {
                diagnostic.range.start.line == 2 && diagnostic.range.start.character == 4
            }) {
                assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::INFORMATION));
                assert_eq!(diagnostic.range.end.line, 2);
                assert_eq!(diagnostic.range.end.character, 10);
                found_diagnostic = true;
                break;
            }
        }
    }

    assert!(
        found_diagnostic,
        "Did not receive publishDiagnostics with expected diagnostic within timeout"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_markdown_paragraph_with_inline_code_checks_as_one_unit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///README.md";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "`docolint` uses `tree-sitter` to extract prose from doc comments, markdown, and other text within source files, then checks it with LanguageTool. Works in Rust, C#, HTML, Markdown, JavaScript/TypeScript, Python, and more.".to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected one LT request, got {requests:#?}"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("text=%60docolint%60+uses+%60tree-sitter%60+to+extract+prose+from+doc+comments%2C+markdown%2C+and+other+text+within+source+files%2C+then+checks+it+with+LanguageTool.+Works+in+Rust%2C+C%23%2C+HTML%2C+Markdown%2C+JavaScript%2FTypeScript%2C+Python%2C+and+more."),
        "unexpected LT request body: {body}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_markdown_inline_code_after_comma_preserves_space() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "software": { "name": "LanguageTool", "version": "6.5" },
            "warnings": {},
            "language": { "name": "English (US)", "code": "en-US", "detectedLanguage": "en-US" },
            "matches": []
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

    let server_handle = tokio::spawn(async move { run(server_conn, params).await });

    let uri = "file:///README.md";
    let line = "A running LanguageTool HTTP server. By default, `docolint` expects one at `http://localhost:8081`.";
    let did_open = Notification::new(
        "textDocument/didOpen".to_string(),
        DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: serde_json::from_str(&format!("\"{}\"", uri)).unwrap(),
                language_id: "markdown".to_string(),
                version: 1,
                text: line.to_string(),
            },
        },
    );
    client_conn
        .sender
        .send(Message::Notification(did_open))
        .unwrap();

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_publish = false;

    while start.elapsed() < timeout {
        if let Ok(Message::Notification(not)) = client_conn
            .receiver
            .recv_timeout(Duration::from_millis(100))
            && not.method == "textDocument/publishDiagnostics"
        {
            let params: PublishDiagnosticsParams = serde_json::from_value(not.params).unwrap();
            assert!(
                params.diagnostics.is_empty(),
                "got diagnostics: {:?}",
                params.diagnostics
            );
            saw_publish = true;
            break;
        }
    }

    assert!(
        saw_publish,
        "Did not receive publishDiagnostics within timeout"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "expected one LT request, got {requests:#?}"
    );
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains("text=A+running+LanguageTool+HTTP+server.+By+default%2C+%60docolint%60+expects+one+at+%60http%3A%2F%2Flocalhost%3A8081%60."),
        "unexpected LT request body: {body}"
    );

    let shutdown = Request::new(1.into(), "shutdown".to_string(), serde_json::Value::Null);
    client_conn.sender.send(Message::Request(shutdown)).unwrap();
    let _ = client_conn.receiver.recv_timeout(Duration::from_secs(2));

    drop(client_conn);
    let _ = server_handle.await;
}
