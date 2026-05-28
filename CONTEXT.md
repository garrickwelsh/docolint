# Context: docolint

A Language Server Protocol (LSP) implementation in Rust that integrates LanguageTool to provide grammar checking and optional spelling checking for documentation and comments.

## Glossary

### Core Concepts
- **docolint**: The project name. A Rust-based LSP server for documentation grammar checking with optional LanguageTool dictionary spelling checks.
- **LanguageTool Language**: Configured LanguageTool language code (for example `en-US` or `en-AU`) used for requests and for deriving the dictionary spelling rule ID when spell checking is disabled.
- **Local Truth**: A dictionary pattern where a project-local file (`.docolint-ignore`) is the single source of truth for ignored words, rather than the server's internal state.
- **Recursive Parsing**: The process of parsing a document (e.g., Markdown), identifying code blocks, and then running a second parsing pass on those blocks using the appropriate language grammar to extract comments.
- **Circuit Breaker**: A failure-handling pattern that stops sending requests to the LanguageTool server for a cooldown period after a detected failure, preventing system spam and user annoyance.
- **Offset Translation**: The process of mapping relative offsets returned by the LanguageTool API back to absolute byte offsets in the original document for LSP diagnostics.

### Components
- **LSP Server**: The main process implementing the LSP specification using `lsp-server`.
- **Tree-sitter Manager**: The component responsible for language identification and extracting "checkable" text blocks (comments, visible HTML text, etc.).
- **LanguageTool Client**: The HTTP client that communicates with the local LanguageTool server.
- **LanguageTool Container**: Shared local container named `docolint-lt-server` that provides LanguageTool HTTP API when no local service is already reachable.
- **Container Runtime**: Local container CLI used to manage LanguageTool Container. `docolint` tries Docker first, then Podman.
- **Docker-from-Docker**: Development environment where current container has Docker socket mounted from host. In this mode LanguageTool Container must use host networking to share `localhost` with editor and server process.
- **Diagnostic Mapper**: The logic that converts LanguageTool matches into LSP `Diagnostic` objects using offset translation.
