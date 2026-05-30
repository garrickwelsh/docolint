# Neovim Configuration

Requires Neovim 0.11+.

## Configuration Options

- `endpoint = "http://localhost:8081"`: LanguageTool endpoint.
- `language = "en-US"`: LanguageTool language.
- `disableSpellCheck = false`: disable LanguageTool's dictionary spelling rule while keeping grammar and context rules.
- `includeInlineComments = false`: include inline comments for languages that distinguish them from doc comments.

## Minimal Config

Add to your `init.lua`:

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
})

vim.lsp.enable('docolint')
```

## Full Default-Values Config

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  init_options = {
    endpoint = 'http://localhost:8081',
    language = 'en-US',
    disableSpellCheck = false,
    includeInlineComments = false,
  },
})

vim.lsp.enable('docolint')
```

## Attach To Filetypes

Choose the filetypes you want `docolint` to attach to.

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  filetypes = { 'rust', 'cs', 'markdown' },
})

vim.lsp.enable('docolint')
```

Add more supported filetypes as needed.

## Common Changes

Custom LanguageTool endpoint:

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  init_options = {
    endpoint = 'http://your-lt-server:8081',
  },
})

vim.lsp.enable('docolint')
```

Different LanguageTool language with dictionary spelling disabled:

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  init_options = {
    language = 'en-AU',
    disableSpellCheck = true,
  },
})

vim.lsp.enable('docolint')
```

Include inline comments too:

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  init_options = {
    includeInlineComments = true,
  },
})

vim.lsp.enable('docolint')
```

## Recommended: Codebook + docolint

For best signal/noise, use [Codebook](https://github.com/blopker/codebook) for spelling and configure `docolint` for grammar and context rules only.

Install `codebook-lsp` and make it available on your `$PATH`. Common options include `cargo install codebook-lsp`, `brew install codebook-lsp`, and `pacman -S codebook-lsp`. See the [Codebook installation docs](https://github.com/blopker/codebook#installation) for more.

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  filetypes = { 'rust', 'markdown' },
  init_options = {
    disableSpellCheck = true,
  },
})

vim.lsp.config('codebook', {
  cmd = { 'codebook-lsp', 'serve' },
  filetypes = { 'rust', 'markdown' },
})

vim.lsp.enable('docolint')
vim.lsp.enable('codebook')
```

Add more supported filetypes as needed.

## Verify

Run `:checkhealth vim.lsp` in Neovim.
