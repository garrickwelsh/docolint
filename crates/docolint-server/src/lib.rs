use docolint_client::{ClientConfig, LanguageToolClient};
use docolint_dictionary::Dictionary;
use docolint_parser::{ParserConfig, parse_document};
use docolint_types::GrammarError;
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Command, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentEdit, TextEdit, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use url::Url;

/// Returns the server's capabilities as a JSON value.
///
/// Advertises support for:
/// - Full text document sync (open/close + full content on change)
/// - Code action provider (for quick fixes and ignore-word actions)
/// - Execute command provider (`docolint.ignoreWord`)
pub fn server_capabilities() -> serde_json::Value {
    serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp_types::TextDocumentSyncKind::FULL),
                ..Default::default()
            },
        )),
        code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
        execute_command_provider: Some(lsp_types::ExecuteCommandOptions {
            commands: vec!["docolint.ignoreWord".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap()
}

/// Generates "ignore word" code actions for each `.docolint-ignore` file between
/// the document and the workspace root.
///
/// Each action, when executed by the editor, sends a `docolint.ignoreWord` command
/// with the word and target file path.
///
/// # Arguments
/// * `workspace_root` - Root of the workspace. Used to label the root-level action.
/// * `document_path` - Path to the source file being edited. Walking starts from
///   this file's parent directory.
/// * `word` - The word to offer ignoring.
/// * `uri` - The document's LSP URI, passed to the command for rechecking after ignore.
///
/// # Returns
/// A vector of `CodeActionOrCommand`, one per directory level up to (and including)
/// the workspace root.
pub fn generate_ignore_actions(
    workspace_root: &Path,
    document_path: &Path,
    word: &str,
    uri: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let mut current = document_path.parent();

    while let Some(path) = current {
        let ignore_file = path.join(".docolint-ignore");
        let ignore_file_str = ignore_file.to_string_lossy().to_string();

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
            title: title.clone(),
            kind: Some(CodeActionKind::QUICKFIX),
            command: Some(Command {
                title,
                command: "docolint.ignoreWord".to_string(),
                arguments: Some(vec![
                    serde_json::Value::String(word.to_string()),
                    serde_json::Value::String(ignore_file_str),
                    serde_json::Value::String(uri.to_string()),
                ]),
            }),
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

/// Generates replacement code actions from a diagnostic's suggested replacements.
///
/// The first replacement is marked as `is_preferred` for editor auto-selection.
///
/// # Arguments
/// * `diag` - The LSP diagnostic containing replacement data in its `data` field.
///   Expected format: `{ "replacements": ["word1", "word2", ...] }`.
/// * `uri` - The document URI to apply the text edit to.
/// * `_content` - Unused. Reserved for future context-aware filtering.
///
/// # Returns
/// A vector of `CodeActionOrCommand`, one per replacement. Empty if no replacements exist.
pub fn generate_replacement_actions(
    diag: &Diagnostic,
    uri: &lsp_types::Uri,
    _content: &str,
) -> Vec<CodeActionOrCommand> {
    let replacements: Vec<String> = diag
        .data
        .as_ref()
        .and_then(|d| d.get("replacements"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if replacements.is_empty() {
        return Vec::new();
    }

    let mut actions = Vec::new();
    for (i, replacement) in replacements.iter().enumerate() {
        let edit = WorkspaceEdit {
            document_changes: Some(lsp_types::DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: None,
                },
                edits: vec![OneOf::Left(TextEdit {
                    range: diag.range,
                    new_text: replacement.clone(),
                })],
            }])),
            ..Default::default()
        };

        let action = CodeAction {
            title: format!("Replace with '{}'", replacement),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(edit),
            is_preferred: if i == 0 { Some(true) } else { None },
            ..Default::default()
        };
        actions.push(CodeActionOrCommand::CodeAction(action));
    }

    actions
}

use docolint_types::AnnotatedText;

/// Converts a character offset inside plain text to byte offset inside a UTF-8 string.
fn char_offset_to_byte_offset(text: &str, char_offset: usize) -> Option<usize> {
    if char_offset == text.chars().count() {
        return Some(text.len());
    }

    text.char_indices().nth(char_offset).map(|(idx, _)| idx)
}

/// Maps a LanguageTool offset (relative to plain text) to an absolute byte offset
/// in the original source file.
///
/// Iterates through non-markup segments, accumulating their lengths until the
/// `lt_offset` falls within the current segment. Returns the segment's original
/// offset plus the offset within that segment.
///
/// # Arguments
/// * `text` - The `AnnotatedText` used in the LanguageTool request.
/// * `lt_offset` - Character offset relative to the concatenated plain text.
///
/// # Returns
/// `Some(absolute_offset)` if the offset maps to a valid non-markup segment,
/// `None` if the offset falls outside all segments or within markup.
pub fn map_lt_offset_to_absolute(text: &AnnotatedText, lt_offset: usize) -> Option<usize> {
    let mut current_lt_offset = 0;
    let mut last_segment_end = None;
    for segment in &text.segments {
        if segment.is_markup {
            continue;
        }
        let segment_char_len = segment.text.chars().count();
        let next_lt_offset = current_lt_offset + segment_char_len;
        if lt_offset >= current_lt_offset && lt_offset < next_lt_offset {
            let offset_in_segment = lt_offset - current_lt_offset;
            let byte_offset = char_offset_to_byte_offset(&segment.text, offset_in_segment)?;
            return Some(segment.offset + byte_offset);
        }
        if lt_offset == next_lt_offset {
            last_segment_end = Some(segment.offset + segment.text.len());
        }
        current_lt_offset = next_lt_offset;
    }

    if lt_offset == current_lt_offset {
        return last_segment_end;
    }

    None
}

/// Converts a byte offset in source content to an LSP `Position` (line, character).
///
/// Lines and characters are 0-indexed. Handles multi-byte UTF-8 characters correctly
/// by iterating with `char_indices`.
///
/// # Arguments
/// * `content` - The full source file content.
/// * `offset` - Byte offset within `content`. If `offset` exceeds content length,
///   returns the position at the end of the file.
///
/// # Returns
/// An LSP `Position` with `line` and `character` fields.
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

/// Holds all mutable state for the LSP server runtime.
///
/// Tracks open documents, their content, versions, languages, and cached errors.
/// Manages async task handles for debounced grammar checks and implements a simple
/// error cooldown to avoid spamming the LanguageTool server on repeated failures.
pub struct ServerState {
    /// HTTP client for LanguageTool API calls.
    pub client: LanguageToolClient,
    /// Maps document URIs to their latest known version numbers.
    pub document_versions: HashMap<String, i32>,
    /// Maps document URIs to their full text content.
    pub document_content: HashMap<String, String>,
    /// Maps document URIs to their LSP language IDs.
    pub document_languages: HashMap<String, String>,
    /// Maps document URIs to their latest raw grammar errors (pre-dictionary filter).
    pub document_errors: HashMap<String, Vec<GrammarError>>,
    /// Maps document URIs to their in-flight async check task handles.
    /// Used for cancellation when a document changes before the previous check completes.
    pub in_flight_tasks: HashMap<String, JoinHandle<()>>,
    /// Timestamp of the last LanguageTool error, used for cooldown tracking.
    pub last_error_time: Option<Instant>,
    /// Duration to wait after an error before retrying. Defaults to 60 seconds.
    pub error_cooldown: Duration,
    /// Root path of the workspace, used for dictionary loading and ignore actions.
    pub workspace_root: Option<PathBuf>,
    /// Whether this server instance attempted to auto-start a local LanguageTool container.
    pub started_lt: bool,
    /// Retained for config compatibility. Shared LanguageTool containers are not auto-stopped.
    pub stop_on_exit: bool,
    pub include_inline_comments: bool,
}

impl ServerState {
    /// Creates a new server state with the given LanguageTool client.
    ///
    /// All document maps are initialized empty. `error_cooldown` defaults to 60 seconds.
    ///
    /// # Arguments
    /// * `client` - The LanguageTool HTTP client to use for grammar checks.
    pub fn new(client: LanguageToolClient) -> Self {
        Self {
            client,
            document_versions: HashMap::new(),
            document_content: HashMap::new(),
            document_languages: HashMap::new(),
            document_errors: HashMap::new(),
            in_flight_tasks: HashMap::new(),
            last_error_time: None,
            error_cooldown: Duration::from_secs(60),
            workspace_root: None,
            started_lt: false,
            stop_on_exit: false,
            include_inline_comments: false,
        }
    }

    /// Records the current time as the last error timestamp.
    /// Triggers the cooldown period for subsequent check attempts.
    pub fn mark_error(&mut self) {
        self.last_error_time = Some(Instant::now());
    }

    /// Returns `true` if the server is currently in the error cooldown period.
    pub fn is_cooling_down(&self) -> bool {
        if let Some(last_error) = self.last_error_time {
            last_error.elapsed() < self.error_cooldown
        } else {
            false
        }
    }

    /// Updates the version number for a document.
    ///
    /// # Arguments
    /// * `uri` - Document URI string.
    /// * `version` - New version number from the LSP notification.
    pub fn update_version(&mut self, uri: String, version: i32) {
        self.document_versions.insert(uri, version);
    }

    /// Cancels any in-flight async task for the given document URI.
    ///
    /// # Arguments
    /// * `uri` - Document URI string. If no task exists for this URI, this is a no-op.
    pub fn cancel_task(&mut self, uri: &str) {
        if let Some(handle) = self.in_flight_tasks.remove(uri) {
            handle.abort();
        }
    }

    /// Registers a new async task for a document, cancelling any existing task first.
    ///
    /// # Arguments
    /// * `uri` - Document URI string.
    /// * `handle` - The `JoinHandle` of the spawned async task.
    pub fn register_task(&mut self, uri: String, handle: JoinHandle<()>) {
        self.cancel_task(&uri);
        self.in_flight_tasks.insert(uri, handle);
    }
}

/// Deserialized from `InitializeParams.initialization_options`.
///
/// Allows clients to configure the LanguageTool endpoint and parser behavior.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    /// URL of the LanguageTool server. Defaults to `http://localhost:8081` if not provided.
    pub endpoint: Option<String>,
    /// LanguageTool language code. Defaults to `en-US` if not provided.
    pub language: Option<String>,
    /// Retained for compatibility but ignored because LanguageTool containers are shared.
    pub stop_on_exit: Option<bool>,
    pub include_inline_comments: Option<bool>,
    pub disable_spell_check: Option<bool>,
}

const LT_DOCKER_IMAGE: &str = "ghcr.io/garrickwelsh/languagetool";
const LT_CONTAINER_NAME: &str = "docolint-lt-server";
const LT_CONTAINER_PORT: &str = "8081/tcp";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    fn command_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainerNetworkMode {
    Host,
    PublishedPort,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeContainerState {
    network_mode: String,
    port_binding: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

trait CommandRunner {
    fn run(&self, runtime: ContainerRuntime, args: &[&str]) -> io::Result<CommandOutput>;
}

struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, runtime: ContainerRuntime, args: &[&str]) -> io::Result<CommandOutput> {
        let output = ProcessCommand::new(runtime.command_name())
            .args(args)
            .output()?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

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
    let addrs = match (host, port).to_socket_addrs() {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(_) => return false,
    };

    addrs
        .iter()
        .any(|addr| std::net::TcpStream::connect_timeout(addr, Duration::from_secs(2)).is_ok())
}

fn show_info_message(sender: &crossbeam_channel::Sender<Message>, msg: &str) {
    let params = lsp_types::ShowMessageParams {
        typ: lsp_types::MessageType::INFO,
        message: msg.to_string(),
    };
    let not = Notification::new("window/showMessage".to_string(), params);
    let _ = sender.send(Message::Notification(not));
}

fn is_local_endpoint(endpoint: &str) -> bool {
    let url = match Url::parse(endpoint) {
        Ok(url) => url,
        Err(_) => return false,
    };

    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

fn is_docker_from_docker_mount(mountinfo: &str) -> bool {
    mountinfo
        .lines()
        .any(|line| line.contains("sock") && line.contains("docker"))
}

fn has_docker_from_docker_mount() -> bool {
    fs::read_to_string("/proc/self/mountinfo")
        .map(|mountinfo| is_docker_from_docker_mount(&mountinfo))
        .unwrap_or(false)
}

fn expected_network_mode() -> ContainerNetworkMode {
    if has_docker_from_docker_mount() {
        ContainerNetworkMode::Host
    } else {
        ContainerNetworkMode::PublishedPort
    }
}

fn runtime_is_usable(runner: &impl CommandRunner, runtime: ContainerRuntime) -> bool {
    matches!(runner.run(runtime, &["--version"]), Ok(output) if output.success)
        && matches!(runner.run(runtime, &["ps"]), Ok(output) if output.success)
}

fn inspect_container_state(
    runner: &impl CommandRunner,
    runtime: ContainerRuntime,
) -> Option<RuntimeContainerState> {
    let inspect = runner
        .run(
            runtime,
            &[
                "inspect",
                "--format",
                "{{.HostConfig.NetworkMode}}",
                LT_CONTAINER_NAME,
            ],
        )
        .ok()?;
    if !inspect.success {
        return None;
    }

    let port_binding = runner
        .run(runtime, &["port", LT_CONTAINER_NAME, LT_CONTAINER_PORT])
        .ok()
        .filter(|output| output.success)
        .map(|output| output.stdout);

    Some(RuntimeContainerState {
        network_mode: inspect.stdout,
        port_binding,
    })
}

fn published_port_matches(binding: Option<&str>) -> bool {
    binding
        .map(|binding| {
            binding
                .lines()
                .any(|line| line.trim() == "8081" || line.contains(":8081"))
        })
        .unwrap_or(false)
}

fn container_matches_expected(
    state: &RuntimeContainerState,
    expected_mode: ContainerNetworkMode,
) -> bool {
    match expected_mode {
        ContainerNetworkMode::Host => state.network_mode == "host",
        ContainerNetworkMode::PublishedPort => {
            state.network_mode != "host" && published_port_matches(state.port_binding.as_deref())
        }
    }
}

fn pull_language_tool_image(
    runner: &impl CommandRunner,
    runtime: ContainerRuntime,
    sender: &crossbeam_channel::Sender<Message>,
) -> bool {
    show_info_message(sender, "Pulling LanguageTool image...");
    let result = runner.run(runtime, &["pull", "-q", LT_DOCKER_IMAGE]);
    if matches!(result, Ok(output) if output.success) {
        show_info_message(sender, "LanguageTool image ready.");
        true
    } else {
        false
    }
}

fn start_existing_container(
    runner: &impl CommandRunner,
    runtime: ContainerRuntime,
    sender: &crossbeam_channel::Sender<Message>,
) -> bool {
    show_info_message(sender, "Starting LanguageTool container...");
    matches!(runner.run(runtime, &["start", LT_CONTAINER_NAME]), Ok(output) if output.success)
}

fn run_new_container(
    runner: &impl CommandRunner,
    runtime: ContainerRuntime,
    mode: ContainerNetworkMode,
    sender: &crossbeam_channel::Sender<Message>,
) -> bool {
    show_info_message(sender, "Starting LanguageTool container...");
    let args = match mode {
        ContainerNetworkMode::Host => vec![
            "run",
            "-d",
            "--network",
            "host",
            "--name",
            LT_CONTAINER_NAME,
            LT_DOCKER_IMAGE,
        ],
        ContainerNetworkMode::PublishedPort => vec![
            "run",
            "-d",
            "-p",
            "8081:8081",
            "--name",
            LT_CONTAINER_NAME,
            LT_DOCKER_IMAGE,
        ],
    };

    matches!(runner.run(runtime, &args), Ok(output) if output.success)
}

fn remove_existing_container(runner: &impl CommandRunner, runtime: ContainerRuntime) -> bool {
    matches!(runner.run(runtime, &["rm", "-f", LT_CONTAINER_NAME]), Ok(output) if output.success)
}

fn wait_for_language_tool_ready(
    endpoint: &str,
    sender: &crossbeam_channel::Sender<Message>,
    probe: &impl Fn(&str) -> bool,
) -> bool {
    show_info_message(sender, "Waiting for LanguageTool...");
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(250));
        if probe(endpoint) {
            show_info_message(sender, "LanguageTool ready.");
            return true;
        }
    }

    false
}

fn try_start_language_tool_with_runtime(
    endpoint: &str,
    sender: &crossbeam_channel::Sender<Message>,
    runner: &impl CommandRunner,
    runtime: ContainerRuntime,
    mode: ContainerNetworkMode,
    probe: &impl Fn(&str) -> bool,
) -> bool {
    match inspect_container_state(runner, runtime) {
        Some(state) if container_matches_expected(&state, mode) => {
            start_existing_container(runner, runtime, sender)
                && wait_for_language_tool_ready(endpoint, sender, probe)
        }
        Some(_) => {
            if !pull_language_tool_image(runner, runtime, sender) {
                return false;
            }
            if !remove_existing_container(runner, runtime) {
                return false;
            }
            run_new_container(runner, runtime, mode, sender)
                && wait_for_language_tool_ready(endpoint, sender, probe)
        }
        None => {
            if !pull_language_tool_image(runner, runtime, sender) {
                return false;
            }
            run_new_container(runner, runtime, mode, sender)
                && wait_for_language_tool_ready(endpoint, sender, probe)
        }
    }
}

fn ensure_language_tool_running_with(
    endpoint: &str,
    sender: &crossbeam_channel::Sender<Message>,
    runner: &impl CommandRunner,
    mode: ContainerNetworkMode,
    probe: &impl Fn(&str) -> bool,
) -> bool {
    if !is_local_endpoint(endpoint) {
        return false;
    }

    for runtime in [ContainerRuntime::Docker, ContainerRuntime::Podman] {
        if !runtime_is_usable(runner, runtime) {
            continue;
        }

        if try_start_language_tool_with_runtime(endpoint, sender, runner, runtime, mode, probe) {
            return true;
        }
    }

    false
}

fn ensure_language_tool_running(
    endpoint: &str,
    sender: &crossbeam_channel::Sender<Message>,
) -> bool {
    let runner = SystemCommandRunner;
    ensure_language_tool_running_with(
        endpoint,
        sender,
        &runner,
        expected_network_mode(),
        &probe_language_tool,
    )
}

fn recover_language_tool(endpoint: &str, sender: &crossbeam_channel::Sender<Message>) -> bool {
    if probe_language_tool(endpoint) {
        return true;
    }

    ensure_language_tool_running(endpoint, sender)
}

fn load_dictionary(state: &ServerState, uri: &str) -> Dictionary {
    let root = match &state.workspace_root {
        Some(r) => r,
        None => return Dictionary::new(),
    };
    let u = match Url::parse(uri) {
        Ok(u) => u,
        Err(_) => return Dictionary::new(),
    };
    let path = match u.to_file_path() {
        Ok(p) => p,
        Err(_) => return Dictionary::new(),
    };
    Dictionary::load(root, &path)
}

fn publish_diagnostics(
    state: &ServerState,
    sender: &crossbeam_channel::Sender<Message>,
    uri: &str,
    version: i32,
    content: &str,
    annotated: &AnnotatedText,
) {
    let raw_errors = match state.document_errors.get(uri) {
        Some(e) => e.clone(),
        None => return,
    };

    let dict = load_dictionary(state, uri);
    let filtered = dict.filter_errors(&annotated.plain_text(), raw_errors);
    let mut diagnostics = Vec::new();

    for err in filtered {
        if let Some(abs_offset) = map_lt_offset_to_absolute(annotated, err.offset) {
            let start = offset_to_position(content, abs_offset);
            let end_abs_offset =
                map_lt_offset_to_absolute(annotated, err.offset + err.length).unwrap_or(abs_offset);
            let end = offset_to_position(content, end_abs_offset);

            diagnostics.push(Diagnostic {
                range: Range { start, end },
                severity: Some(lsp_types::DiagnosticSeverity::INFORMATION),
                code: Some(lsp_types::NumberOrString::String(err.rule_id.clone())),
                source: Some("docolint".to_string()),
                message: err.message,
                data: Some(serde_json::json!({
                    "rule_id": err.rule_id,
                    "replacements": err.replacements
                })),
                ..Default::default()
            });
        }
    }

    let uri_lsp: lsp_types::Uri =
        serde_json::from_value(serde_json::to_value(uri).unwrap()).unwrap();
    let params = PublishDiagnosticsParams {
        uri: uri_lsp,
        diagnostics,
        version: Some(version),
    };

    let not = Notification::new("textDocument/publishDiagnostics".to_string(), params);
    let _ = sender.send(Message::Notification(not));
}

fn recheck_document(
    state: Arc<RwLock<ServerState>>,
    sender: crossbeam_channel::Sender<Message>,
    uri: String,
) {
    let (content, annotated, version) = {
        let s = state.read().unwrap();
        let content = match s.document_content.get(&uri) {
            Some(c) => c.clone(),
            None => return,
        };
        let lang = s
            .document_languages
            .get(&uri)
            .cloned()
            .unwrap_or_else(|| "plain".to_string());
        let version = s.document_versions.get(&uri).copied().unwrap_or(1);
        let config = ParserConfig {
            include_inline_comments: s.include_inline_comments,
        };
        let annotated = parse_document(&lang, &content, &config);
        (content, annotated, version)
    };

    publish_diagnostics(
        &state.read().unwrap(),
        &sender,
        &uri,
        version,
        &content,
        &annotated,
    );
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
    let sender_task = sender.clone();
    let handle = tokio::spawn(async move {
        // Debounce
        tokio::time::sleep(Duration::from_millis(500)).await;

        let (client, is_cooling) = {
            let s = state_task.read().unwrap();
            (s.client.clone(), s.is_cooling_down())
        };

        if is_cooling {
            return;
        }

        let config = {
            let s = state_task.read().unwrap();
            ParserConfig {
                include_inline_comments: s.include_inline_comments,
            }
        };
        let annotated = parse_document(&lang, &content, &config);
        let mut result = client.check(annotated.clone()).await;
        if result.is_err() && recover_language_tool(client.base_url(), &sender_task) {
            result = client.check(annotated.clone()).await;
        }

        match result {
            Ok(errors) => {
                state_task
                    .write()
                    .unwrap()
                    .document_errors
                    .insert(uri_task.clone(), errors.clone());

                publish_diagnostics(
                    &state_task.read().unwrap(),
                    &sender_task,
                    &uri_task,
                    version,
                    &content,
                    &annotated,
                );
            }
            Err(_) => {
                state_task.write().unwrap().mark_error();
            }
        }
    });

    state.write().unwrap().register_task(uri, handle);
}

/// Handles `docolint.ignoreWord` command payloads.
///
/// Writes selected word into target ignore file, then triggers recheck for
/// originating document when URI is present.
fn handle_ignore_word_command(
    state: Arc<RwLock<ServerState>>,
    sender: crossbeam_channel::Sender<Message>,
    cmd_params: lsp_types::ExecuteCommandParams,
) {
    if cmd_params.command != "docolint.ignoreWord" {
        return;
    }

    let args = &cmd_params.arguments;
    if args.len() < 3 {
        return;
    }

    let word = args[0].as_str().unwrap_or("").to_string();
    let ignore_path = args[1].as_str().unwrap_or("").to_string();
    let uri = args[2].as_str().unwrap_or("").to_string();

    if ignore_path.is_empty() || word.is_empty() {
        return;
    }

    let mut dict = Dictionary::new();
    let _ = dict.add_word(&word, Path::new(&ignore_path));

    if !uri.is_empty() {
        recheck_document(state, sender, uri);
    }
}

/// Handles `workspace/executeCommand` requests.
///
/// Returns `true` when request method matches, even if payload decoding fails.
/// Successful decoding delegates to `handle_ignore_word_command` and sends null response.
fn handle_execute_command_request(
    state: Arc<RwLock<ServerState>>,
    sender: &crossbeam_channel::Sender<Message>,
    req: &lsp_server::Request,
) -> bool {
    if req.method != "workspace/executeCommand" {
        return false;
    }

    if let Ok(cmd_params) =
        serde_json::from_value::<lsp_types::ExecuteCommandParams>(req.params.clone())
    {
        handle_ignore_word_command(state, sender.clone(), cmd_params);
        let resp = Response::new_ok(req.id.clone(), serde_json::Value::Null);
        let _ = sender.send(Message::Response(resp));
    }

    true
}

/// Builds code actions for diagnostics attached to single document.
///
/// Combines ignore-word actions derived from selected text range with
/// replacement actions derived from diagnostic metadata.
fn collect_code_actions(
    state: &Arc<RwLock<ServerState>>,
    params: CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let uri = params.text_document.uri.clone();
    let root = state
        .read()
        .unwrap()
        .workspace_root
        .clone()
        .unwrap_or_default();

    let mut actions = Vec::new();
    let uri_val = serde_json::to_value(&uri).unwrap();
    if let Ok(url) = serde_json::from_value::<Url>(uri_val) {
        let path = url.to_file_path().unwrap_or_default();
        let uri_str = uri.to_string();

        for diag in params.context.diagnostics {
            let content = state
                .read()
                .unwrap()
                .document_content
                .get(&uri_str)
                .cloned();
            if let Some(content) = content {
                let start_offset = position_to_offset(&content, diag.range.start);
                let end_offset = position_to_offset(&content, diag.range.end);
                if start_offset < end_offset && end_offset <= content.len() {
                    let word = &content[start_offset..end_offset];
                    actions.extend(generate_ignore_actions(&root, &path, word, &uri_str));
                }
                actions.extend(generate_replacement_actions(&diag, &uri, &content));
            }
        }
    }

    actions
}

/// Handles `textDocument/codeAction` requests.
///
/// Returns `true` when request method matches. Matching requests are decoded,
/// converted into actions, then answered over LSP sender.
fn handle_code_action_request(
    state: Arc<RwLock<ServerState>>,
    sender: &crossbeam_channel::Sender<Message>,
    req: lsp_server::Request,
) -> bool {
    if req.method != "textDocument/codeAction" {
        return false;
    }

    if let Ok(params) = serde_json::from_value::<CodeActionParams>(req.params) {
        let result = serde_json::to_value(collect_code_actions(&state, params)).unwrap();
        let resp = Response::new_ok(req.id, result);
        let _ = sender.send(Message::Response(resp));
    }

    true
}

/// Handles `textDocument/didOpen` notifications.
///
/// Stores current content, version, and language in server state, then starts
/// debounced grammar check for opened document.
fn handle_did_open(
    state: Arc<RwLock<ServerState>>,
    sender: crossbeam_channel::Sender<Message>,
    params: DidOpenTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();
    let version = params.text_document.version;
    let content = params.text_document.text;
    let lang = params.text_document.language_id;

    {
        let mut state_w = state.write().unwrap();
        state_w.update_version(uri.clone(), version);
        state_w
            .document_content
            .insert(uri.clone(), content.clone());
        state_w.document_languages.insert(uri.clone(), lang.clone());
    }

    spawn_check(state, sender, uri, version, content, lang);
}

/// Handles `textDocument/didChange` notifications.
///
/// Applies full-content replacement, updates tracked version, reuses known
/// language, then starts fresh debounced grammar check.
fn handle_did_change(
    state: Arc<RwLock<ServerState>>,
    sender: crossbeam_channel::Sender<Message>,
    params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();
    let version = params.text_document.version;
    if let Some(change) = params.content_changes.into_iter().next() {
        let content = change.text;

        let lang = {
            let mut state_w = state.write().unwrap();
            state_w.update_version(uri.clone(), version);
            state_w
                .document_content
                .insert(uri.clone(), content.clone());
            state_w
                .document_languages
                .get(&uri)
                .cloned()
                .unwrap_or_else(|| "plain".to_string())
        };

        spawn_check(state, sender, uri, version, content, lang);
    }
}

/// Handles `textDocument/didClose` notifications.
///
/// Cancels in-flight work and removes all document-specific state tracked for URI.
fn handle_did_close(state: Arc<RwLock<ServerState>>, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri.to_string();
    let mut state_w = state.write().unwrap();
    state_w.cancel_task(&uri);
    state_w.document_versions.remove(&uri);
    state_w.document_content.remove(&uri);
    state_w.document_languages.remove(&uri);
    state_w.document_errors.remove(&uri);
}

/// Routes LSP notifications to document lifecycle handlers.
///
/// Unknown notifications are ignored.
fn handle_notification(
    state: Arc<RwLock<ServerState>>,
    sender: &crossbeam_channel::Sender<Message>,
    not: Notification,
) {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                handle_did_open(state, sender.clone(), params);
            }
        }
        "textDocument/didChange" => {
            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(not.params) {
                handle_did_change(state, sender.clone(), params);
            }
        }
        "textDocument/didClose" => {
            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(not.params) {
                handle_did_close(state, params);
            }
        }
        _ => {}
    }
}

/// Stops auto-started LanguageTool container when shutdown policy requires it.
fn stop_language_tool_if_needed(state: &Arc<RwLock<ServerState>>) {
    if state.read().unwrap().stop_on_exit {
        // Auto-started LanguageTool acts as shared infra across docolint instances.
    }
}

/// Routes LSP requests handled inside main server loop.
///
/// Shutdown requests terminate loop via `Err(())`. All other known request types
/// are handled in-place and return `Ok(())`.
fn handle_request(
    connection: &Connection,
    state: Arc<RwLock<ServerState>>,
    req: lsp_server::Request,
) -> Result<(), ()> {
    if let Ok(true) = connection.handle_shutdown(&req) {
        stop_language_tool_if_needed(&state);
        return Err(());
    }

    if handle_execute_command_request(state.clone(), &connection.sender, &req) {
        return Ok(());
    }

    let _ = handle_code_action_request(state, &connection.sender, req);
    Ok(())
}

/// Main server entry point. Runs the LSP event loop on the given connection.
///
/// Initializes server state, probes for LanguageTool availability (auto-starts local
/// Docker/Podman container when appropriate), then blocks on the connection receiver
/// processing LSP messages:
/// - `textDocument/didOpen`, `didChange`: spawns debounced async grammar checks
/// - `textDocument/didClose`: cancels pending tasks and clears document state
/// - `textDocument/codeAction`: generates replacement and ignore-word quick fixes
/// - `workspace/executeCommand`: handles `docolint.ignoreWord` to add words to `.docolint-ignore`
///
/// # Arguments
/// * `connection` - The LSP stdio connection from the editor.
/// * `params` - Initialization parameters from the editor, including optional
///   `InitializationOptions` for endpoint and container lifecycle config.
///
/// # Errors
/// Returns an error if the connection fails or the message loop encounters an
/// unrecoverable issue.
pub async fn run(
    connection: Connection,
    params: InitializeParams,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Decode init options once so downstream setup reads concrete values.
    let options: InitializationOptions = params
        .initialization_options
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(InitializationOptions {
            endpoint: None,
            language: None,
            stop_on_exit: None,
            include_inline_comments: None,
            disable_spell_check: None,
        });

    let endpoint = options
        .endpoint
        .unwrap_or_else(|| "http://localhost:8081".to_string());
    let language = options.language.unwrap_or_else(|| "en-US".to_string());
    let stop_on_exit = options.stop_on_exit.unwrap_or(false);
    let include_inline_comments = options.include_inline_comments.unwrap_or(false);
    let disable_spell_check = options.disable_spell_check.unwrap_or(false);

    // Build client and mutable server state shared by request/notification handlers.
    let client = LanguageToolClient::new(ClientConfig {
        base_url: endpoint.clone(),
        language,
        disable_spell_check,
    });

    let mut state_raw = ServerState::new(client);
    state_raw.workspace_root = params.workspace_folders.and_then(|folders| {
        let first = folders.first()?;
        let url: Url = serde_json::from_value(serde_json::to_value(&first.uri).ok()?).ok()?;
        url.to_file_path().ok()
    });
    state_raw.stop_on_exit = stop_on_exit;
    state_raw.include_inline_comments = include_inline_comments;
    let state = Arc::new(RwLock::new(state_raw));

    // Ensure local/default LanguageTool is reachable before entering blocking LSP receive loop.
    if !probe_language_tool(&endpoint)
        && ensure_language_tool_running(&endpoint, &connection.sender)
    {
        state.write().unwrap().started_lt = true;
    }

    let state_for_loop = state.clone();
    tokio::task::spawn_blocking(move || {
        // Main LSP loop: receive one message at time, dispatch by message type.
        while let Ok(msg) = connection.receiver.recv() {
            match msg {
                Message::Request(req) => {
                    // Request path: shutdown, executeCommand, codeAction.
                    if handle_request(&connection, state_for_loop.clone(), req).is_err() {
                        return Ok(());
                    }
                }
                Message::Notification(not) => {
                    // Notification path: didOpen, didChange, didClose.
                    handle_notification(state_for_loop.clone(), &connection.sender, not)
                }
                // LSP responses are client-side in this server, so ignore them.
                Message::Response(_) => {}
            }
        }
        Ok(())
    })
    .await?
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
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeCommandRunner {
        responses: HashMap<String, Result<CommandOutput, String>>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeCommandRunner {
        fn with_response(
            mut self,
            runtime: ContainerRuntime,
            args: &[&str],
            success: bool,
            stdout: &str,
        ) -> Self {
            self.responses.insert(
                Self::key(runtime, args),
                Ok(CommandOutput {
                    success,
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                }),
            );
            self
        }

        fn with_error(mut self, runtime: ContainerRuntime, args: &[&str]) -> Self {
            self.responses
                .insert(Self::key(runtime, args), Err("command failed".to_string()));
            self
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        fn key(runtime: ContainerRuntime, args: &[&str]) -> String {
            format!("{} {}", runtime.command_name(), args.join(" "))
        }
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(&self, runtime: ContainerRuntime, args: &[&str]) -> io::Result<CommandOutput> {
            let key = Self::key(runtime, args);
            self.calls.lock().unwrap().push(key.clone());
            self.responses
                .get(&key)
                .cloned()
                .unwrap_or_else(|| Err(key.clone()))
                .map_err(io::Error::other)
        }
    }

    #[tokio::test]
    async fn test_initialization_options_extraction() {
        let (server_conn, client_conn) = Connection::memory();
        let params = InitializeParams {
            initialization_options: Some(json!({
                "endpoint": "http://custom-lt:8081",
                "language": "en-AU",
                "disableSpellCheck": true
            })),
            ..Default::default()
        };

        let server_handle = tokio::spawn(async move { run(server_conn, params).await });

        drop(client_conn);
        let result = server_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_local_endpoint() {
        assert!(is_local_endpoint("http://localhost:8081"));
        assert!(is_local_endpoint("http://127.0.0.1:8081"));
        assert!(!is_local_endpoint("http://lt.internal:8081"));
    }

    #[test]
    fn test_docker_from_docker_mount_detection() {
        assert!(is_docker_from_docker_mount(
            "123 456 0:42 / /var/run/docker.sock rw,nosuid - tmpfs tmpfs rw"
        ));
        assert!(!is_docker_from_docker_mount(
            "123 456 0:42 / /var/run/podman.sock rw,nosuid - tmpfs tmpfs rw"
        ));
    }

    #[test]
    fn test_probe_language_tool_resolves_localhost() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(probe_language_tool(&format!("http://localhost:{port}")));
    }

    #[test]
    fn test_ensure_language_tool_uses_podman_when_docker_unusable() {
        let runner = FakeCommandRunner::default()
            .with_response(
                ContainerRuntime::Docker,
                &["--version"],
                true,
                "Docker version",
            )
            .with_response(ContainerRuntime::Docker, &["ps"], false, "")
            .with_response(
                ContainerRuntime::Podman,
                &["--version"],
                true,
                "podman version",
            )
            .with_response(ContainerRuntime::Podman, &["ps"], true, "")
            .with_error(
                ContainerRuntime::Podman,
                &[
                    "inspect",
                    "--format",
                    "{{.HostConfig.NetworkMode}}",
                    LT_CONTAINER_NAME,
                ],
            )
            .with_response(
                ContainerRuntime::Podman,
                &["pull", "-q", LT_DOCKER_IMAGE],
                true,
                "",
            )
            .with_response(
                ContainerRuntime::Podman,
                &[
                    "run",
                    "-d",
                    "-p",
                    "8081:8081",
                    "--name",
                    LT_CONTAINER_NAME,
                    LT_DOCKER_IMAGE,
                ],
                true,
                "",
            );
        let (sender, _receiver) = crossbeam_channel::unbounded();

        let ready = ensure_language_tool_running_with(
            "http://localhost:8081",
            &sender,
            &runner,
            ContainerNetworkMode::PublishedPort,
            &|_| true,
        );

        assert!(ready);
        assert_eq!(
            runner.calls(),
            vec![
                "docker --version",
                "docker ps",
                "podman --version",
                "podman ps",
                "podman inspect --format {{.HostConfig.NetworkMode}} docolint-lt-server",
                "podman pull -q ghcr.io/garrickwelsh/languagetool",
                "podman run -d -p 8081:8081 --name docolint-lt-server ghcr.io/garrickwelsh/languagetool",
            ]
        );
    }

    #[test]
    fn test_wrong_network_container_is_recreated_after_pull() {
        let runner = FakeCommandRunner::default()
            .with_response(
                ContainerRuntime::Docker,
                &["--version"],
                true,
                "Docker version",
            )
            .with_response(ContainerRuntime::Docker, &["ps"], true, "")
            .with_response(
                ContainerRuntime::Docker,
                &[
                    "inspect",
                    "--format",
                    "{{.HostConfig.NetworkMode}}",
                    LT_CONTAINER_NAME,
                ],
                true,
                "bridge",
            )
            .with_response(
                ContainerRuntime::Docker,
                &["port", LT_CONTAINER_NAME, LT_CONTAINER_PORT],
                true,
                "",
            )
            .with_response(
                ContainerRuntime::Docker,
                &["pull", "-q", LT_DOCKER_IMAGE],
                true,
                "",
            )
            .with_response(
                ContainerRuntime::Docker,
                &["rm", "-f", LT_CONTAINER_NAME],
                true,
                "",
            )
            .with_response(
                ContainerRuntime::Docker,
                &[
                    "run",
                    "-d",
                    "--network",
                    "host",
                    "--name",
                    LT_CONTAINER_NAME,
                    LT_DOCKER_IMAGE,
                ],
                true,
                "",
            );
        let (sender, _receiver) = crossbeam_channel::unbounded();

        let ready = ensure_language_tool_running_with(
            "http://localhost:8081",
            &sender,
            &runner,
            ContainerNetworkMode::Host,
            &|_| true,
        );

        assert!(ready);
        assert_eq!(
            runner.calls(),
            vec![
                "docker --version",
                "docker ps",
                "docker inspect --format {{.HostConfig.NetworkMode}} docolint-lt-server",
                "docker port docolint-lt-server 8081/tcp",
                "docker pull -q ghcr.io/garrickwelsh/languagetool",
                "docker rm -f docolint-lt-server",
                "docker run -d --network host --name docolint-lt-server ghcr.io/garrickwelsh/languagetool",
            ]
        );
    }

    #[test]
    fn test_offset_mapping() {
        use docolint_types::TextSegment;
        let text = AnnotatedText {
            segments: vec![
                TextSegment {
                    text: "Hello ".to_string(),
                    is_markup: false,
                    offset: 0,
                },
                TextSegment {
                    text: "<b>".to_string(),
                    is_markup: true,
                    offset: 6,
                },
                TextSegment {
                    text: "world".to_string(),
                    is_markup: false,
                    offset: 9,
                },
            ],
        };

        assert_eq!(map_lt_offset_to_absolute(&text, 0), Some(0)); // 'H'
        assert_eq!(map_lt_offset_to_absolute(&text, 6), Some(9)); // 'w'
        assert_eq!(map_lt_offset_to_absolute(&text, 10), Some(13)); // 'd'
        assert_eq!(map_lt_offset_to_absolute(&text, 11), Some(14));
    }

    #[test]
    fn test_offset_mapping_handles_unicode() {
        use docolint_types::TextSegment;
        let text = AnnotatedText {
            segments: vec![TextSegment {
                text: "alpha ❌ beta".to_string(),
                is_markup: false,
                offset: 0,
            }],
        };

        assert_eq!(map_lt_offset_to_absolute(&text, 6), Some(6));
        assert_eq!(map_lt_offset_to_absolute(&text, 7), Some(9));
    }

    #[test]
    fn test_offset_to_position() {
        let content = "line1\nline2";
        assert_eq!(
            offset_to_position(content, 0),
            lsp_types::Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            offset_to_position(content, 5),
            lsp_types::Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(
            offset_to_position(content, 6),
            lsp_types::Position {
                line: 1,
                character: 0
            }
        );
    }

    #[test]
    fn test_circuit_breaker() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
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
        use docolint_dictionary::Dictionary;

        let content = "/// This is a testt.";

        let annotated = parse_document("rust", content, &ParserConfig::default());
        assert_eq!(annotated.plain_text().trim(), "This is a testt.");

        let errors = vec![docolint_types::GrammarError {
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
        let uri = "file:///workspaces/project/src/module/file.rs";

        let actions = generate_ignore_actions(root, &doc, "typo", uri);

        assert_eq!(actions.len(), 3);

        if let CodeActionOrCommand::CodeAction(a) = &actions[0] {
            assert!(a.title.contains("module"));
            assert!(a.command.is_some());
            let cmd = a.command.as_ref().unwrap();
            assert_eq!(cmd.command, "docolint.ignoreWord");
            let args = cmd.arguments.as_ref().unwrap();
            assert_eq!(args.len(), 3);
            assert_eq!(args[0].as_str().unwrap(), "typo");
            assert!(args[1].as_str().unwrap().ends_with(".docolint-ignore"));
            assert_eq!(args[2].as_str().unwrap(), uri);
        }
        if let CodeActionOrCommand::CodeAction(a) = &actions[2] {
            assert!(a.title.contains("workspace root"));
        }
    }

    #[test]
    fn test_replacement_code_actions() {
        let uri: lsp_types::Uri = serde_json::from_str("\"file:///test.rs\"").unwrap();
        let diag = Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 10,
                },
                end: Position {
                    line: 0,
                    character: 15,
                },
            },
            severity: Some(lsp_types::DiagnosticSeverity::INFORMATION),
            code: Some(lsp_types::NumberOrString::String("SPELLING".to_string())),
            source: Some("docolint".to_string()),
            message: "Spelling error".to_string(),
            data: Some(json!({
                "rule_id": "SPELLING",
                "replacements": ["test", "testing"]
            })),
            ..Default::default()
        };

        let content = "/// This is a testt.";
        let actions = generate_replacement_actions(&diag, &uri, content);

        assert_eq!(actions.len(), 2);

        if let CodeActionOrCommand::CodeAction(a) = &actions[0] {
            assert!(a.title.contains("Replace with 'test'"));
            assert!(a.is_preferred == Some(true));
            let edit = a.edit.as_ref().unwrap();
            if let Some(lsp_types::DocumentChanges::Edits(edits)) = &edit.document_changes {
                assert_eq!(edits.len(), 1);
                let text_edit = &edits[0].edits[0];
                if let OneOf::Left(edit) = text_edit {
                    assert_eq!(edit.new_text, "test");
                } else {
                    panic!("Expected TextEdit");
                }
            } else {
                panic!("Expected document_changes with edits");
            }
        }

        if let CodeActionOrCommand::CodeAction(a) = &actions[1] {
            assert!(a.title.contains("Replace with 'testing'"));
            assert!(a.is_preferred.is_none());
        }
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
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
    fn test_handle_ignore_word_command_rechecks_document() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
        });
        let mut state_raw = ServerState::new(client);
        let uri = "file:///tmp/test.rs".to_string();
        state_raw
            .document_content
            .insert(uri.clone(), "/// testt".to_string());
        state_raw
            .document_languages
            .insert(uri.clone(), "rust".to_string());
        state_raw.document_versions.insert(uri.clone(), 1);
        state_raw.document_errors.insert(
            uri.clone(),
            vec![GrammarError {
                message: "Spelling".to_string(),
                offset: 0,
                length: 5,
                replacements: vec!["test".to_string()],
                rule_id: "SPELLING".to_string(),
            }],
        );

        let state = Arc::new(RwLock::new(state_raw));
        let (sender, receiver) = crossbeam_channel::unbounded();
        let ignore_file = std::env::temp_dir().join("docolint-ignore-command-test.txt");
        let _ = std::fs::remove_file(&ignore_file);

        handle_ignore_word_command(
            state,
            sender,
            lsp_types::ExecuteCommandParams {
                command: "docolint.ignoreWord".to_string(),
                arguments: vec![
                    json!("testt"),
                    json!(ignore_file.to_string_lossy().to_string()),
                    json!(uri),
                ],
                work_done_progress_params: Default::default(),
            },
        );

        match receiver.recv().unwrap() {
            Message::Notification(not) => {
                assert_eq!(not.method, "textDocument/publishDiagnostics");
            }
            _ => panic!("expected publishDiagnostics notification"),
        }
    }

    #[test]
    fn test_collect_code_actions_includes_ignore_and_replace() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
        });
        let mut state_raw = ServerState::new(client);
        state_raw.workspace_root = Some(PathBuf::from("/workspaces/project"));
        let uri: lsp_types::Uri =
            serde_json::from_str("\"file:///workspaces/project/src/file.rs\"").unwrap();
        state_raw
            .document_content
            .insert(uri.to_string(), "testt".to_string());
        let state = Arc::new(RwLock::new(state_raw));

        let diag = Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 5,
                },
            },
            data: Some(json!({ "replacements": ["test"] })),
            ..Default::default()
        };

        let actions = collect_code_actions(
            &state,
            CodeActionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri },
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                context: lsp_types::CodeActionContext {
                    diagnostics: vec![diag],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
        );

        assert!(actions.iter().any(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title.contains("Ignore 'testt'"),
            _ => false,
        }));
        assert!(actions.iter().any(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title.contains("Replace with 'test'"),
            _ => false,
        }));
    }

    #[tokio::test]
    async fn test_handle_did_open_stores_document_state() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
        });
        let state = Arc::new(RwLock::new(ServerState::new(client)));
        let (sender, _receiver) = crossbeam_channel::unbounded();

        handle_did_open(
            state.clone(),
            sender,
            DidOpenTextDocumentParams {
                text_document: lsp_types::TextDocumentItem {
                    uri: serde_json::from_str("\"file:///test.rs\"").unwrap(),
                    language_id: "rust".to_string(),
                    version: 3,
                    text: "/// hello".to_string(),
                },
            },
        );

        let state_r = state.read().unwrap();
        assert_eq!(state_r.document_versions.get("file:///test.rs"), Some(&3));
        assert_eq!(
            state_r
                .document_languages
                .get("file:///test.rs")
                .map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            state_r
                .document_content
                .get("file:///test.rs")
                .map(String::as_str),
            Some("/// hello")
        );
        assert!(state_r.in_flight_tasks.contains_key("file:///test.rs"));
    }

    #[tokio::test]
    async fn test_handle_did_close_clears_document_state() {
        let client = LanguageToolClient::new(ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
        });
        let mut state_raw = ServerState::new(client);
        let uri = "file:///test.rs".to_string();
        state_raw.document_versions.insert(uri.clone(), 1);
        state_raw
            .document_languages
            .insert(uri.clone(), "rust".to_string());
        state_raw
            .document_content
            .insert(uri.clone(), "/// hello".to_string());
        state_raw.document_errors.insert(uri.clone(), vec![]);
        state_raw.register_task(
            uri.clone(),
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }),
        );
        let state = Arc::new(RwLock::new(state_raw));

        handle_did_close(
            state.clone(),
            DidCloseTextDocumentParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: serde_json::from_str("\"file:///test.rs\"").unwrap(),
                },
            },
        );

        let state_r = state.read().unwrap();
        assert!(!state_r.document_versions.contains_key(&uri));
        assert!(!state_r.document_languages.contains_key(&uri));
        assert!(!state_r.document_content.contains_key(&uri));
        assert!(!state_r.document_errors.contains_key(&uri));
        assert!(!state_r.in_flight_tasks.contains_key(&uri));
    }

    #[test]
    fn test_position_to_offset() {
        let content = "a\nbc";
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 0,
                    character: 0
                }
            ),
            0
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 0,
                    character: 1
                }
            ),
            1
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 1,
                    character: 0
                }
            ),
            2
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 1,
                    character: 1
                }
            ),
            3
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 1,
                    character: 2
                }
            ),
            4
        );
    }
}
