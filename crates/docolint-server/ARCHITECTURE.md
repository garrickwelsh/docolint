# Architecture

## Purpose

Core LSP runtime. Coordinates parsing, LanguageTool calls, dictionary filtering, diagnostic mapping, code actions, container recovery.

## Public API

- `server_capabilities()`: advertised LSP capabilities.
- `generate_ignore_actions()`: quick fixes for `.docolint-ignore` targets.
- `generate_replacement_actions()`: quick fixes from suggested replacements.
- `map_lt_offset_to_absolute()`: plain-text offset to source offset mapping.
- `offset_to_position()`: source byte offset to LSP position.
- `run()`: async server event loop.

## Internal Map

- Server state: open documents, versions, languages, cached tasks, cooldown state.
- Request handling: `initialize`, `shutdown`, code actions, execute command flow.
- Notification handling: open/change/close lifecycle and recheck scheduling.
- Diagnostics: map `GrammarError` values into LSP `Diagnostic` payloads.
- Recovery: local LanguageTool reachability checks and container startup fallback.

## Key Flows

- Document event -> parse -> check -> dictionary filter -> offset mapping -> publish diagnostics.
- Diagnostic request -> replacement and ignore-word code actions.
- LanguageTool failure -> circuit breaker cooldown -> retry after window expires.

## Tests

- In-module unit tests for mapping, actions, state helpers, recovery behavior.
- `tests/integration.rs`: in-memory LSP flow with mocked LanguageTool.

## Update When

- LSP flow, server state, or code action behavior changes.
- Parser/client/dictionary orchestration changes.
- Container recovery or cooldown behavior changes.
