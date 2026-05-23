use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, Position, PublishDiagnosticsParams, Range,
};
use ltlsp_client::{ClientConfig, LanguageToolClient};
use ltlsp_dictionary::Dictionary;
use ltlsp_parser::parse_document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use url::Url;

pub fn generate_ignore_actions(
    workspace_root: &Path,
    document_path: &Path,
    word: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let mut current = document_path.parent();

    while let Some(path) = current {
        let title = if path == workspace_root {
            format!("Ignore '{}' in workspace root", word)
        } else {
            format!(
                "Ignore '{}' in {}",
                word,
                path.file_name().unwrap_or_default().to_string_lossy()
            )
        };

        let action = CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            ..Default::default()
        };
        actions.push(CodeActionOrCommand::CodeAction(action));

        if path == workspace_root {
            break;
        }
        current = path.parent();
    }

    actions
}

use ltlsp_types::AnnotatedText;

pub fn map_lt_offset_to_absolute(text: &AnnotatedText, lt_offset: usize) -> Option<usize> {
    let mut current_lt_offset = 0;
    for segment in &text.segments {
        if segment.is_markup {
            continue;
        }
        let next_lt_offset = current_lt_offset + segment.text.len();
        if lt_offset >= current_lt_offset && lt_offset < next_lt_offset {
            let offset_in_segment = lt_offset - current_lt_offset;
            return Some(segment.offset + offset_in_segment);
        }
        current_lt_offset = next_lt_offset;
    }
    None
}

pub fn offset_to_position(content: &str, offset: usize) -> lsp_types::Position {
    let mut line = 0;
    let mut character = 0;
    for (i, c) in content.char_indices() {
        if i == offset {
            break;
        }
        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    lsp_types::Position { line, character }
}

pub struct ServerState {
    pub client: LanguageToolClient,
    pub document_versions: HashMap<String, i32>,
    pub document_content: HashMap<String, String>,
    pub document_languages: HashMap<String, String>,
    pub in_flight_tasks: HashMap<String, JoinHandle<()>>,
    pub last_error_time: Option<Instant>,
    pub error_cooldown: Duration,
    pub workspace_root: Option<PathBuf>,
    pub started_lt: bool,
    pub stop_on_exit: bool,
}

impl ServerState {
    pub fn new(client: LanguageToolClient) -> Self {
        Self {
            client,
            document_versions: HashMap::new(),
            document_content: HashMap::new(),
            document_languages: HashMap::new(),
            in_flight_tasks: HashMap::new(),
            last_error_time: None,
            error_cooldown: Duration::from_secs(60),
            workspace_root: None,
            started_lt: false,
            stop_on_exit: false,
        }
    }

    pub fn mark_error(&mut self) {
        self.last_error_time = Some(Instant::now());
    }

    pub fn is_cooling_down(&self) -> bool {
        if let Some(last_error) = self.last_error_time {
            last_error.elapsed() < self.error_cooldown
        } else {
            false
        }
    }

    pub fn update_version(&mut self, uri: String, version: i32) {
        self.document_versions.insert(uri, version);
    }

    pub fn cancel_task(&mut self, uri: &str) {
        if let Some(handle) = self.in_flight_tasks.remove(uri) {
            handle.abort();
        }
    }

    pub fn register_task(&mut self, uri: String, handle: JoinHandle<()>) {
        self.cancel_task(&uri);
        self.in_flight_tasks.insert(uri, handle);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    pub endpoint: Option<String>,
    pub stop_on_exit: Option<bool>,
}

const LT_DOCKER_IMAGE: &str = "ghcr.io/garrickwelsh/languagetool";
const LT_CONTAINER_NAME: &str = "ltlsp-lt-server";

fn probe_language_tool(endpoint: &str) -> bool {
    let url = match Url::parse(endpoint) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let host = match url.host_str() {
        Some(h) => h,
        None => return false,
    };
    let port = url.port_or_known_default().unwrap_or(8081);
    let addr = match format!("{}:{}", host, port).parse::<std::net::SocketAddr>() {
        Ok(a) => a,
        Err(_) => return false,
    };
    std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

fn start_language_tool(_endpoint: &str, sender: &crossbeam_channel::Sender<Message>) {
    let show_message = |msg: &str| {
        let params = lsp_types::ShowMessageParams {
            typ: lsp_types::MessageType::INFO,
            message: msg.to_string(),
        };
        let not = Notification::new("window/showMessage".to_string(), params);
        let _ = sender.send(Message::Notification(not));
    };

    show_message("LanguageTool not reachable. Starting Docker container...");

    let start_result = std::process::Command::new("docker")
        .args(["start", LT_CONTAINER_NAME])
        .output();

    if start_result.is_ok() {
        show_message("LanguageTool container started. Warming up...");
        return;
    }

    let run_result = std::process::Command::new("docker")
        .args([
            "run", "-d",
            "--network", "host",
            "--name", LT_CONTAINER_NAME,
            LT_DOCKER_IMAGE,
        ])
        .output();

    match run_result {
        Ok(output) if output.status.success() => {
            show_message("LanguageTool container created. Warming up...");
        }
        _ => {
            show_message("Failed to start LanguageTool via Docker. Check that Docker is running.");
        }
    }
}

fn spawn_check(
    state: Arc<RwLock<ServerState>>,
    sender: crossbeam_channel::Sender<Message>,
    uri: String,
    version: i32,
    content: String,
    lang: String,
) {
    let state_task = state.clone();
    let uri_task = uri.clone();
    let handle = tokio::spawn(async move {
        // Debounce
        tokio::time::sleep(Duration::from_millis(500)).await;

        let (client, is_cooling, root) = {
            let s = state_task.read().unwrap();
            (s.client.clone(), s.is_cooling_down(), s.workspace_root.clone())
        };

        if is_cooling {
            return;
        }

        let annotated = parse_document(&lang, &content);
        let result = client.check(annotated.clone()).await;

        match result {
            Ok(errors) => {
                let dict = if let Some(root) = root {
                    if let Ok(u) = Url::parse(&uri_task) {
                        if let Ok(path) = u.to_file_path() {
                            Dictionary::load(&root, &path)
                        } else {
                            Dictionary::new()
                        }
                    } else {
                        Dictionary::new()
                    }
                } else {
                    Dictionary::new()
                };

                let filtered = dict.filter_errors(&annotated.plain_text(), errors);
                let mut diagnostics = Vec::new();

                for err in filtered {
                    if let Some(abs_offset) = map_lt_offset_to_absolute(&annotated, err.offset) {
                        let start = offset_to_position(&content, abs_offset);
                        let end = offset_to_position(&content, abs_offset + err.length);
                        
                        diagnostics.push(Diagnostic {
                            range: Range { start, end },
                            severity: Some(lsp_types::DiagnosticSeverity::INFORMATION),
                            code: Some(lsp_types::NumberOrString::String(err.rule_id.clone())),
                            source: Some("ltlsp".to_string()),
                            message: err.message,
                            data: Some(serde_json::to_value(err.rule_id).unwrap_or_default()),
                            ..Default::default()
                        });
                    }
                }

                let params = PublishDiagnosticsParams {
                    uri: serde_json::from_value(serde_json::to_value(&uri_task).unwrap()).unwrap(),
                    diagnostics,
                    version: Some(version),
                };

                let not = Notification::new("textDocument/publishDiagnostics".to_string(), params);
                let _ = sender.send(Message::Notification(not));
            }
            Err(_) => {
                state_task.write().unwrap().mark_error();
            }
        }
    });

    state.write().unwrap().register_task(uri, handle);
}

pub async fn run(connection: Connection, params: InitializeParams) -> Result<(), Box<dyn Error + Send + Sync>> {
    let options: InitializationOptions = params
        .initialization_options
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(InitializationOptions { endpoint: None, stop_on_exit: None });

    let endpoint = options.endpoint.unwrap_or_else(|| "http://localhost:8081".to_string());
    let stop_on_exit = options.stop_on_exit.unwrap_or(false);
    
    let client = LanguageToolClient::new(ClientConfig {
        base_url: endpoint.clone(),
    });

    let mut state_raw = ServerState::new(client);
    state_raw.workspace_root = params.workspace_folders.and_then(|folders| {
        let first = folders.first()?;
        let url: Url = serde_json::from_value(serde_json::to_value(&first.uri).ok()?).ok()?;
        url.to_file_path().ok()
    });
    state_raw.stop_on_exit = stop_on_exit;
    let state = Arc::new(RwLock::new(state_raw));

    if !probe_language_tool(&endpoint) {
        start_language_tool(&endpoint, &connection.sender);
        state.write().unwrap().started_lt = true;
    }

    let state_for_loop = state.clone();
    tokio::task::spawn_blocking(move || {
        while let Ok(msg) = connection.receiver.recv() {
            match msg {
                Message::Request(req) => {
                    if let Ok(true) = connection.handle_shutdown(&req) {
                        if state_for_loop.read().unwrap().stop_on_exit {
                            let _ = std::process::Command::new("docker")
                                .args(["stop", LT_CONTAINER_NAME])
                                .output();
                        }
                        return Ok(());
                    }
                    
                    if req.method == "textDocument/codeAction"
                        && let Ok(params) = serde_json::from_value::<CodeActionParams>(req.params) {
                        let uri = params.text_document.uri.clone();
                        let root = state_for_loop.read().unwrap().workspace_root.clone().unwrap_or_default();
                        
                        let mut actions = Vec::new();
                        
                        let uri_val = serde_json::to_value(&uri).unwrap();
                        if let Ok(url) = serde_json::from_value::<Url>(uri_val) {
                            let path = url.to_file_path().unwrap_or_default();

                            for diag in params.context.diagnostics {
                                let content = state_for_loop.read().unwrap().document_content.get(&uri.to_string()).cloned();
                                if let Some(content) = content {
                                    let start_offset = position_to_offset(&content, diag.range.start);
                                    let end_offset = position_to_offset(&content, diag.range.end);
                                    if start_offset < end_offset && end_offset <= content.len() {
                                        let word = &content[start_offset..end_offset];
                                        actions.extend(generate_ignore_actions(&root, &path, word));
                                    }
                                }
                            }
                        }
                        
                        let result = serde_json::to_value(actions).unwrap();
                        let resp = Response::new_ok(req.id, result);
                        let _ = connection.sender.send(Message::Response(resp));
                    }
                }
                Message::Notification(not) => {
                    match not.method.as_str() {
                        "textDocument/didOpen" => {
                            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                                let uri = params.text_document.uri.to_string();
                                let version = params.text_document.version;
                                let content = params.text_document.text;
                                let lang = params.text_document.language_id;
                                
                                {
                                    let mut state_w = state_for_loop.write().unwrap();
                                    state_w.update_version(uri.clone(), version);
                                    state_w.document_content.insert(uri.clone(), content.clone());
                                    state_w.document_languages.insert(uri.clone(), lang.clone());
                                }
                                
                                spawn_check(state_for_loop.clone(), connection.sender.clone(), uri, version, content, lang);
                            }
                        }
                        "textDocument/didChange" => {
                            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(not.params) {
                                let uri = params.text_document.uri.to_string();
                                let version = params.text_document.version;
                                if let Some(change) = params.content_changes.into_iter().next() {
                                    let content = change.text;
                                    
                                    let lang = {
                                        let mut state_w = state_for_loop.write().unwrap();
                                        state_w.update_version(uri.clone(), version);
                                        state_w.document_content.insert(uri.clone(), content.clone());
                                        state_w.document_languages.get(&uri).cloned().unwrap_or_else(|| "plain".to_string())
                                    };
                                    
                                spawn_check(state_for_loop.clone(), connection.sender.clone(), uri, version, content, lang);

                                }
                            }
                        }
                        "textDocument/didClose" => {
                            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(not.params) {
                                let uri = params.text_document.uri.to_string();
                                let mut state_w = state_for_loop.write().unwrap();
                                state_w.cancel_task(&uri);
                                state_w.document_versions.remove(&uri);
                                state_w.document_content.remove(&uri);
                                state_w.document_languages.remove(&uri);
                            }
                        }
                        _ => {}
                    }
                }
                Message::Response(_) => {}
            }
        }
        Ok(())
    }).await?
}

fn position_to_offset(content: &str, pos: Position) -> usize {
    let mut current_line = 0;
    for (i, c) in content.char_indices() {
        if current_line == pos.line {
            for (char_count, (j, _)) in content[i..].char_indices().enumerate() {
                if char_count == pos.character as usize {
                    return i + j;
                }
            }
            return content.len();
        }
        if c == '\n' {
            current_line += 1;
        }
    }
    content.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::InitializeParams;
    use serde_json::json;

    #[tokio::test]
    async fn test_initialization_options_extraction() {
        let (server_conn, client_conn) = Connection::memory();
        let params = InitializeParams {
            initialization_options: Some(json!({
                "endpoint": "http://custom-lt:8081"
            })),
            ..Default::default()
        };

        let server_handle = tokio::spawn(async move {
            run(server_conn, params).await
        });
        
        drop(client_conn);
        let result = server_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_offset_mapping() {
        use ltlsp_types::TextSegment;
        let text = AnnotatedText {
            segments: vec![
                TextSegment { text: "Hello ".to_string(), is_markup: false, offset: 0 },
                TextSegment { text: "<b>".to_string(), is_markup: true, offset: 6 },
                TextSegment { text: "world".to_string(), is_markup: false, offset: 9 },
            ],
        };
        
        assert_eq!(map_lt_offset_to_absolute(&text, 0), Some(0)); // 'H'
        assert_eq!(map_lt_offset_to_absolute(&text, 6), Some(9)); // 'w'
        assert_eq!(map_lt_offset_to_absolute(&text, 10), Some(13)); // 'd'
        assert_eq!(map_lt_offset_to_absolute(&text, 11), None);
    }

    #[test]
    fn test_offset_to_position() {
        let content = "line1\nline2";
        assert_eq!(offset_to_position(content, 0), lsp_types::Position { line: 0, character: 0 });
        assert_eq!(offset_to_position(content, 5), lsp_types::Position { line: 0, character: 5 });
        assert_eq!(offset_to_position(content, 6), lsp_types::Position { line: 1, character: 0 });
    }

    #[test]
    fn test_circuit_breaker() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
        });
        let mut state = ServerState::new(client);
        state.error_cooldown = Duration::from_millis(100);
        
        assert!(!state.is_cooling_down());
        state.mark_error();
        assert!(state.is_cooling_down());
        
        std::thread::sleep(Duration::from_millis(150));
        assert!(!state.is_cooling_down());
    }

    #[tokio::test]
    async fn test_full_diagnostic_pipeline() {
        use ltlsp_parser::parse_document;
        use ltlsp_dictionary::Dictionary;
        
        // This test requires a live LT server or we can mock it.
        // Since Cycle 2 tested wiremock, let's just test the orchestration here
        // using the components we've built.
        
        let content = "/// This is a testt.";

        let annotated = parse_document("rust", content);
        assert_eq!(annotated.plain_text().trim(), "This is a testt.");
        
        let errors = vec![ltlsp_types::GrammarError {
            message: "Spelling error".to_string(),
            offset: 10,
            length: 5,
            replacements: vec!["test".to_string()],
            rule_id: "SPELLING".to_string(),
        }];
        
        let dict = Dictionary::new();
        let filtered = dict.filter_errors(&annotated.plain_text(), errors);
        assert_eq!(filtered.len(), 1);
        
        let abs_offset = map_lt_offset_to_absolute(&annotated, filtered[0].offset).unwrap();
        let pos = offset_to_position(content, abs_offset);
        
        assert_eq!(pos.line, 0);
        assert!(pos.character > 0);
    }

    #[test]
    fn test_code_action_generation() {
        let root = Path::new("/workspaces/project");
        let sub = root.join("src/module");
        let doc = sub.join("file.rs");
        
        let actions = generate_ignore_actions(root, &doc, "typo");
        
        assert_eq!(actions.len(), 3);
        
        if let CodeActionOrCommand::CodeAction(a) = &actions[0] {
            assert!(a.title.contains("module"));
        }
        if let CodeActionOrCommand::CodeAction(a) = &actions[2] {
            assert!(a.title.contains("workspace root"));
        }
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
        });
        let mut state = ServerState::new(client);
        let uri = "file:///test.rs".to_string();

        let handle1 = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        });
        state.register_task(uri.clone(), handle1);
        
        let handle2 = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        });
        
        state.register_task(uri.clone(), handle2);
    }

    #[test]
    fn test_position_to_offset() {
        let content = "a\nbc";
        assert_eq!(position_to_offset(content, Position { line: 0, character: 0 }), 0);
        assert_eq!(position_to_offset(content, Position { line: 0, character: 1 }), 1);
        assert_eq!(position_to_offset(content, Position { line: 1, character: 0 }), 2);
        assert_eq!(position_to_offset(content, Position { line: 1, character: 1 }), 3);
        assert_eq!(position_to_offset(content, Position { line: 1, character: 2 }), 4);
    }
}
