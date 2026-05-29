# Phase 10 Step 2 Handoff: Parser Shared Comment Walker

## Goal

Deepen parser comment extraction by moving shared AST traversal behind a private seam while preserving Stage 1 behavior.

## Scope

- Keep `parse_document()` and `ParserConfig` as the only public interface.
- Split `docolint-parser` comment extraction into modest private modules.
- Introduce a shared comment-node walker reused by Rust, C#, and generic comment classifiers.
- Preserve current extracted text and offsets exactly.

## Files

- `crates/docolint-parser/src/lib.rs`
- `crates/docolint-parser/src/comments.rs`
- `crates/docolint-parser/src/rust.rs`
- `crates/docolint-parser/src/csharp.rs`
- `crates/docolint-parser/src/generic_comments.rs`
- `crates/docolint-parser/ARCHITECTURE.md`
- `IMPLEMENTATION_PLAN.md`

## Preconditions

- Step 1 characterization tests are green and protect current offset behavior.

## Green Implementation Targets

- Shared private comment walker owns traversal and comment-node detection.
- Rust classifier preserves `doc_comment` child offset precision and current inline comment behavior.
- C# classifier preserves current doc-comment handling and current coarse doc offsets.
- Generic classifier preserves current JS/TS/Java doc-vs-inline policy and comment stripping.

## Plan Update on Completion

Mark Phase 10 Step 2 complete and Step 3 in progress.

## Verification Command

`cargo test -p docolint-parser`
