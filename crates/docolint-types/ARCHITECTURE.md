# Architecture

## Purpose

Define shared domain transport types used across parser, client, dictionary, server crates.

## Public API

- `GrammarError`: normalized LanguageTool match.
- `TextSegment`: extracted text or markup segment with source offset.
- `AnnotatedText`: ordered collection of `TextSegment` values.
- `AnnotatedText::plain_text()`: concatenate non-markup segments into checked text.

## Internal Map

- Error model: message, offset, length, replacements, rule ID.
- Segment model: text payload, markup flag, source byte offset.
- Plain-text view: produce LanguageTool-facing string from non-markup segments only.

## Key Flows

- Parser builds `AnnotatedText`.
- Client serializes segments or plain text for LanguageTool.
- Server uses offsets on segments to map diagnostics back into source.

## Tests

- No dedicated crate test file now. Behavior verified indirectly through parser, client, dictionary, server tests.

## Update When

- Shared text or error model changes.
- Serialization expectations for annotated requests change.
- Offset semantics change.
