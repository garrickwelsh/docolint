# ADR-0003: Container Runtime Startup and Recovery

When local `http://localhost:8081` LanguageTool is unreachable, `docolint` auto-manages shared local container `docolint-lt-server` instead of failing immediately. It tries Docker first, then Podman, uses host networking only when Docker-from-Docker is detected from runtime mount state, otherwise publishes `8081:8081`, recreates mismatched containers, retries one failed request after recovery, and leaves the shared LanguageTool container running across `docolint` shutdown.
