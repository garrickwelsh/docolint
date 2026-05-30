# Architecture

## Purpose

Core LSP runtime. Coordinates parsing, per-unit LanguageTool calls, dictionary filtering, diagnostic caching, code actions, container recovery.

## Public API

- `server_capabilities()`: advertised LSP capabilities.
- `generate_ignore_actions()`: quick fix for the workspace-root `.docolint-ignore` target.
- `generate_replacement_actions()`: quick fixes from suggested replacements.
- `map_lt_offset_to_absolute()`: plain-text offset to source offset mapping.
- `offset_to_position()`: source byte offset to LSP position.
- `run()`: async server event loop.

## Internal Map

- Server state: open documents, versions, languages, cached diagnostics, cached tasks, cooldown state.
- Request handling: `initialize`, `shutdown`, code actions, execute command flow.
- Notification handling: open/change/close lifecycle and recheck scheduling.
- Per-unit checking: group parser output by `unit_id` and send one LanguageTool request per check unit.
- Diagnostics: map unit-local `GrammarError` values into LSP `Diagnostic` payloads early, then republish cached diagnostics through dictionary filtering.
- Recovery: local LanguageTool reachability checks and container startup fallback.

## Key Flows

- Document event -> parse -> group by `unit_id` -> check each unit -> dictionary filter -> map/cache diagnostics -> publish diagnostics.
- Diagnostic request -> replacement and workspace-dictionary ignore-word code actions.
- Dictionary change -> reload ignore words -> re-filter cached diagnostics -> republish.
- LanguageTool failure -> circuit breaker cooldown -> retry after window expires.

## Tests

- In-module unit tests for per-unit grouping, offset mapping, actions, state helpers, recovery behavior.
- `tests/integration.rs`: in-memory LSP flow with mocked LanguageTool.

## Update When

- LSP flow, server state, or code action behavior changes.
- Parser/client/dictionary orchestration or diagnostic caching behavior changes.
- Container recovery or cooldown behavior changes.
