# ltlsp

Grammar and spelling checking for code comments and prose — powered by [LanguageTool](https://languagetool.org/) and the Language Server Protocol.

`ltlsp` uses `tree-sitter` to extract prose from doc comments, markdown, and other text within source files, then checks it with LanguageTool. Works in Rust, C#, HTML, Markdown, JavaScript/TypeScript, Python, and more.

## Features

- **AST-based extraction** — Uses `tree-sitter` to identify doc comments and prose, avoiding false positives on variable names and code
- **Inline diagnostics** — Grammar and spelling errors appear directly in your editor
- **Quick fixes** — Apply suggested replacements or ignore words with a single action
- **Hierarchical ignore files** — `.ltlsp-ignore` files work like `.gitignore`, scoped from file to workspace root
- **Zero-config** — Auto-starts a LanguageTool Docker container if no server is reachable
- **Multi-language** — Supports Rust, C#, HTML, Markdown, JavaScript, TypeScript, Python, and JSON

## Requirements

A running LanguageTool HTTP server. By default, `ltlsp` expects one at `http://localhost:8081`.

If no server is reachable and Docker is available, `ltlsp` will automatically start a container (`ghcr.io/garrickwelsh/languagetool`).

To run LanguageTool manually:

```bash
docker run -d --network host ghcr.io/garrickwelsh/languagetool
```

## Installation

```bash
cargo install --path .
```

The `ltlsp` binary will be placed in `~/.cargo/bin`. Ensure this directory is on your `$PATH`.

## Editor Configuration

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[language-server.ltlsp]
command = "ltlsp"

[[language]]
name = "rust"
language-servers = ["ltlsp", "rust-analyzer"]

[[language]]
name = "c-sharp"
language-servers = ["ltlsp"]

[[language]]
name = "html"
language-servers = ["ltlsp"]

[[language]]
name = "markdown"
language-servers = ["ltlsp"]

[[language]]
name = "javascript"
language-servers = ["ltlsp"]

[[language]]
name = "typescript"
language-servers = ["ltlsp"]

[[language]]
name = "python"
language-servers = ["ltlsp"]

[[language]]
name = "json"
language-servers = ["ltlsp"]
```

To configure a custom LanguageTool endpoint:

```toml
[language-server.ltlsp]
command = "ltlsp"
config = { endpoint = "http://your-lt-server:8081" }
```

### Neovim

Requires Neovim 0.11+. Add to your `init.lua`:

```lua
vim.lsp.config('ltlsp', {
  cmd = { 'ltlsp' },
  settings = {
    initializationOptions = {
      endpoint = "http://localhost:8081",  -- optional, defaults to localhost:8081
      stopOnExit = false,                   -- optional, stops auto-started Docker on shutdown
    },
  },
})

vim.lsp.enable('ltlsp')
```

### VS Code

A dedicated VS Code extension is planned.

## Ignoring Words

Create a `.ltlsp-ignore` file in your project root or any subdirectory. Each line contains one word to ignore:

```
# Project-specific terms
ltlsp
tree-sitter
languagetool
```

Words are matched case-insensitively. Ignore files are merged hierarchically from the current file up to the workspace root.

When hovering over a grammar error, quick-fix actions let you add the offending word to a `.ltlsp-ignore` file at any directory level.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed component documentation, execution flow diagrams, and design trade-offs.

## License

MPL-2.0. See [LICENSE](LICENSE) for details.
