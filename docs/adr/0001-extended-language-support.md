# ADR-0001: Extended Language Support Decisions

## Context

Extending docolint-parser beyond MVP languages (Rust, C#, HTML, Markdown, JS/TS/Python fallback) to include proper comment extraction for additional languages.

## Decisions

### 1. Comment extraction strategy per language category
Languages with doc comment conventions (Rust, C#, JS/TS/Java): extract doc comments by default. Non-doc inline comments are extracted only when `include_inline_comments` config is `true`. Languages without doc comment conventions (Bash, PowerShell, SCSS, CSS, Lua, Python): extract all comments regardless of config.

### 2. Configurable inline comment inclusion
New `include_inline_comments` option added to `InitializationOptions` (default: `false`). Flows through `ServerState` → `ParserConfig` → language-specific comment extractors. Only affects languages with doc comment distinctions.

### 3. Comment delimiter stripping
All extracted comments have delimiters stripped before sending to LanguageTool. Stripped patterns: `//`, `#`, `/*`, `*/`, `/**`, `///`, `--`, `--[[`, `--]]`, `<#`, `#>`.

### 4. Python docstrings excluded
Python `"""..."""` and `'''...'''` docstrings are NOT extracted as prose. Only `#` comments. Docstrings often contain code examples that would produce false positives.

### 5. JSON removed from supported languages
JSON has no comment syntax. Removed from `language_from_id` and `language_from_extension` mappings.

### 6. Vue and Nushell deferred
Vue SFC and Nushell support postponed. Tree-sitter-nu not on crates.io; tree-sitter-vue has version compatibility issues. Will revisit with git-based dependency strategy.

### 7. Kotlin deferred
`tree-sitter-kotlin` on crates.io requires tree-sitter <0.23 (incompatible). `tree-sitter-kotlin-ng` available but deferring to keep changes minimal. Will revisit with git-based dependency strategy.

### 8. Crates.io-only dependencies
All new tree-sitter grammar crates sourced from crates.io, not git. Keeps build reproducible and avoids network fetches during compilation.

### 9. SCSS silent comments not extractable
SCSS `//` "silent comments" are not exposed in the tree-sitter AST. Only `/* */` block comments are extractable.

### 10. Java uses distinct node kinds
Java tree-sitter uses `block_comment` and `line_comment` node kinds, not `comment`. The extractor handles all three variants.

### 11. Rust and C# inline comments now configurable
Rust and C# keep custom extractors for now instead of moving to the generic extractor. This preserves Rust doc-comment offset precision and C# doc-comment handling while allowing non-doc `//` and `/* */` comments when `include_inline_comments` is `true`.
