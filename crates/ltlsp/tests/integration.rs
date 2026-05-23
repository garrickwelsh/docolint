use std::process::{Command, Stdio};
use std::io::{Read, Write, BufRead, BufReader};
use std::time::{Duration, Instant};

fn send_message(stdin: &mut impl Write, body: &str) {
    let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    stdin.write_all(msg.as_bytes()).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");
}

fn read_message(reader: &mut BufReader<impl Read>) -> Option<String> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.starts_with("Content-Length:") {
            content_length = Some(trimmed["Content-Length:".len()..].trim().parse().ok()?);
        }
    }

    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    Some(String::from_utf8(buf).ok()?)
}

#[test]
fn test_binary_handshake() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "ltlsp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{},"initializationOptions":{"endpoint":"http://localhost:8081"}}}"#;
    send_message(&mut stdin, init_req);

    let resp = read_message(&mut reader).expect("No response from server");
    assert!(resp.contains("\"result\""), "Expected initialize result, got: {}", resp);

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    send_message(&mut stdin, initialized);

    let shutdown_req = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    send_message(&mut stdin, shutdown_req);
    read_message(&mut reader);

    let exit_req = r#"{"jsonrpc":"2.0","method":"exit"}"#;
    send_message(&mut stdin, exit_req);

    child.wait().expect("Failed to wait for child");
}

/// Requires LanguageTool running at localhost:8081.
/// Run with `cargo test -- --include-ignored` or `just test-all`.
#[ignore]
#[test]
fn test_full_lsp_flow_with_lt() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "ltlsp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{},"initializationOptions":{"endpoint":"http://localhost:8081"}}}"#;
    send_message(&mut stdin, init_req);
    let resp = read_message(&mut reader).expect("No initialize response");
    assert!(resp.contains("\"result\""), "Expected initialize result, got: {}", resp);

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    send_message(&mut stdin, initialized);

    let did_open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.txt","languageId":"plaintext","version":1,"text":"This is a testt. It has a speling mistake."}}}"#;
    send_message(&mut stdin, did_open);

    let timeout = Duration::from_secs(15);
    let start = Instant::now();
    let mut found_diagnostic = false;

    while start.elapsed() < timeout {
        if let Some(body) = read_message(&mut reader) {
            if body.contains("textDocument/publishDiagnostics") {
                let json: serde_json::Value = serde_json::from_str(&body).expect("Invalid JSON");
                let diagnostics = json["params"]["diagnostics"].as_array().expect("diagnostics is not an array");
                assert!(!diagnostics.is_empty(), "Expected at least one diagnostic");

                let diag = &diagnostics[0];
                assert!(diag["message"].as_str().is_some_and(|s| !s.is_empty()), "diagnostic message is empty");
                assert_eq!(diag["source"].as_str(), Some("ltlsp"), "source is not 'ltlsp'");
                assert!(diag["code"].is_string() || diag["code"].is_number(), "code is missing");

                let range = &diag["range"];
                assert_eq!(range["start"]["line"].as_i64(), Some(0), "range start line is not 0");

                found_diagnostic = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(found_diagnostic, "Did not receive publishDiagnostics within timeout");

    let shutdown_req = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    send_message(&mut stdin, shutdown_req);
    read_message(&mut reader);

    let exit_req = r#"{"jsonrpc":"2.0","method":"exit"}"#;
    send_message(&mut stdin, exit_req);

    child.wait().expect("Failed to wait for child");
}
