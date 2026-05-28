# Architecture

`docolint` is Rust Cargo workspace for extracting prose from source files, checking it with LanguageTool, mapping results back into LSP diagnostics.

## Read Next

- `CONTEXT.md`: domain vocabulary.
- `docs/STANDARDS.md`: coding, naming, testing expectations.
- `docs/README.md`: supplemental guides and ADR index.
- `.github/workflows/`: release and automation flow.
- `crates/<crate>/ARCHITECTURE.md`: crate-local maps.

## Workspace Map

- `crates/docolint`: binary entrypoint.
- `crates/docolint-server`: LSP runtime, diagnostics, code actions, container recovery.
- `crates/docolint-parser`: `tree-sitter` extraction and recursive Markdown parsing.
- `crates/docolint-client`: LanguageTool HTTP client.
- `crates/docolint-dictionary`: `.docolint-ignore` loading and filtering.
- `crates/docolint-types`: shared types for extracted text and grammar errors.

## Main Flow

1. Editor sends document content to `docolint-server`.
2. Server calls `docolint-parser` to extract `AnnotatedText`.
3. Server sends extracted prose to `docolint-client`.
4. Client calls LanguageTool, returns `GrammarError` values.
5. Server filters ignored words via `docolint-dictionary`.
6. Server maps offsets back into source ranges, publishes diagnostics and code actions.

## Automation

- `.github/workflows/release.yml`: tag-driven release pipeline for `v*` tags.
- Builds release artifacts for Linux, macOS, and Windows targets.
- Publishes GitHub Release assets, then publishes workspace crates to crates.io.

## Crate Index

- [`crates/docolint/ARCHITECTURE.md`](crates/docolint/ARCHITECTURE.md)
- [`crates/docolint-server/ARCHITECTURE.md`](crates/docolint-server/ARCHITECTURE.md)
- [`crates/docolint-parser/ARCHITECTURE.md`](crates/docolint-parser/ARCHITECTURE.md)
- [`crates/docolint-client/ARCHITECTURE.md`](crates/docolint-client/ARCHITECTURE.md)
- [`crates/docolint-dictionary/ARCHITECTURE.md`](crates/docolint-dictionary/ARCHITECTURE.md)
- [`crates/docolint-types/ARCHITECTURE.md`](crates/docolint-types/ARCHITECTURE.md)

## Design Notes

- AST-first extraction over regex: see `crates/docolint-parser/ARCHITECTURE.md` and `docs/adr/0001-extended-language-support.md`.
- Readability refactor policy: see `docs/adr/0002-function-length-and-readability-refactor-policy.md`.
- Container startup and recovery: see `docs/adr/0003-container-runtime-startup-and-recovery.md`.
