# Neovim Configuration

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

## Recommended: Codebook + docolint

For best signal/noise, use [Codebook](https://github.com/blopker/codebook) for spelling and configure `docolint` for grammar and context rules only.

Install `codebook-lsp` and make it available on your `$PATH`. Common options include `cargo install codebook-lsp`, `brew install codebook-lsp`, and `pacman -S codebook-lsp`. See the [Codebook installation docs](https://github.com/blopker/codebook#installation) for more.

```lua
vim.lsp.config('docolint', {
  cmd = { 'docolint' },
  settings = {
    initializationOptions = {
      disableSpellCheck = true,
    },
  },
})

vim.lsp.enable('docolint')
vim.lsp.enable('codebook')
```

`includeInlineComments = true` expands `docolint` from doc comments and prose to inline comments when you want grammar checks there too.
