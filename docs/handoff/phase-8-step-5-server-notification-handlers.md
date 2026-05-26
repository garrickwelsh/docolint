# Phase 8 Step 5 Handoff: Extract Server Notification Handlers

## Goal

Extract notification handling logic from `run` into cohesive private helpers while preserving behavior.

## Scope

- Pull out notification branches:
  - `textDocument/didOpen`
  - `textDocument/didChange`
  - `textDocument/didClose`
- Keep state updates, task registration/cancellation, and version handling unchanged.

## Red Preconditions

- Step 3 tests must exist and fail first for extracted seams, then pass.

## Green Implementation Targets

Possible helper boundaries:

1. notification router
2. didOpen handler
3. didChange handler
4. didClose handler

Helpers stay private unless cross-module use requires broader visibility.

## Files

- `crates/docolint-server/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Verification Commands

```bash
cargo test -p docolint-server
cargo clippy -p docolint-server -- -D warnings
```

## Plan Update on Completion

- Mark Phase 8 Step 5 complete in `IMPLEMENTATION_PLAN.md`.
