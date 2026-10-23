# Codifica

A Go service scaffold for an HTML website, a forward HTTP proxy with a separate
admin listener, and reverse proxies for services such as GitHub, OpenAI, npm,
and Cargo. The website and forward proxy have separate binaries; service-specific
reverse proxies are planned. Uses the standard library.

## Run

```sh
go run ./cmd/web
```

The website listens on `127.0.0.1:8080`, configured in `cmd/web/main.go`.
Routes: `/` (HTML) and `/healthz` (liveness). SIGINT/SIGTERM trigger graceful
shutdown with a ten-second deadline.

Run the forward proxy separately:

```sh
go run ./cmd/forwardproxy
curl --proxy http://127.0.0.1:8081 http://example.com
curl --proxy http://127.0.0.1:8081 https://example.com
curl http://127.0.0.1:8082/healthz
```

The proxy listens on `127.0.0.1:8081`, with a separate admin listener on
`127.0.0.1:8082`, configured in `cmd/forwardproxy/main.go`. It forwards HTTP and
supports HTTPS CONNECT tunnels. Only hostname destinations are allowed; literal
IP addresses are rejected. Hostnames resolving to private or loopback addresses
are allowed. There is no authentication: keep the listeners private. Shutdown
closes active tunnels as well as ordinary HTTP connections.

## Layout

```text
cmd/web/               Website executable; composition, signals, and startup
cmd/forwardproxy/      Forward proxy and private admin executable
internal/httpserver/   Shared server defaults and multi-listener lifecycle
internal/web/          HTML routes and embedded templates
internal/forwardproxy/ HTTP forwarding, CONNECT tunnels, and admin health handler
internal/serviceproxy/ Planned upstream-specific reverse proxies
```

Handlers do not open ports. Each binary constructs its handlers and servers
directly in its `cmd/` entry point and passes its servers to `httpserver.Run`.
The forward proxy and its admin API should use
separate listeners within the same forward-proxy binary, sharing proxy state.
Servers start independently using `ListenAndServe`. A server failure shuts down
the other servers rather than leaving a partially running service.

Add separate commands such as `cmd/githubproxy`, `cmd/openaiproxy`, `cmd/npmproxy`,
and `cmd/cargoproxy` as those services are implemented. These entry points are not
created yet. Build each independently, for example:

```sh
go build -o bin/web ./cmd/web
```

Introduce shared domain packages only when real behavior needs them, not a
generic framework up front.
Proxy authentication, listener TLS, fine-grained access control, and upstream
routing are not yet implemented. The listeners use the standard library's default
timeouts, without header-read or whole-request deadlines.

## Check

```sh
go test -race ./...
go vet ./...
```
