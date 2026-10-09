# Codifica

A Go service scaffold for an HTML website, a forward HTTP proxy with a separate
admin listener, and reverse proxies for services such as GitHub, OpenAI, npm,
and Cargo. Each server has its own binary; only the website entry point is
implemented initially. Uses the standard library.

## Run

```sh
go run ./cmd/web
```

The website listens on `127.0.0.1:8080`, configured in `cmd/web/main.go`.
Routes: `/` (HTML) and `/healthz` (liveness). SIGINT/SIGTERM trigger graceful
shutdown with a ten-second deadline.

## Layout

```text
cmd/web/               Website executable; composition, signals, and startup
internal/httpserver/   Shared server defaults and multi-listener lifecycle
internal/web/          HTML routes and embedded templates
internal/forwardproxy/ Planned forward proxy and separate admin handler
internal/serviceproxy/ Planned upstream-specific reverse proxies
```

Handlers do not open ports. Each binary constructs its handlers and servers
directly in its `cmd/` entry point and passes its servers to `httpserver.Run`.
The forward proxy and its admin API should use
separate listeners within the same forward-proxy binary, sharing proxy state.
Servers start independently using `ListenAndServe`. A server failure shuts down
the other servers rather than leaving a partially running service.

Add `cmd/forwardproxy` for the forward proxy and admin API, and separate commands
such as `cmd/githubproxy`, `cmd/openaiproxy`, `cmd/npmproxy`, and `cmd/cargoproxy`
as those services are implemented. These entry points are intentionally not
created yet. Build each independently, for example:

```sh
go build -o bin/web ./cmd/web
```

Introduce shared domain packages only when real behavior needs them, not a
generic framework up front.
Proxy authentication, access control, TLS, and upstream routing are not yet
implemented. Server timeouts should be reviewed for streaming and tunneling.

## Check

```sh
go test -race ./...
go vet ./...
```
