# Phase 8 Step 2 Handoff: Parser Helper Extraction

## Goal

Refactor `extract_markdown_text` into smaller cohesive helpers to reduce cognitive load.

## Scope

- Split fenced-code handling from general node traversal.
- Split markup-node predicate.
- Split child-gap extraction.
- Keep behavior and offsets stable.

## Red Preconditions

- Step 1 tests must be failing first, then pass after this step.

## Green Implementation Targets

Possible helper seams:

1. fenced block info extraction
2. supported-language recursion branch
3. markup-kind filter
4. gap-text emit logic
5. leaf-text emit logic

Helper names can vary; keep helpers private unless reuse forces wider visibility.

## Files

- `crates/docolint-parser/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Verification Commands

```bash
cargo test -p docolint-parser
cargo clippy -p docolint-parser -- -D warnings
```

## Plan Update on Completion

- Mark Phase 8 Step 2 complete in `IMPLEMENTATION_PLAN.md`.
