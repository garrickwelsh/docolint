update:
    #!/bin/bash
    jq ".image = \"ghcr.io/garrickwelsh/devcontainer-rust:$(skopeo list-tags docker://ghcr.io/garrickwelsh/devcontainer-rust | jq -r .Tags[-1])\"" .devcontainer/devcontainer.json > /tmp/devcontainer.json && mv /tmp/devcontainer.json .devcontainer/devcontainer.json
    jq .image .devcontainer/devcontainer.json

pull:
    docker pull $(jq -r .image .devcontainer/devcontainer.json)

lt-pull:
    docker pull ghcr.io/garrickwelsh/languagetool

lt-server:
    docker run -d --network host --name ltlsp-lt-server ghcr.io/garrickwelsh/languagetool

run:
    cargo run -p ltlsp

test:
    cargo test

test-all:
    #!/bin/bash
    set -euo pipefail
    if ! docker ps --format '{{{{.Names}}}}' | grep -q '^ltlsp-lt-server$'; then
        docker start ltlsp-lt-server 2>/dev/null || just lt-server
        echo "Waiting for LanguageTool to become ready..."
        for i in $(seq 1 30); do
            if nc -z localhost 8081 2>/dev/null; then break; fi
            sleep 1
        done
    fi
    cargo test -- --include-ignored
