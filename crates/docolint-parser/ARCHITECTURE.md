# Architecture

## Purpose

Extract checkable prose from source files with `tree-sitter`. Preserve offsets and assign logical prose `unit_id` values so diagnostics map back into original content and checking stays scoped to one prose block.

## Public API

- `ParserConfig`: parser behavior flags, including inline comment inclusion.
- `parse_document()`: parse source content into `AnnotatedText`.

## Internal Map

- Language resolution: map LSP language IDs and file extensions to grammars.
- Comment extraction dispatch: `lib.rs` keeps public entrypoints, owns the document-local `next_unit_id` counter, and routes to private extractors.
- Shared comment traversal: `comments.rs` walks Tree-sitter comment nodes, reuses `unit_id` values across whitespace-adjacent stacked comments, splits code-gapped comments, and centralizes shared segment helpers.
- Language-specific comment classifiers: `rust_comments.rs`, `csharp.rs`, and `generic_comments.rs` preserve language-family extraction rules.
- Markup extraction: HTML text nodes and Markdown prose.
- Recursive parsing: fenced Markdown code blocks parsed with nested language grammars while reusing the same document-local `next_unit_id` stream.
- Offset tracking: shared retained-comment helpers strip line-doc delimiters and leading comment whitespace before recording source offsets; joined multi-line block comments emit one prose segment per retained line so later-line diagnostics map to later retained prose bytes while `plain_text()` stays concatenated.

## Key Flows

- Known language -> parse AST -> extract prose segments -> assign `unit_id` values conservatively -> mark non-prose as markup where needed.
- Markdown -> split prose and code fences -> recurse into fenced language when supported without resetting unit numbering.
- Unknown language -> fall back to one plain-text segment with one fresh `unit_id`.

## Tests

- In-module unit tests cover language mapping, comment stripping, Markdown recursion, HTML extraction, offset tracking.

## Update When

- Supported language set changes.
- Extraction rules, check-unit grouping, or recursive Markdown behavior changes.
- Offset preservation behavior changes.
