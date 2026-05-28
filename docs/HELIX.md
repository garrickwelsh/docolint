# Helix Configuration

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

Custom LanguageTool endpoint:

```toml
[language-server.docolint]
command = "docolint"
config = { endpoint = "http://your-lt-server:8081" }
```

Specific LanguageTool language with dictionary spelling disabled:

```toml
[language-server.docolint]
command = "docolint"
config = { language = "en-AU", disableSpellCheck = true }
```

## Recommended: Codebook + docolint

For best signal/noise, use [Codebook](https://github.com/blopker/codebook) for spelling and configure `docolint` for grammar and context rules only.

Install `codebook-lsp` and make it available on your `$PATH`. Common options include `cargo install codebook-lsp`, `brew install codebook-lsp`, and `pacman -S codebook-lsp`. See the [Codebook installation docs](https://github.com/blopker/codebook#installation) for more.

```toml
[language-server.docolint]
command = "docolint"
config = { disableSpellCheck = true }

[language-server.codebook]
command = "codebook-lsp"
args = ["serve"]

[[language]]
name = "rust"
language-servers = ["docolint", "codebook", "rust-analyzer"]

[[language]]
name = "markdown"
language-servers = ["docolint", "codebook"]

[[language]]
name = "typescript"
language-servers = ["docolint", "codebook"]
```

`docolint` defaults to doc comments and prose-oriented content. If you want grammar checking for inline comments too, set `includeInlineComments = true` on `docolint`.

You can verify setup with:

```bash
hx --health rust
```
