# Diagrams

Contributor-facing diagrams for the main runtime flows.

## Workspace Component Flow

```mermaid
flowchart LR
    Editor[Editor / LSP client] --> Server[docolint-server]
    Server --> Parser[docolint-parser]
    Server --> Client[docolint-client]
    Server --> Dictionary[docolint-dictionary]
    Client --> LT[LanguageTool HTTP API]
```

Update when: crate responsibilities or main cross-crate calls change.

## Check-Unit Diagnostic Flow

```mermaid
flowchart TD
    A[Document event] --> B[parse_document]
    B --> C[AnnotatedText with unit_id values]
    C --> D[Server groups segments by unit_id]
    D --> E[One LanguageTool request per Check Unit]
    E --> F[GrammarError values]
    F --> G[Dictionary filtering]
    G --> H[Offset translation to source ranges]
    H --> I[Publish and cache diagnostics]
```

Update when: check-unit grouping, filtering order, or diagnostic publication changes.

## LanguageTool Startup And Recovery Flow

```mermaid
flowchart TD
    A[Server starts] --> B{Endpoint reachable?}
    B -->|Yes| C[Use existing LanguageTool service]
    B -->|No| D{Local localhost endpoint?}
    D -->|No| E[Keep configured endpoint and fail requests normally]
    D -->|Yes| F[Try Docker, then Podman]
    F --> G[Reuse or recreate shared docolint-lt-server]
    G --> H[Probe endpoint again]
    H -->|Recovered| I[Retry one failed request]
    H -->|Still down| J[Circuit breaker cooldown]
```

Update when: recovery order, container runtime behavior, or retry policy changes.

## Offset Translation Flow

```mermaid
flowchart TD
    A[Source document] --> B[Parser emits TextSegment]
    B --> B1[text, is_markup, offset, unit_id]
    B1 --> C[Server groups segments by unit_id]
    C --> D[plain_text concatenates non-markup segments]
    D --> E[LanguageTool returns offset and length in plain text]
    E --> F[map_lt_offset_to_absolute walks non-markup segments and adds segment.offset]
    F --> G[offset_to_position converts absolute byte offsets to LSP positions]
```

Plain-text example:

```text
source:   "/// A sentnce."
segments: ["A sentnce." at source byte offset 4]
LT:       offset 2, length 7
map:      plain-text offset 2 -> source byte offset 6
result:   byte offsets -> LSP range via offset_to_position()
```

Update when: `TextSegment` structure, plain-text concatenation, or offset mapping logic changes.
