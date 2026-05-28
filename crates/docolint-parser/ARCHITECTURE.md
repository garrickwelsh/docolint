# Architecture

## Purpose

Extract checkable prose from source files with `tree-sitter`. Preserve offsets so diagnostics can map back into original content.

## Public API

- `ParserConfig`: parser behavior flags, including inline comment inclusion.
- `parse_document()`: parse source content into `AnnotatedText`.

## Internal Map

- Language resolution: map LSP language IDs and file extensions to grammars.
- Comment extraction: Rust, C#, JS/TS/Java, shell-style, CSS-style comment handling.
- Markup extraction: HTML text nodes and Markdown prose.
- Recursive parsing: fenced Markdown code blocks parsed with nested language grammars.
- Offset tracking: each extracted segment keeps original byte offset.

## Key Flows

- Known language -> parse AST -> extract prose segments -> mark non-prose as markup where needed.
- Markdown -> split prose and code fences -> recurse into fenced language when supported.
- Unknown language -> fall back to plain text.

## Tests

- In-module unit tests cover language mapping, comment stripping, Markdown recursion, HTML extraction, offset tracking.

## Update When

- Supported language set changes.
- Extraction rules or recursive Markdown behavior changes.
- Offset preservation behavior changes.
