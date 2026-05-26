# ADR-0002: Function Length and Readability Refactor Policy

## Context

Current code has long functions that mix many responsibilities:

- `docolint-server::run` in `crates/docolint-server/src/lib.rs` is about 220 lines.
- `docolint-parser::extract_markdown_text` in `crates/docolint-parser/src/lib.rs` is about 140 lines, with nested `walk` around 127 lines.

These functions are harder to scan, reason about, and safely change. This affects both human maintenance and AI-assisted navigation.

## Decision

Adopt a readability refactor policy for function length:

1. Function length is a review signal, not a strict gate.
2. Functions over 100 lines trigger readability review.
3. Functions over 150 lines are strong refactor candidates.
4. Refactors must split by cohesive responsibility boundaries, not arbitrary line-count slicing.
5. Prefer keeping extracted helpers private unless cross-module reuse requires broader visibility.

## Testing Policy During Refactor

1. Add failing tests first when introducing new internal seams.
2. Prefer behavior-focused tests through stable public interfaces.
3. For parser/server internals where behavior cannot be isolated cleanly via public APIs, private helper unit tests in the same module are acceptable.
4. Existing tests must continue passing after each split step.

## Process Policy

For multi-step refactors under this policy:

1. Update `IMPLEMENTATION_PLAN.md` as each step completes.
2. Create a per-step handoff doc under `docs/handoff/` with goal, files, red test target, green implementation target, and verification commands.

## Scope

This policy is applied immediately to:

- `docolint-server::run`
- `docolint-parser::extract_markdown_text`

Future long functions should be evaluated with this same policy.

## Consequences

### Positive

- Lower cognitive load in core server loop and markdown parsing flow.
- Clearer seams for testing and future change.
- Better maintainability for both human and AI contributors.

### Trade-offs

- More internal helper functions and test cases.
- Short-term refactor effort before long-term readability gains.
