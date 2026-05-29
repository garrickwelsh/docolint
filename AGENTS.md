## Project Docs

Read docs in this order:
- `CONTEXT.md` for domain vocabulary. Keep terms aligned with glossary.
- `ARCHITECTURE.md` for repo overview, crate index, doc routing.
- `docs/STANDARDS.md` for coding, naming, style, testing expectations.
- `docs/README.md` for supplemental repo docs.
- `.github/workflows/` for release and automation flow.
- `docs/adr/` for hard-to-reverse design decisions.
- `crates/<crate>/ARCHITECTURE.md` for crate-local code maps.

## Update Rules

- Crate structure or responsibilities changed -> update that crate's `ARCHITECTURE.md`.
- Cross-crate flow or dependency changed -> update root `ARCHITECTURE.md`.
- Release or CI automation changed -> update root `ARCHITECTURE.md` and `docs/README.md`.
- Domain vocabulary changed -> update `CONTEXT.md`.
- Coding, naming, style, or test expectations changed -> update `docs/STANDARDS.md`.
- Surprising, hard-to-reverse trade-off chosen -> add or update ADR.

## Writing Rules

- Keep docs brief, specific, non-redundant.
- Root docs route to detail; crate docs describe crate internals.
- Prefer responsibility maps over function inventories.
- When code changes, update closest matching map in same change.
