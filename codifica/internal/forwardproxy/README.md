# Forward proxy

`New` constructs shared proxy state. `Handler` forwards absolute HTTP URLs and
supports HTTPS through HTTP/1 CONNECT tunnels; `AdminHandler` serves `GET /healthz`
on a separate listener. Call `Close` during shutdown to cancel outbound requests
and close tunnels: `http.Server.Shutdown` does not close hijacked connections.

Only hostname destinations are allowed; literal IPv4 and IPv6 addresses are
rejected with 403. CONNECT requires an explicit port. Hostnames resolving to
loopback, private, or other IPs are intentionally allowed. This is not an SSRF
protection boundary. Outbound connections ignore environment proxy settings.

There is no authentication or TLS on the listeners. Keep both listeners private;
do not expose this proxy to untrusted clients. HTTP upgrades are unsupported;
use CONNECT for tunnels instead.
