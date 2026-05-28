# Implementation Plan: docolint

This project will be developed using a strict **Test-Driven Development (TDD)** approach:
`Red (Failing Test) -> Green (Minimal Implementation) -> Refactor`.

Development will be organized by module to support isolated subagent execution.

## Phase 0: Environment Setup
Focus: Ensuring the development environment and network boundaries are ready.

- [x] **Network Strategy**: Update `.devcontainer/devcontainer.json` to use `--network host` in `runArgs`.
- [x] **LT Server Setup**: Add `just` recipes to pull and run the LanguageTool server (`ghcr.io/garrickwelsh/languagetool`) using `--network host` so it binds to the shared `localhost`.
- [x] **Project Init**: Initialize Rust workspace (`crates/*`) and add core dependencies (`lsp-server`, `tree-sitter`, statically linked tree-sitter grammars, `reqwest`, `tokio`, `wiremock` for dev).

## Phase 1: `client` Module (LanguageTool HTTP Client)
Focus: Communicating with the LanguageTool server using batched `AnnotatedText`.
Public Interface: `pub struct LanguageToolClient`, `pub async fn check(&self, text: AnnotatedText) -> Result<Vec<GrammarError>>`

- [x] **TDD Cycle 1**: Test client initialization with configurable `base_url` (supporting both localhost and remote cloud servers).
- [x] **TDD Cycle 2**: Test sending a simple string to a `wiremock` mock LT server and receiving a JSON response.
- [x] **TDD Cycle 3**: Test the mapping of LT `match` objects to internal `GrammarError` models.
- [x] **TDD Cycle 4**: Test compiling an `AnnotatedText` request (mixing text and markup) and handling the batched response.

## Phase 2: `parser` Module (Tree-sitter Integration)
Focus: Turning raw files into single `AnnotatedText` blocks using static grammars.
Public Interface: `pub fn parse_document(language: &str, content: &str) -> AnnotatedText`

### Markdown Grammar (Dual-Parser Architecture)
Markdown requires two passes. Use crate `tree-sitter-md = "0.5.3"` — the official
`tree-sitter-grammars/tree-sitter-markdown` repo, same source Helix uses, published to
crates.io as `tree-sitter-md`.

| Constant / Type | Purpose |
|---|---|
| `tree_sitter_md::LANGUAGE` | Block grammar: sections, headings, lists, code fences, paragraphs |
| `tree_sitter_md::INLINE_LANGUAGE` | Inline grammar: bold, links, code spans |
| `tree_sitter_md::MarkdownParser` (`features = ["parser"]`) | Convenience wrapper: runs both passes, returns `MarkdownTree` |

Parse flow for text extraction:
1. `MarkdownParser::parse(source.as_bytes(), None)` → `MarkdownTree`
2. Walk `MarkdownTree` with `MarkdownCursor`; prose lives in `inline` nodes
3. Fenced code blocks: extract language tag + content → recurse into language-specific
   sub-parser (unknown/unsupported language → treat entire block as **markup**, skip)

- [x] **TDD Cycle 5**: Test mapping `languageId` and file extensions to the statically linked grammars.
- [x] **TDD Cycle 6**: Test extracting single-line and multi-line doc comments in Rust.
- [x] **TDD Cycle 7**: Test extraction of text from HTML tags (`<p>`, `<div>`, `<li>`) and exclusion of `<script>`/`<style>`.
- [x] **TDD Cycle 8**: Test recursive parsing via `MarkdownParser`: prose `inline` nodes extracted as text, fenced code blocks recursed into language-specific sub-parser (Rust doc comments extracted), unknown fence language treated as markup (skipped).
- [x] **TDD Cycle 9**: Test that extracted text snippets track their **absolute byte offset** from the root document to simplify translation later.

## Phase 3: `dictionary` Module (Local Truth)
Focus: Managing multiple `.docolint-ignore` files.
Public Interface: `pub struct Dictionary`, `pub fn load(workspace_root: PathBuf, document_path: PathBuf)`, `pub fn is_ignored(&self, word: &str) -> bool`, `pub fn add_word(&self, word: &str, target_file: PathBuf)`

- [x] **TDD Cycle 10**: Test discovering and merging words from multiple `.docolint-ignore` files (workspace root + local module).
- [x] **TDD Cycle 11**: Test creating a new `.docolint-ignore` in the workspace root if none exists.
- [x] **TDD Cycle 12**: Test filtering a list of `GrammarError` to remove matches for ignored words.

## Phase 4: `server` Module (LSP State & Routing)
Focus: Establishing the LSP loop, managing state, and the Diagnostic Pipeline.
Public Interface: `pub async fn run(connection: Connection, init_options: InitializationOptions)`

- [x] **TDD Cycle 13**: Test server initialization, extracting LanguageTool `endpoint` from `InitializationOptions`.
- [x] **TDD Cycle 14**: Test `Arc<RwLock<ServerState>>` tracking `document_versions` (`i32`) to discard stale diagnostics.
- [x] **TDD Cycle 15**: **Debounce & Cancel**: Test that rapid `didChange` events abort in-flight `tokio::spawn` LT check tasks.
- [x] **TDD Cycle 16**: **Circuit Breaker**: Test that LT server unavailability sets a cooldown timer in `ServerState`, pausing further requests and sending a `window/showMessage`.
- [x] **TDD Cycle 17**: Test mapping the single absolute offset returned by LT (+ `AnnotatedText` offset mapping) to LSP `Range`.
- [x] **TDD Cycle 18**: Test generating CodeActions for adding words to dictionary (one action per found ignore file in path).

## Phase 5: Integration & Verification
- [x] **TDD Cycle 19**: Module Integration Test using `lsp-server::Connection::memory()` for fast in-process E2E simulation.
- [x] **TDD Cycle 20**: OS-level Integration Test using `std::process::Command` + `Stdio::piped()` to verify JSON-RPC over `stdin`/`stdout`.
- [x] Linting (`clippy`) and Typechecking (`cargo check`).

## Phase 6 (Post-MVP): Dynamic Grammars
- [ ] Support downloading and C-compiling additional Tree-sitter grammars (e.g., Java, Kotlin) dynamically based on configuration.

## Phase 7: Extended Language Support
Focus: Add comment extraction for 6 new languages + fix existing JS/TS/Python. Remove JSON.

### Design Decisions
See [docs/adr/0001-extended-language-support.md](docs/adr/0001-extended-language-support.md).

### New Public Interface
```rust
pub struct ParserConfig {
    pub include_inline_comments: bool,
}

pub fn parse_document(language_id: &str, content: &str, config: &ParserConfig) -> AnnotatedText
```

### Dependencies Added
| Language | Crate | Version |
|---|---|---|
| Java | `tree-sitter-java` | `0.23.5` |
| Bash | `tree-sitter-bash` | `0.25.1` |
| PowerShell | `tree-sitter-powershell` | `0.26.4` |
| SCSS | `tree-sitter-scss` | `1.0.0` |
| CSS | `tree-sitter-css` | `0.25.0` |
| Lua | `tree-sitter-lua` | `0.5.0` |

### TDD Cycles
- [x] **TDD Cycle 21**: `ParserConfig` default + CSS comment extraction (tracer bullet)
- [x] **TDD Cycle 22**: Lua `--` and `--[[ ]]` comment extraction
- [x] **TDD Cycle 23**: Bash `#` comment extraction
- [x] **TDD Cycle 24**: PowerShell `#` and `<# #>` comment extraction
- [x] **TDD Cycle 25**: SCSS `/* */` comment extraction (note: `//` silent comments not in AST)
- [x] **TDD Cycle 26**: Python `#` comment extraction
- [x] **TDD Cycle 27**: Java `/** */` doc extraction, `//` excluded by default
- [x] **TDD Cycle 28**: Java inline comments when `include_inline_comments: true`
- [x] **TDD Cycle 29**: JavaScript `/** */` doc extraction, `//` excluded by default
- [x] **TDD Cycle 30**: JSON removed from language mappings
- [x] **TDD Cycle 31**: Markdown recursive parsing with Java fenced blocks
- [x] **TDD Cycle 32**: `include_inline_comments` flows from `InitializationOptions` → `ServerState` → `ParserConfig`
- [x] Linting (`clippy`) and Typechecking (`cargo check`) — zero warnings
- [ ] Future: Refactor Rust/C# custom extractors and generic comment extractor toward shared path if offset precision and doc-comment behavior can stay intact.
- [ ] Future: Investigate optional filtering for code-like inline comments to reduce false positives in opt-in inline comment checking.
- [ ] Future: Add nested parser support for structured doc comment text such as C# XML documentation comments so tags can be treated as markup and prose checked more precisely.
- [ ] Future: Improve stripped block comment offset mapping so diagnostics can start at exact prose position after leading `*` prefixes, not coarse post-delimiter offsets.
- [ ] Future: Normalize leading `*` prefixes in non-doc multi-line block comments without losing precise diagnostic offset mapping.
- [ ] Future: Improve C# doc comment offset mapping so stripped `///` and XML doc content map diagnostics to exact prose positions.

## Phase 8: Long Function Readability Refactor
Focus: Split long, multi-responsibility functions while preserving behavior.

### Design Decisions
See [docs/adr/0002-function-length-and-readability-refactor-policy.md](docs/adr/0002-function-length-and-readability-refactor-policy.md).

### Stepwise Plan (each step updates this plan + writes handoff doc)
- [x] **Step 1 (Red)**: Add parser tests for `extract_markdown_text` helper behavior.
  - Handoff: `docs/handoff/phase-8-step-1-parser-red-tests.md`
- [x] **Step 2 (Green/Refactor)**: Extract markdown parsing helpers from `extract_markdown_text`.
  - Handoff: `docs/handoff/phase-8-step-2-parser-helper-extract.md`
- [x] **Step 3 (Red)**: Add server tests for `run` request/notification handler behavior.
  - Handoff: `docs/handoff/phase-8-step-3-server-red-tests.md`
- [x] **Step 4 (Green/Refactor)**: Extract request handling from `run`.
  - Handoff: `docs/handoff/phase-8-step-4-server-request-handlers.md`
- [x] **Step 5 (Green/Refactor)**: Extract notification handling from `run`.
  - Handoff: `docs/handoff/phase-8-step-5-server-notification-handlers.md`
- [x] **Step 6 (Verify)**: Run full verification and update graph.
  - Handoff: `docs/handoff/phase-8-step-6-verification-graph.md`

## Phase 9: Language-Specific Spell Check Toggle
Focus: Let editors choose LanguageTool language and disable only that language's dictionary spelling rule.

- [x] **TDD Cycle 33**: Add `language` and `disableSpellCheck` initialization options with safe defaults.
- [x] **TDD Cycle 34**: Pass configured language through LT client requests.
- [x] **TDD Cycle 35**: Derive and disable `MORFOLOGIK_RULE_<LANG>` when spell check is disabled.
- [x] **TDD Cycle 36**: Update docs and glossary for grammar-first, optional spelling behavior.
