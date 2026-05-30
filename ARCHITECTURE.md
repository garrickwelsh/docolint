# Architecture

`docolint` is Rust Cargo workspace for extracting prose from source files, checking it with LanguageTool, mapping results back into LSP diagnostics.

## Read Next

- `CONTEXT.md`: domain vocabulary.
- `docs/STANDARDS.md`: coding, naming, testing expectations.
- `docs/README.md`: supplemental guides and ADR index.
- `docs/DIAGRAMS.md`: visual architecture and offset-mapping flows.
- `.github/workflows/`: release and automation flow.
- `crates/<crate>/ARCHITECTURE.md`: crate-local maps.

## Workspace Map

- `crates/docolint`: binary entrypoint.
- `crates/docolint-server`: LSP runtime, per-unit LanguageTool orchestration, cached diagnostics, code actions, container recovery.
- `crates/docolint-parser`: `tree-sitter` extraction, parser-assigned check units, recursive Markdown parsing.
- `crates/docolint-client`: LanguageTool HTTP client.
- `crates/docolint-dictionary`: workspace-root `.docolint-ignore` loading and filtering.
- `crates/docolint-types`: shared types for extracted text, `unit_id` metadata, and grammar errors.

## Main Flow

1. Editor sends document content to `docolint-server`.
2. Server calls `docolint-parser` to extract `AnnotatedText` with parser-assigned `unit_id` values for logical prose blocks.
3. Server groups segments by `unit_id` and sends one check unit at a time to `docolint-client`.
4. Client calls LanguageTool, returns `GrammarError` values per unit.
5. Server filters ignored words via `docolint-dictionary`, maps unit-local offsets back into source ranges, and caches built LSP diagnostics.
6. Server republishes cached diagnostics on document and dictionary changes, then serves diagnostics and code actions.

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
