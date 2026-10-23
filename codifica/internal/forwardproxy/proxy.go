// Package forwardproxy provides an outbound HTTP proxy and a separate admin handler.
package forwardproxy

import (
	"context"
	"io"
	"net"
	"net/http"
	"net/http/httputil"
	"net/netip"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Proxy allows hostname destinations, including names resolving to private IPs.
// It provides no authentication and must not be exposed to untrusted clients.
type Proxy struct {
	transport *http.Transport
	forward   *httputil.ReverseProxy
	ctx       context.Context
	cancel    context.CancelFunc
	mu        sync.Mutex
	closed    bool
	tunnels   map[net.Conn]net.Conn
}

func New() *Proxy {
	ctx, cancel := context.WithCancel(context.Background())
	transport := http.DefaultTransport.(*http.Transport).Clone()
	// Dial destinations directly, ignoring environment proxy settings.
	transport.Proxy = nil
	p := &Proxy{transport: transport, ctx: ctx, cancel: cancel, tunnels: make(map[net.Conn]net.Conn)}
	p.forward = &httputil.ReverseProxy{
		Transport: transport,
		Rewrite: func(r *httputil.ProxyRequest) {
			r.Out.Host = r.Out.URL.Host
			r.Out.Header.Del("Proxy-Authorization")
			r.Out.Header.Del("Proxy-Connection")
		},
		ModifyResponse: func(r *http.Response) error {
			r.Header.Del("Proxy-Authenticate")
			return nil
		},
		ErrorHandler: func(w http.ResponseWriter, r *http.Request, err error) {
			http.Error(w, "upstream unavailable", http.StatusBadGateway)
		},
	}
	return p
}

// Handler serves forward-proxy requests, not admin routes.
func (p *Proxy) Handler() http.Handler {
	return http.HandlerFunc(p.serveHTTP)
}

func (p *Proxy) AdminHandler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		if p.ctx.Err() != nil {
			http.Error(w, "proxy stopped", http.StatusServiceUnavailable)
			return
		}
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		_, _ = io.WriteString(w, "ok\n")
	})
	return mux
}

func (p *Proxy) serveHTTP(w http.ResponseWriter, r *http.Request) {
	if p.ctx.Err() != nil {
		http.Error(w, "proxy stopped", http.StatusServiceUnavailable)
		return
	}
	host := r.URL.Hostname()
	if r.Method == http.MethodConnect {
		var port string
		var err error
		host, port, err = net.SplitHostPort(r.Host)
		if err != nil || !validPort(port) {
			http.Error(w, "CONNECT requires host:port", http.StatusBadRequest)
			return
		}
	} else if r.URL.Scheme != "http" || r.URL.Host == "" || r.URL.User != nil || (r.URL.Port() != "" && !validPort(r.URL.Port())) {
		http.Error(w, "expected an absolute HTTP URL", http.StatusBadRequest)
		return
	}
	if !allowedHost(host) {
		http.Error(w, "only hostname destinations are allowed", http.StatusForbidden)
		return
	}
	ctx, cancel := context.WithCancel(r.Context())
	stop := context.AfterFunc(p.ctx, cancel)
	defer stop()
	defer cancel()
	r = r.WithContext(ctx)
	if r.Method == http.MethodConnect {
		p.connect(w, r)
		return
	}
	// Upgraded connections would escape normal HTTP server shutdown.
	if r.Header.Get("Upgrade") != "" {
		http.Error(w, "use CONNECT for tunnels", http.StatusBadRequest)
		return
	}
	p.forward.ServeHTTP(w, r)
}

func validPort(port string) bool {
	n, err := strconv.Atoi(port)
	return err == nil && n > 0 && n <= 65535
}

func allowedHost(host string) bool {
	host = strings.TrimSuffix(host, ".")
	if host == "" {
		return false
	}
	if _, err := netip.ParseAddr(host); err == nil {
		return false
	}
	// Reject malformed names as well as scoped/bracketed IP literals.
	if len(host) > 253 {
		return false
	}
	for _, label := range strings.Split(host, ".") {
		if len(label) == 0 || len(label) > 63 || label[0] == '-' || label[len(label)-1] == '-' {
			return false
		}
		for _, c := range label {
			if !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || c == '-') {
				return false
			}
		}
	}
	return true
}

func (p *Proxy) connect(w http.ResponseWriter, r *http.Request) {
	hijacker, ok := w.(http.Hijacker)
	if !ok {
		http.Error(w, "CONNECT requires HTTP/1", http.StatusNotImplemented)
		return
	}
	upstream, err := (&net.Dialer{Timeout: 10 * time.Second}).DialContext(r.Context(), "tcp", r.Host)
	if err != nil {
		http.Error(w, "upstream unavailable", http.StatusBadGateway)
		return
	}
	defer upstream.Close()
	client, buffered, err := hijacker.Hijack()
	if err != nil {
		return
	}
	defer client.Close()
	p.mu.Lock()
	if p.closed {
		p.mu.Unlock()
		return
	}
	p.tunnels[client] = upstream
	p.mu.Unlock()
	defer func() {
		p.mu.Lock()
		delete(p.tunnels, client)
		p.mu.Unlock()
	}()
	if _, err := buffered.WriteString("HTTP/1.1 200 Connection Established\r\n\r\n"); err != nil {
		return
	}
	if err := buffered.Flush(); err != nil {
		return
	}
	done := make(chan struct{})
	go func() {
		// Include data the HTTP server already buffered after the CONNECT headers.
		_, _ = io.Copy(upstream, buffered)
		_ = upstream.(*net.TCPConn).CloseWrite()
		close(done)
	}()
	_, _ = io.Copy(client, upstream)
	_ = client.Close()
	_ = upstream.Close()
	<-done
}

// Close stops outbound requests and closes hijacked tunnels, which are not
// managed by http.Server.Shutdown. It is safe to call more than once.
func (p *Proxy) Close() error {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.closed = true
	p.cancel()
	for client, upstream := range p.tunnels {
		_ = client.Close()
		_ = upstream.Close()
	}
	p.transport.CloseIdleConnections()
	return nil
}
