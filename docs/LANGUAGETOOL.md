# LanguageTool Container

`docolint` expects a LanguageTool server at `http://localhost:8081`.

In common setups, you do not need to start it yourself. If no local server is reachable and Docker or Podman is available, `docolint` automatically starts a local `ghcr.io/garrickwelsh/languagetool` container.

## Start manually with Docker

```bash
docker run -d -p 8081:8081 ghcr.io/garrickwelsh/languagetool
```

## Start manually with Podman

```bash
podman run -d -p 8081:8081 ghcr.io/garrickwelsh/languagetool
```

## Advanced: Docker-from-Docker

If `docolint` detects Docker-from-Docker with a host Docker socket mounted into your devcontainer, it automatically starts LanguageTool with host networking so shared `localhost` still works.

Manual Docker command for that setup:

```bash
docker run -d --network host ghcr.io/garrickwelsh/languagetool
```

For most users, port publishing with `-p 8081:8081` is correct.
