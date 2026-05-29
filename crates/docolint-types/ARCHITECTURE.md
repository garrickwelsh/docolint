# Architecture

## Purpose

Define shared domain transport types used across parser, client, dictionary, server crates.

## Public API

- `GrammarError`: normalized LanguageTool match.
- `TextSegment`: extracted text or markup segment with source offset and internal `unit_id` metadata.
- `AnnotatedText`: ordered collection of `TextSegment` values.
- `AnnotatedText::plain_text()`: concatenate non-markup segments into checked text.

## Internal Map

- Error model: message, offset, length, replacements, rule ID.
- Segment model: text payload, markup flag, source byte offset, and parser-assigned `unit_id` for check-unit grouping.
- Plain-text view: produce LanguageTool-facing string from non-markup segments only.

## Key Flows

- Parser builds `AnnotatedText`.
- Client serializes checkable text while internal-only `offset` and `unit_id` metadata stay local.
- Server uses segment offsets and `unit_id` grouping to map diagnostics back into source.

## Tests

- In-crate unit test covers serde behavior for internal-only `offset` and `unit_id` metadata.
- Behavior is also exercised through parser, client, dictionary, and server tests.

## Update When

- Shared text or error model changes.
- Serialization expectations for annotated requests change.
- Offset semantics or check-unit metadata change.
