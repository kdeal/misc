# Service proxies (planned)

Implement reverse proxies for GitHub, OpenAI, npm, Cargo, and other upstreams
here. Start with `net/http/httputil.ReverseProxy`, and add provider-specific
subpackages only when routing, authentication, or rewriting actually differs.

Expose `http.Handler` values; listener addresses and process lifecycle belong in
each `cmd/` entry point and `internal/httpserver`. Inject upstream URLs, credentials, and
transports rather than using globals. Support streaming responses and restrict
upstream destinations explicitly.
