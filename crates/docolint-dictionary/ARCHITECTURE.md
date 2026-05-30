# Architecture

## Purpose

Provide local ignore-word filtering based on a workspace-root `.docolint-ignore` file.

## Public API

- `Dictionary::new()`: empty dictionary.
- `Dictionary::load()`: load the workspace-root ignore file.
- `Dictionary::is_ignored()`: case-insensitive membership check.
- `Dictionary::add_word()`: append word to target ignore file.
- `Dictionary::filter_errors()`: remove ignored `GrammarError` matches from result set.

## Internal Map

- Root load: read only `workspace_root/.docolint-ignore`.
- Normalization: lowercase stored words for case-insensitive matching.
- Offset translation: convert character offsets to byte offsets before slicing checked text.

## Key Flows

- File open/change -> load workspace-root ignore set for current document.
- Ignore-word action -> append to workspace-root file -> keep in-memory set aligned.
- Grammar result filter -> drop matches whose text resolves to ignored words.

## Tests

- In-module tests cover root-only loading, case-insensitive matching, file creation, filtering, Unicode-safe offsets.

## Update When

- `.docolint-ignore` lookup rules change.
- Filtering behavior or normalization changes.
- Ignore file write behavior changes.
