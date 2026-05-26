# Phase 8 Step 4 Handoff: Extract Server Request Handlers

## Goal

Extract request-specific logic from `run` into cohesive private helpers.

## Scope

- Pull out request dispatch branches:
  - `workspace/executeCommand`
  - `textDocument/codeAction`
- Keep shutdown handling behavior unchanged.
- Keep message send/response behavior byte-for-byte compatible where practical.

## Red Preconditions

- Step 3 tests failing first, then passing after extraction.

## Green Implementation Targets

Potential helper boundaries:

1. request router
2. executeCommand handler
3. codeAction handler
4. response send helper

Helpers remain private unless cross-module reuse requires otherwise.

## Files

- `crates/docolint-server/src/lib.rs`
- `IMPLEMENTATION_PLAN.md`

## Verification Commands

```bash
cargo test -p docolint-server
cargo clippy -p docolint-server -- -D warnings
```

## Plan Update on Completion

- Mark Phase 8 Step 4 complete in `IMPLEMENTATION_PLAN.md`.
