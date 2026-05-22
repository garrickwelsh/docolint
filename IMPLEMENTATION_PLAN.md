# Implementation Plan: ltlsp

This project will be developed using a strict **Test-Driven Development (TDD)** approach:
`Red (Failing Test) -> Green (Minimal Implementation) -> Refactor`.

Development will be organized by module to support isolated subagent execution.

## Phase 0: Environment Setup
Focus: Ensuring the development environment and network boundaries are ready.

- [ ] **Network Strategy**: Update `.devcontainer/devcontainer.json` to use `--network host` in `runArgs`.
- [ ] **LT Server Setup**: Add `just` recipes to pull and run the LanguageTool server (`ghcr.io/garrickwelsh/languagetool`) using `--network host` so it binds to the shared `localhost`.
- [ ] **Project Init**: Initialize Rust workspace and add core dependencies (`lsp-server`, `tree-sitter`, statically linked tree-sitter grammars (`rust`, `markdown`, `html`, `json`, `csharp`), `reqwest`, `tokio`, `wiremock` for dev).

## Phase 1: `client` Module (LanguageTool HTTP Client)
Focus: Communicating with the LanguageTool server using batched `AnnotatedText`.
Public Interface: `pub struct LanguageToolClient`, `pub async fn check(&self, text: AnnotatedText) -> Result<Vec<GrammarError>>`

- [ ] **TDD Cycle 1**: Test client initialization with configurable `base_url` (supporting both localhost and remote cloud servers).
- [ ] **TDD Cycle 2**: Test sending a simple string to a `wiremock` mock LT server and receiving a JSON response.
- [ ] **TDD Cycle 3**: Test the mapping of LT `match` objects to internal `GrammarError` models.
- [ ] **TDD Cycle 4**: Test compiling an `AnnotatedText` request (mixing text and markup) and handling the batched response.

## Phase 2: `parser` Module (Tree-sitter Integration)
Focus: Turning raw files into single `AnnotatedText` blocks using static grammars.
Public Interface: `pub fn parse_document(language: &str, content: &str) -> AnnotatedText`

- [ ] **TDD Cycle 5**: Test mapping `languageId` and file extensions to the statically linked grammars.
- [ ] **TDD Cycle 6**: Test extracting single-line and multi-line doc comments in Rust.
- [ ] **TDD Cycle 7**: Test extraction of text from HTML tags (`<p>`, `<div>`, `<li>`) and exclusion of `<script>`/`<style>`.
- [ ] **TDD Cycle 8**: Test recursive parsing: Markdown prose -> fenced code block -> Rust comments.
- [ ] **TDD Cycle 9**: Test that extracted text snippets track their **absolute byte offset** from the root document to simplify translation later.

## Phase 3: `dictionary` Module (Local Truth)
Focus: Managing multiple `.ltlsp-ignore` files.
Public Interface: `pub struct Dictionary`, `pub fn load(workspace_root: PathBuf, document_path: PathBuf)`, `pub fn is_ignored(&self, word: &str) -> bool`, `pub fn add_word(&self, word: &str, target_file: PathBuf)`

- [ ] **TDD Cycle 10**: Test discovering and merging words from multiple `.ltlsp-ignore` files (workspace root + local module).
- [ ] **TDD Cycle 11**: Test creating a new `.ltlsp-ignore` in the workspace root if none exists.
- [ ] **TDD Cycle 12**: Test filtering a list of `GrammarError` to remove matches for ignored words.

## Phase 4: `server` Module (LSP State & Routing)
Focus: Establishing the LSP loop, managing state, and the Diagnostic Pipeline.
Public Interface: `pub async fn run(connection: Connection, init_options: InitializationOptions)`

- [ ] **TDD Cycle 13**: Test server initialization, extracting LanguageTool `endpoint` from `InitializationOptions`.
- [ ] **TDD Cycle 14**: Test `Arc<RwLock<ServerState>>` tracking `document_versions` (`i32`) to discard stale diagnostics.
- [ ] **TDD Cycle 15**: **Debounce & Cancel**: Test that rapid `didChange` events abort in-flight `tokio::spawn` LT check tasks.
- [ ] **TDD Cycle 16**: **Circuit Breaker**: Test that LT server unavailability sets a cooldown timer in `ServerState`, pausing further requests and sending a `window/showMessage`.
- [ ] **TDD Cycle 17**: Test mapping the single absolute offset returned by LT (+ `AnnotatedText` offset mapping) to LSP `Range`.
- [ ] **TDD Cycle 18**: Test generating CodeActions for adding words to dictionary (one action per found ignore file in path).

## Phase 5: Integration & Verification
- [ ] **TDD Cycle 19**: Module Integration Test using `lsp-server::Connection::memory()` for fast in-process E2E simulation.
- [ ] **TDD Cycle 20**: OS-level Integration Test using `std::process::Command` + `Stdio::piped()` to verify JSON-RPC over `stdin`/`stdout`.
- [ ] Linting (`clippy`) and Typechecking (`cargo check`).

## Phase 6 (Post-MVP): Dynamic Grammars
- [ ] Support downloading and C-compiling additional Tree-sitter grammars (e.g., Java, Kotlin) dynamically based on configuration.
