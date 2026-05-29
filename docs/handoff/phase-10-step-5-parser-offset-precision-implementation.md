# Phase 10 Step 5 Handoff: Parser Offset Precision Implementation

## Goal

Implement the Stage 2 retained-prose offset invariant for C# and generic doc-comment extraction.

## Scope

- Improve C# and generic doc-comment offset calculation.
- Preserve shared private traversal introduced in Stage 1.
- Do not add XML-aware C# parsing.
- Avoid per-line segment splitting unless tests show it is required.

## Files

- `crates/docolint-parser/src/comments.rs`
- `crates/docolint-parser/src/csharp.rs`
- `crates/docolint-parser/src/generic_comments.rs`
- `crates/docolint-parser/ARCHITECTURE.md`
- `IMPLEMENTATION_PLAN.md`

## Preconditions

- Step 4 offset-invariant tests are red.

## Green Implementation Targets

- Single-line doc comments compute exact retained-prose start offsets.
- Block doc comments improve start offsets to the first retained prose byte where possible.
- Later exact mapping inside joined multi-line blocks remains a follow-up unless tests demand more.

## Plan Update on Completion

Mark Phase 10 Step 5 complete and Step 6 in progress.

## Verification Command

`cargo test -p docolint-parser`
