# Phase 10 Step 1 Handoff: Parser Offset Characterization Tests

## Goal

Lock in current Stage 1 comment offset behavior before refactoring parser internals.

## Scope

- Add parser tests for current offset behavior of:
  - C# `///` doc comments
  - C# `/** */` doc comments
  - Java `/** */` doc comments
  - JavaScript `/** */` doc comments
  - CSS/SCSS `/* */` comments
  - Bash/Python line comments
- Keep tests behavior-first through `parse_document()`.

## Files

- `crates/docolint-parser/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Red Test Targets

- Assert current offsets even when they are coarse and delimiter-based.
- Do not change implementation in this step.

## Green Implementation Target (next step)

Split the parser into private comment extraction modules and introduce a shared walker without changing the characterized behavior.

## Plan Update on Completion

Mark Phase 10 Step 1 complete and Step 2 in progress.

## Verification Command

`cargo test -p docolint-parser`
