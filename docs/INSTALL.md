# Installation

## Install from GitHub Releases

1. Download archive for your platform from [GitHub Releases](https://github.com/garrickwelsh/docolint/releases).
2. Extract `docolint` binary.
3. Move it to directory on your `$PATH`, for example `~/.local/bin` on Linux/macOS.

Example for Linux/macOS:

```bash
curl -L -o docolint.tar.gz https://github.com/garrickwelsh/docolint/releases/latest/download/docolint-x86_64-unknown-linux-gnu.tar.gz
tar -xzf docolint.tar.gz
mv docolint ~/.local/bin/
```

Example for Windows PowerShell:

```powershell
Invoke-WebRequest -OutFile docolint.zip https://github.com/garrickwelsh/docolint/releases/latest/download/docolint-x86_64-windows.zip
Expand-Archive docolint.zip .
```

Release artifact names:

- Linux: `docolint-x86_64-unknown-linux-gnu.tar.gz`, `docolint-x86_64-unknown-linux-musl.tar.gz`, `docolint-aarch64-unknown-linux-gnu.tar.gz`
- macOS: `docolint-aarch64-apple-darwin.tar.gz`
- Windows: `docolint-x86_64-windows.zip`

## Install from Source

```bash
cargo install --path .
```

The `docolint` binary will be placed in `~/.cargo/bin`. Ensure this directory is on your `$PATH`.
