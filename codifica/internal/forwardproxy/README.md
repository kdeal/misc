# Forward proxy

`New` constructs shared proxy state. `Handler` forwards absolute HTTP URLs and
supports HTTPS through HTTP/1 CONNECT tunnels; `AdminHandler` serves `GET /healthz`
on a separate listener. Call `Close` during shutdown to cancel outbound requests
and close tunnels: `http.Server.Shutdown` does not close hijacked connections.

Only hostname destinations are allowed; literal IPv4 and IPv6 addresses are
rejected with 403. CONNECT requires an explicit port. Hostnames resolving to
loopback, private, or other IPs are intentionally allowed. This is not an SSRF
protection boundary. Outbound connections ignore environment proxy settings.

## Destination allow list

Run `go run ./cmd/forwardproxy -allow-list ./allowed-hosts.txt` to restrict
HTTP and CONNECT destinations. For example:

```text
# Allow example.com only on these ports
example.com:80,443
# Explicitly allow all ports
api.example.com:*
```

The file is loaded once at startup. Each line must contain `hostname:ports`;
surrounding whitespace, blank lines, and full-line `#` comments are accepted.
Matching ignores hostname case and a trailing dot. Specify one numeric port or a
comma-separated list of numeric ports (1–65535), such as `example.com:80,443`.
Use `hostname:*` to explicitly allow all ports. Bare hostnames, empty port-list
items, whitespace within a port list, and mixing `*` with numeric ports are invalid.
HTTP URLs without an explicit port use port 80; CONNECT requires an explicit
port. Ports match numerically, so leading zeros do not affect matching. Entries
do not include subdomains automatically. URLs, hostname wildcards, and IP literals are
invalid entries. An unreadable or invalid file prevents startup; an empty file
denies all destinations. Requests to unlisted hosts receive 403. Without the flag,
all valid hostname destinations remain allowed.

Hostname validation runs before allow-list evaluation. Invalid hostnames and IP
literals are rejected without producing allow-list miss logs.
Every allow-list miss for a valid hostname is logged at warning level with the destination hostname,
port, request method, client address, and evaluation mode, whether or not it is blocked.
To evaluate a list without enforcing it, run:

```sh
go run ./cmd/forwardproxy -allow-list ./allowed-hosts.txt -evaluate
```

`-evaluate` requires `-allow-list`. It allows unlisted hostnames through but still
rejects IP literals and invalid hostnames. File validation still applies.

Library callers can use `NewWithAllowListFile(path)` instead of `New()`, adding
`WithEvaluateAllowList()` to enable evaluation mode.

There is no authentication or TLS on the listeners. Keep both listeners private;
do not expose this proxy to untrusted clients. HTTP upgrades are unsupported;
use CONNECT for tunnels instead.
