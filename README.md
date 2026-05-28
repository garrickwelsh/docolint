# docolint

Grammar checking and optional spelling checking for code comments and prose — powered by [LanguageTool](https://languagetool.org/) and the Language Server Protocol.

`docolint` uses `tree-sitter` to extract prose from doc comments, markdown, and other text within source files, then checks it with LanguageTool. Works in Rust, C#, HTML, Markdown, JavaScript/TypeScript, Python, and more.

## Features

- **AST-based extraction** — Uses `tree-sitter` to identify doc comments and prose, avoiding false positives on variable names and code
- **Inline diagnostics** — Grammar and optional spelling errors appear directly in your editor
- **Quick fixes** — Apply suggested replacements or ignore words with a single action
- **Hierarchical ignore files** — `.docolint-ignore` files work like `.gitignore`, scoped from file to workspace root
- **Zero-config** — Auto-starts a local LanguageTool container via Docker or Podman if no local server is reachable
- **Multi-language** — Supports Rust, C#, HTML, Markdown, JavaScript, TypeScript, Python, Java, Bash, PowerShell, SCSS, CSS, and Lua

## Requirements

A running LanguageTool HTTP server. By default, `docolint` expects one at `http://localhost:8081`.

If no local server is reachable and Docker or Podman is available, `docolint` will automatically start a container (`ghcr.io/garrickwelsh/languagetool`). Docker is tried first, then Podman.

When `docolint` runs inside a Docker-from-Docker devcontainer with Docker socket mounted from host, it starts LanguageTool with host networking so container shares devcontainer `localhost`. Otherwise it starts normally with `-p 8081:8081`.

To run LanguageTool manually in a Docker-from-Docker devcontainer:

```bash
docker run -d --network host ghcr.io/garrickwelsh/languagetool
```

To run LanguageTool manually on host, Docker-in-Docker, or other non-host-network setups:

```bash
docker run -d -p 8081:8081 ghcr.io/garrickwelsh/languagetool
```

Equivalent Podman commands work too.

## Installation

```bash
cargo install --path .
```

The `docolint` binary will be placed in `~/.cargo/bin`. Ensure this directory is on your `$PATH`.

## Editor Configuration

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[language-server.docolint]
command = "docolint"

[[language]]
name = "rust"
language-servers = ["docolint", "rust-analyzer"]

[[language]]
name = "c-sharp"
language-servers = ["docolint"]

[[language]]
name = "html"
language-servers = ["docolint"]

[[language]]
name = "markdown"
language-servers = ["docolint"]

[[language]]
name = "javascript"
language-servers = ["docolint"]

[[language]]
name = "typescript"
language-servers = ["docolint"]

[[language]]
name = "python"
language-servers = ["docolint"]

[[language]]
name = "json"
language-servers = ["docolint"]

[[language]]
name = "java"
language-servers = ["docolint"]

[[language]]
name = "bash"
language-servers = ["docolint"]

[[language]]
name = "powershell"
language-servers = ["docolint"]

[[language]]
name = "scss"
language-servers = ["docolint"]

[[language]]
name = "css"
language-servers = ["docolint"]

[[language]]
name = "lua"
language-servers = ["docolint"]
```

To configure a custom LanguageTool endpoint:

```toml
[language-server.docolint]
command = "docolint"
config = { endpoint = "http://your-lt-server:8081" }
```

To configure a specific LanguageTool language and disable its dictionary spelling rule:

```toml
[language-server.docolint]
command = "docolint"
config = { language = "en-AU", disableSpellCheck = true }
```

### Neovim

Requires Neovim 0.11+. Add to your `init.lua`:

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  settings = {
    initializationOptions = {
      endpoint = "http://localhost:8081",  -- optional, defaults to localhost:8081
      language = "en-AU",                  -- optional, defaults to en-US
      disableSpellCheck = true,             -- optional, keeps grammar/context rules enabled
      stopOnExit = false,                   -- optional but currently ignored; LT container is shared
    },
  },
})

vim.lsp.enable('docolint')
```

### VS Code

VS Code support may be added in the future.

## Supported Languages

| Language | Doc Comments | Inline Comments | Notes |
|---|---|---|---|
| Rust | ✅ `///`, `/** */` | ❌ | |
| C# | ✅ `///`, `/** */` | ❌ | |
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

## Ignoring Words

Create a `.docolint-ignore` file in your project root or any subdirectory. Each line contains one word to ignore:

```
# Project-specific terms
docolint
tree-sitter
languagetool
```

Words are matched case-insensitively. Ignore files are merged hierarchically from the current file up to the workspace root.

When hovering over a grammar error, quick-fix actions let you add the offending word to a `.docolint-ignore` file at any directory level.

`disableSpellCheck = true` disables LanguageTool's dictionary spelling rule for the configured language, for example `MORFOLOGIK_RULE_EN_AU`, while keeping grammar and context-sensitive rules enabled.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed component documentation, execution flow diagrams, and design trade-offs.

## License

MPL-2.0. See [LICENSE](LICENSE) for details.
