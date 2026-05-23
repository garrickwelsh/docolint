# Architecture

## 1. Overview
`docolint` is a Language Server Protocol (LSP) implementation that provides grammar and spelling checking by extracting prose from source code and sending it to a LanguageTool server. It is split into modular crates for separation of concerns.

## 2. Architecture & Flows

The core execution flow is driven by LSP lifecycle events (e.g., `didChange`):

```mermaid
sequenceDiagram
    participant Editor
    participant Server as docolint-server
    participant Parser as docolint-parser
    participant Client as docolint-client
    participant Dict as docolint-dictionary
    participant LT as LanguageTool HTTP

    Editor->>Server: didChange (File content)
    Server->>Parser: Extract prose (content, lang_id)
    Parser-->>Server: AnnotatedText (prose segments + offsets)
    Server->>Client: Validate(AnnotatedText)
    Client->>LT: HTTP /v2/check
    LT-->>Client: Raw Grammar Matches
    Client-->>Server: GrammarError types
    Server->>Dict: Filter ignored words (.docolint-ignore)
    Dict-->>Server: Filtered Errors
    Server->>Server: Map segments back to original offsets
    Server->>Editor: PublishDiagnostics
```

## 3. Modules

*   **`docolint-types`**: Core domain types shared across the workspace (`GrammarError`, `TextSegment`, `AnnotatedText`).
*   **`docolint-parser`**: Prose extraction engine. Uses `tree-sitter` to parse various languages and extract doc comments/prose while ignoring source code.
*   **`docolint-client`**: HTTP client wrapping `reqwest`. Communicates with LanguageTool `/v2/check` API and deserializes responses.
*   **`docolint-dictionary`**: Manages a hierarchical, local dictionary. Merges `.docolint-ignore` files from the current file up to the workspace root to filter out valid project-specific terminology.
*   **`docolint-server`**: Core LSP implementation. Manages server state, coordinates document processing, handles offset mapping, and processes `CodeAction` requests (quick fixes).
*   **`docolint`**: Executable entry point. Sets up stdio connection with the editor and starts the async `docolint-server` runtime.

## 4. Design Choices & Trade-offs

*   **AST-Based Extraction (`tree-sitter`) vs. Regex**: 
    *   *Choice*: Use `tree-sitter` to explicitly identify comments and prose.
    *   *Trade-off*: Increases binary size and build time due to multiple C-based grammars, but drastically reduces false positives (e.g., ignoring variable names).
*   **Annotated Text Segmentation**: 
    *   *Choice*: Isolate raw source logic from HTTP logic using `AnnotatedText`.
    *   *Trade-off*: Adds intermediate object overhead, but allows marking segments as `is_markup` so LanguageTool can ignore internal formatting (like Markdown bold tags) without breaking offset mapping.
*   **Auto-Provisioned Infrastructure Fallback**: 
    *   *Choice*: If the expected LanguageTool HTTP API is unreachable, `docolint-server` attempts to auto-start a local `ghcr.io/garrickwelsh/languagetool` Docker container.
    *   *Trade-off*: Provides a zero-config experience for users with Docker. If Docker is missing and no LanguageTool instance exists, the user is no worse off than if it hadn't tried (graceful failure).
*   **Aggressive Modularization**: 
    *   *Choice*: Split logic into distinct `docolint-*` crates.
    *   *Trade-off*: Requires managing a Cargo workspace, but enforces strict boundaries and enables isolated testing.
