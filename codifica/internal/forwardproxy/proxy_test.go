package forwardproxy

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"testing"
	"time"
)

func TestAllowedHost(t *testing.T) {
	for _, tc := range []struct {
		name string
		host string
		want bool
	}{
		{"domain", "example.com", true},
		{"single label", "localhost", true},
		{"uppercase with trailing dot", "EXAMPLE.com.", true},
		{"hyphenated label", "my-service.internal", true},
		{"numeric label", "123.example", true},
		{"empty", "", false},
		{"IPv4", "127.0.0.1", false},
		{"IPv4 with trailing dot", "127.0.0.1.", false},
		{"IPv6", "::1", false},
		{"IPv4-mapped IPv6", "::ffff:127.0.0.1", false},
		{"scoped IPv6", "fe80::1%lo", false},
		{"bracketed IPv6", "[::1]", false},
		{"space", "bad host", false},
		{"leading hyphen", "-bad.test", false},
		{"empty label", "bad..test", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if got := allowedHost(tc.host); got != tc.want {
				t.Fatalf("allowedHost(%q) = %t, want %t", tc.host, got, tc.want)
			}
		})
	}
}

func TestHTTPForwarding(t *testing.T) {
	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" || r.URL.RequestURI() != "/hello?q=world" {
			t.Errorf("unexpected request: %s %s", r.Method, r.URL)
		}
		for _, header := range []string{"Proxy-Authorization", "Proxy-Connection", "X-Hop", "Connection"} {
			if r.Header.Get(header) != "" {
				t.Errorf("forwarded %s header", header)
			}
		}
		body, _ := io.ReadAll(r.Body)
		w.Header().Set("X-Upstream", "yes")
		w.Header().Set("Connection", "X-Hop")
		w.Header().Set("X-Hop", "secret")
		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write(body)
	}))
	defer upstream.Close()
	p := New()
	defer p.Close()
	server := httptest.NewServer(p.Handler())
	defer server.Close()
	proxyURL, _ := url.Parse(server.URL)
	transport := &http.Transport{Proxy: http.ProxyURL(proxyURL)}
	defer transport.CloseIdleConnections()
	client := &http.Client{Transport: transport, Timeout: 5 * time.Second}
	target := strings.Replace(upstream.URL, "127.0.0.1", "localhost", 1) + "/hello?q=world"
	req, _ := http.NewRequest(http.MethodPost, target, strings.NewReader("payload"))
	req.Header.Set("Proxy-Authorization", "Basic secret")
	req.Header.Set("Proxy-Connection", "keep-alive")
	req.Header.Set("Connection", "X-Hop")
	req.Header.Set("X-Hop", "secret")
	resp, err := client.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	body, _ := io.ReadAll(resp.Body)
	if resp.StatusCode != http.StatusCreated || string(body) != "payload" || resp.Header.Get("X-Upstream") != "yes" || resp.Header.Get("X-Hop") != "" {
		t.Fatalf("unexpected response: %s, headers %v, body %q", resp.Status, resp.Header, body)
	}
}

func TestRejectedRequests(t *testing.T) {
	p := New()
	defer p.Close()
	for _, tc := range []struct {
		method string
		target string
		status int
	}{
		{"GET", "http://127.0.0.1/", 403},
		{"GET", "http://[::1]/", 403},
		{"GET", "http://127.0.0.1./", 403},
		{"CONNECT", "127.0.0.1:443", 403},
		{"CONNECT", "[::1]:443", 403},
		{"CONNECT", "example.com", 400},
		{"CONNECT", "example.com:0", 400},
		{"CONNECT", "example.com:65536", 400},
		{"GET", "/healthz", 400},
		{"GET", "https://example.com/", 400},
		{"GET", "http://example.com:65536/", 400},
		{"GET", "http://user:secret@example.com/", 400},
	} {
		t.Run(tc.method+" "+tc.target, func(t *testing.T) {
			r := httptest.NewRequest(tc.method, tc.target, nil)
			w := httptest.NewRecorder()
			p.Handler().ServeHTTP(w, r)
			if w.Code != tc.status {
				t.Fatalf("got status %d, want %d", w.Code, tc.status)
			}
		})
	}
}

func TestUpstreamFailure(t *testing.T) {
	p := New()
	defer p.Close()
	// A closed local listener gives a deterministic connection refusal.
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	target := strings.Replace(listener.Addr().String(), "127.0.0.1", "localhost", 1)
	_ = listener.Close()
	for _, method := range []string{"GET", "CONNECT"} {
		t.Run(method, func(t *testing.T) {
			server := httptest.NewServer(p.Handler())
			defer server.Close()
			conn, err := net.DialTimeout("tcp", server.Listener.Addr().String(), time.Second)
			if err != nil {
				t.Fatal(err)
			}
			defer conn.Close()
			_ = conn.SetDeadline(time.Now().Add(5 * time.Second))
			requestTarget := target
			if method != http.MethodConnect {
				requestTarget = "http://" + target + "/"
			}
			_, _ = fmt.Fprintf(conn, "%s %s HTTP/1.1\r\nHost: %s\r\n\r\n", method, requestTarget, target)
			resp, err := http.ReadResponse(bufio.NewReader(conn), &http.Request{Method: method})
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()
			if resp.StatusCode != http.StatusBadGateway {
				t.Fatalf("got %s", resp.Status)
			}
		})
	}
}

func TestConnectAndClose(t *testing.T) {
	upstream, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer upstream.Close()
	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := upstream.Accept()
		if err != nil {
			return
		}
		defer conn.Close()
		_ = conn.SetDeadline(time.Now().Add(5 * time.Second))
		_, _ = io.Copy(conn, conn)
	}()
	p := New()
	defer p.Close()
	server := httptest.NewServer(p.Handler())
	defer server.Close()
	client, err := net.DialTimeout("tcp", server.Listener.Addr().String(), time.Second)
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	_ = client.SetDeadline(time.Now().Add(5 * time.Second))
	target := strings.Replace(upstream.Addr().String(), "127.0.0.1", "localhost", 1)
	// Pipeline tunnel bytes to exercise the hijacked reader's buffered data.
	_, _ = fmt.Fprintf(client, "CONNECT %s HTTP/1.1\r\nHost: %s\r\n\r\nhello", target, target)
	reader := bufio.NewReader(client)
	resp, err := http.ReadResponse(reader, &http.Request{Method: http.MethodConnect})
	if err != nil {
		t.Fatal(err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("CONNECT failed: %s", resp.Status)
	}
	body := make([]byte, 5)
	if _, err := io.ReadFull(reader, body); err != nil || string(body) != "hello" {
		t.Fatalf("tunnel echo = %q, error %v", body, err)
	}
	_ = p.Close()
	if _, err := reader.ReadByte(); err == nil {
		t.Fatal("tunnel remained open after Close")
	}
	select {
	case <-done:
	case <-time.After(5 * time.Second):
		t.Fatal("upstream remained open after Close")
	}
}

func TestHTTPSConnect(t *testing.T) {
	upstream := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = io.WriteString(w, "secure response")
	}))
	defer upstream.Close()
	p := New()
	defer p.Close()
	server := httptest.NewServer(p.Handler())
	defer server.Close()
	proxyURL, _ := url.Parse(server.URL)
	transport := upstream.Client().Transport.(*http.Transport).Clone()
	transport.TLSClientConfig = transport.TLSClientConfig.Clone()
	transport.TLSClientConfig.ServerName = "example.com"
	transport.Proxy = http.ProxyURL(proxyURL)
	defer transport.CloseIdleConnections()
	client := &http.Client{Transport: transport, Timeout: 5 * time.Second}
	resp, err := client.Get(strings.Replace(upstream.URL, "127.0.0.1", "localhost", 1))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	body, _ := io.ReadAll(resp.Body)
	if string(body) != "secure response" {
		t.Fatalf("unexpected HTTPS response %q", body)
	}
}

func TestAdminAndClosedProxy(t *testing.T) {
	for _, tc := range []struct {
		name   string
		admin  bool
		closed bool
		path   string
		status int
	}{
		{"healthy admin", true, false, "/healthz", http.StatusOK},
		{"unknown admin route", true, false, "/", http.StatusNotFound},
		{"closed admin", true, true, "/healthz", http.StatusServiceUnavailable},
		{"closed proxy", false, true, "http://example.com/", http.StatusServiceUnavailable},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p := New()
			defer p.Close()
			if tc.closed {
				_ = p.Close()
			}
			handler := p.Handler()
			if tc.admin {
				handler = p.AdminHandler()
			}
			w := httptest.NewRecorder()
			handler.ServeHTTP(w, httptest.NewRequest("GET", tc.path, nil))
			if w.Code != tc.status {
				t.Fatalf("%s: got %d, want %d", tc.path, w.Code, tc.status)
			}
		})
	}
}
