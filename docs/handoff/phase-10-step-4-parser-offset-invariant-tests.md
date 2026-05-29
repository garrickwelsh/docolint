# Phase 10 Step 4 Handoff: Parser Offset Invariant Tests

## Goal

Add Stage 2 red tests for the retained-prose offset invariant.

## Scope

- Add tests that require `TextSegment.offset` to point at the first retained prose byte whenever possible.
- Focus on C# and generic doc-comment languages.
- Keep XML-aware C# parsing out of scope.

## Files

- `crates/docolint-parser/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Red Test Targets

- C# `///` doc comments start at the first retained prose byte.
- C# `/** */` doc comments start at the first retained prose byte.
- Java and JavaScript `/** */` doc comments start at the first retained prose byte.

## Green Implementation Target (next step)

Improve classifier offset calculation to satisfy the new invariant without taking on XML-aware parsing.

## Plan Update on Completion

Mark Phase 10 Step 4 complete and Step 5 in progress.

## Verification Command

`cargo test -p docolint-parser`
