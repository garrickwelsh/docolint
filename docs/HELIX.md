# Helix Configuration

## Configuration Options

- `endpoint = "http://localhost:8081"`: LanguageTool endpoint.
- `language = "en-US"`: LanguageTool language.
- `disableSpellCheck = false`: disable LanguageTool's dictionary spelling rule while keeping grammar and context rules.
- `includeInlineComments = false`: include inline comments for languages that distinguish them from doc comments.

## Minimal Config

Add to `~/.config/helix/languages.toml`:

```toml
[language-server.docolint]
command = "docolint"
```

## Full Default-Values Config

```toml
[language-server.docolint]
command = "docolint"
config = {
  endpoint = "http://localhost:8081",
  language = "en-US",
  disableSpellCheck = false,
  includeInlineComments = false,
}
```

## Attach To Languages

Use `default-servers` to keep Helix's existing language-server set for each language and add `docolint` on top.

Full example covering all currently supported parser languages:
```toml
[language-server.docolint]
command = "docolint"

[[language]]
name = "rust"
language-servers = ["default-servers", "docolint"]

[[language]]
name = "c-sharp"
language-servers = ["default-servers", "docolint"]

[[language]]
name = "markdown"
language-servers = ["default-servers", "docolint"]
# ...add more supported languages as needed.
```

## Common Changes

Custom LanguageTool endpoint:

```toml
[language-server.docolint]
command = "docolint"
config = { endpoint = "http://your-lt-server:8081" }
```

Different LanguageTool language with dictionary spelling disabled:

```toml
[language-server.docolint]
command = "docolint"
config = { language = "en-AU", disableSpellCheck = true }
```

Include inline comments too:

```toml
[language-server.docolint]
command = "docolint"
config = { includeInlineComments = true }
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
language-servers = ["default-servers", "docolint", "codebook"]

[[language]]
name = "markdown"
language-servers = ["default-servers", "docolint", "codebook"]

# ...add more supported languages as needed.
```

## Verify

```bash
hx --health
```
