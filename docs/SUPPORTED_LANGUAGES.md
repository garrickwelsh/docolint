# Supported Languages

| Language | Doc Comments | Inline Comments | Notes |
|---|---|---|---|
| Rust | ✅ `///`, `//!`, `/** */`, `/*! */` | ⚙️ `//`, `/* */` | Configurable |
| C# | ✅ `///`, `/** */` | ⚙️ `//`, `/* */` | Configurable |
| JavaScript | ✅ `/** */` | ⚙️ `//` | Configurable |
| TypeScript | ✅ `/** */` | ⚙️ `//` | Configurable |
| TSX | ✅ `/** */` | ⚙️ `//` | Configurable |
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

## Unsupported Languages

These languages are documented by `harper-ls` for Helix `[[language]]` configuration but are not currently included in `docolint` parser support. They are reasonable future support candidates:

- AsciiDoc: `asciidoc`
- C: `c`
- Clojure: `clojure`
- CMake: `cmake`
- C++: `cpp`
- DAML: `daml`
- Dart: `dart`
- Git Commit: `git-commit`, `gitcommit`
- Go: `go`
- Groovy: `groovy`
- Haskell: `haskell`
- Ink: `ink`
- JavaScript React: `javascriptreact`
- Jujutsu Description: `jj-commit`, `jjdescription`
- Kotlin: `kotlin`
- Literate Haskell: `lhaskell`, `literate haskell`
- Email: `mail`
- Nix: `nix`
- Org Mode: `org`
- PHP: `php`
- Plain Text: `plaintext`, `text`
- Ruby: `ruby`
- Scala: `scala`
- Shell/Bash Script: `shellscript`
- Solidity: `solidity`
- Swift: `swift`
- TOML: `toml`
- Typst: `typst`
- TypeScript React: `typescriptreact`
- Zig: `zig`
- LaTeX/TeX: `latex`, `tex`, `plaintex`
