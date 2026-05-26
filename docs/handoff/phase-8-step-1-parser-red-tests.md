# Phase 8 Step 1 Handoff: Parser Red Tests

## Goal

Add failing tests that define seams for splitting `extract_markdown_text` without changing behavior.

## Scope

- Target function: `extract_markdown_text` in `crates/docolint-parser/src/lib.rs`
- Add tests in existing parser test module.

## Red Test Targets

1. Fenced code block path delegates into language-aware parse path when language is supported.
2. Unsupported fenced language remains markup/skipped behavior.
3. Gap-text extraction around children preserves plain prose segments and offsets.
4. Markup node filtering behavior remains unchanged.

## Green Implementation Target (next step)

Extract cohesive helpers from `extract_markdown_text` and nested `walk` while preserving exact behavior.

## Files

- `crates/docolint-parser/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Verification Commands

```bash
cargo test -p docolint-parser
```

## Plan Update on Completion

- Mark Phase 8 Step 1 complete in `IMPLEMENTATION_PLAN.md`.
