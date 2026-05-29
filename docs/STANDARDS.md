# Standards

Shared repo standards. Keep brief. Do not repeat crate-local structure here.

## Coding

- Prefer small public APIs. Keep helpers private unless reuse needs wider visibility.
- Split code by cohesive responsibility, not arbitrary line counts.
- Functions over 100 lines require readability review. Functions over 150 lines are strong refactor candidates.
- Files over 500 lines require cohesion/readability review, especially when size hurts AI context effectiveness.
- Files over 800 lines are strong split candidates.
- Split files by cohesive responsibility and stable seams, not arbitrary line counts.
- Large test modules may exceed these thresholds when keeping behavior examples together improves locality.
- Prefer direct code over clever abstractions. Introduce abstraction only when shared behavior is real.
- Preserve exact LanguageTool and LSP terms where code crosses those boundaries.

## Naming

- Use glossary terms from `CONTEXT.md` for domain names.
- Keep crate names in `docolint-*` form for reusable workspace crates.
- Name tests for observable behavior, not implementation details.
- Prefer names that describe responsibility over mechanism.

## Style

- Use `cargo fmt` output as formatting baseline.
- Keep modules cohesive. Group parsing, transport, dictionary, types, and server concerns by responsibility.
- Prefer comments that explain non-obvious constraints or mappings, especially around offsets and protocol boundaries.

## Testing

- Prefer behavior-first tests through stable public APIs.
- In-module unit tests are acceptable when parser or server internals need narrow seam coverage.
- Use integration tests for LSP flows and binary startup/handshake behavior.
- Keep live external LanguageTool tests ignored by default unless environment is prepared.
- Behavior changes should ship with test coverage in same area.

## Verification

- Run `cargo test` for behavior verification.
- Run `cargo clippy` for linting.
- Run `cargo fmt --check` before merge when formatting matters.

## Doc Updates

- Crate responsibility change -> update `crates/<crate>/ARCHITECTURE.md`.
- Cross-crate flow change -> update root `ARCHITECTURE.md`.
- Release or CI automation change -> update root `ARCHITECTURE.md` and `docs/README.md`.
- Vocabulary change -> update `CONTEXT.md`.
- Standard or testing policy change -> update this file.
