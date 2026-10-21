# Forward proxy (planned)

Implement the outbound HTTP proxy here, including CONNECT support when needed.
Expose proxy and admin handlers separately so `cmd/forwardproxy` can place them on
different listeners. Both handlers can share an explicitly constructed proxy
instance rather than package globals.

Keep the admin listener private by default. Authentication, destination policy,
and the lifecycle of tunneled connections must be addressed before enabling the
proxy; `http.Server.Shutdown` does not close hijacked CONNECT connections.
