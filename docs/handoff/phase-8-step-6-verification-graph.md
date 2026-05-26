# Phase 8 Step 6 Handoff: Verification and Graph Update

## Goal

Verify refactor behavior unchanged across workspace, then refresh code graph artifacts.

## Scope

- Run full test suite.
- Run lint/type checks used by project.
- Run graph update command to keep graph artifacts current.
- Mark all Phase 8 steps complete in implementation plan.

## Preconditions

- Steps 1-5 complete and committed in working tree.

## Verification Targets

1. Parser behavior unchanged after `extract_markdown_text` split.
2. Server behavior unchanged after `run` handler splits.
3. No new clippy warnings in touched crates.
4. Graph artifacts refreshed for new code structure.

## Files

- `IMPLEMENTATION_PLAN.md`
- `graphify-out/` (tool-managed output)

## Verification Commands

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
graphify update .
```

## Plan Update on Completion

- Mark Phase 8 Step 6 complete in `IMPLEMENTATION_PLAN.md`.
