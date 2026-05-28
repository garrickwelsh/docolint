# Supported Languages

| Language | Doc Comments | Inline Comments | Notes |
|---|---|---|---|
| Rust | ✅ `///`, `//!`, `/** */`, `/*! */` | ⚙️ `//`, `/* */` | Configurable |
| C# | ✅ `///`, `/** */` | ⚙️ `//`, `/* */` | Configurable |
| JavaScript | ✅ `/** */` | ⚙️ `//` | Configurable |
| TypeScript | ✅ `/** */` | ⚙️ `//` | Configurable |
| Python | ✅ `#` | | All comments |
| Java | ✅ `/** */` | ⚙️ `//`, `/* */` | Configurable |
| Bash | ✅ `#` | | All comments |
| PowerShell | ✅ `#`, `<# #>` | | All comments |
| SCSS | ✅ `/* */` | | `//` silent comments not in AST |
| CSS | ✅ `/* */` | | All comments |
| Lua | ✅ `--`, `--[[ ]]` | | All comments |
| HTML | ✅ text nodes | | |
| Markdown | ✅ prose + recursion | | |

⚙️ Inline comments are controlled by the `includeInlineComments` initialization option (default: `false`).
