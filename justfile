update:
    #!/bin/bash
    jq ".image = \"ghcr.io/garrickwelsh/devcontainer-rust:$(skopeo list-tags docker://ghcr.io/garrickwelsh/devcontainer-rust | jq -r .Tags[-1])\"" .devcontainer/devcontainer.json > /tmp/devcontainer.json && mv /tmp/devcontainer.json .devcontainer/devcontainer.json
    jq .image .devcontainer/devcontainer.json

pull:
    docker pull $(jq -r .image .devcontainer/devcontainer.json)

lt-pull:
    docker pull ghcr.io/garrickwelsh/languagetool

lt-server:
    docker run -d -p 8081:8081 --name ltlsp-lt-server ghcr.io/garrickwelsh/languagetool

