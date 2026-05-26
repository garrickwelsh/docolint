# Phase 8 Step 3 Handoff: Server Red Tests

## Goal

Add failing tests that define extraction seams for `docolint-server::run` request and notification handling.

## Scope

- Target function: `run` in `crates/docolint-server/src/lib.rs`
- Add tests in server unit tests and/or integration tests using `Connection::memory()`.

## Red Test Targets

1. `workspace/executeCommand` ignore-word flow still writes dictionary and triggers recheck.
2. `textDocument/codeAction` still emits replacement + ignore actions for diagnostics.
3. `didOpen` stores doc state and schedules check.
4. `didChange` updates version/content and schedules check.
5. `didClose` clears state and cancels task.

## Green Implementation Target (next step)

Extract request-handler logic from `run` without behavior changes.

## Files

- `crates/docolint-server/src/lib.rs`
- `crates/docolint-server/tests/integration.rs` (if needed)
- `IMPLEMENTATION_PLAN.md`

## Verification Commands

```bash
cargo test -p docolint-server
```

## Plan Update on Completion

- Mark Phase 8 Step 3 complete in `IMPLEMENTATION_PLAN.md`.
