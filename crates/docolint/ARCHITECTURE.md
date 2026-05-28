# Architecture

## Purpose

Executable entrypoint for `docolint`. Owns stdio handshake, LSP initialization, server startup.

## Public API

- Binary `main`: creates stdio `Connection`, completes `initialize`, starts `docolint-server::run`.

## Internal Map

- Startup: create LSP transport and parse `InitializeParams`.
- Capability handshake: return `docolint-server::server_capabilities()` during initialization.
- Runtime handoff: delegate all long-lived behavior to `docolint-server`.

## Key Flows

- Editor starts binary -> binary performs JSON-RPC initialization -> server runtime takes over -> stdio threads join on shutdown.

## Tests

- `tests/integration.rs`: process-level handshake test.
- `tests/integration.rs` ignored live test: full LSP flow against running LanguageTool.

## Update When

- Startup sequence changes.
- Binary-specific config or CLI behavior changes.
- Process-level integration expectations change.
