# Architecture

## Purpose

Wrap LanguageTool HTTP API behind crate-local config and response mapping.

## Public API

- `ClientConfig`: base URL, language, spell-check toggle.
- `LanguageToolClient::new()`: construct HTTP client.
- `LanguageToolClient::base_url()`: inspect configured endpoint.
- `LanguageToolClient::check()`: submit `AnnotatedText`, return `GrammarError` values.
- Re-exports `AnnotatedText`, `GrammarError`, `TextSegment` for call-site convenience.

## Internal Map

- Request building: choose plain-text form request or annotated JSON payload.
- Rule configuration: derive disabled spelling rule from configured language.
- Response mapping: deserialize LanguageTool matches into shared error types.

## Key Flows

- Plain text only -> send `text` form field.
- Markup present -> send `data.annotation` payload so LanguageTool skips markup segments.
- Response -> map rule ID, offsets, replacements into `GrammarError`.

## Tests

- In-module tests cover config defaults, spelling rule derivation, plain-text requests, annotated requests, response mapping.

## Update When

- LanguageTool request format changes.
- Client config surface changes.
- Error mapping or disabled-rule behavior changes.
