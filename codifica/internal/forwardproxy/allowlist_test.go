package forwardproxy

import (
	"bytes"
	"context"
	"errors"
	"log/slog"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func allowListFile(t *testing.T, contents string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "allow-list.txt")
	if err := os.WriteFile(path, []byte(contents), 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestAllowListEvaluationAndLogging(t *testing.T) {
	var logs bytes.Buffer
	previous := slog.Default()
	slog.SetDefault(slog.New(slog.NewTextHandler(&logs, nil)))
	t.Cleanup(func() { slog.SetDefault(previous) })

	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	}))
	defer upstream.Close()
	target := strings.Replace(upstream.URL, "127.0.0.1", "localhost", 1)

	for _, tc := range []struct {
		name     string
		list     string
		evaluate bool
		wantLog  bool
	}{
		{"enforce miss", "example.com:*", false, true},
		{"evaluate miss", "example.com:*", true, true},
		{"enforce match", "localhost:*", false, false},
		{"evaluate match", "localhost:*", true, false},
		{"evaluate empty", "", true, true},
		{"enforce port match", strings.TrimPrefix(target, "http://"), false, false},
		{"evaluate port match", strings.TrimPrefix(target, "http://"), true, false},
		{"enforce port miss", "localhost:1", false, true},
		{"evaluate port miss", "localhost:1", true, true},
		{"enforce port list match", strings.TrimPrefix(target, "http://") + ",1", false, false},
		{"evaluate port list match", strings.TrimPrefix(target, "http://") + ",1", true, false},
		{"enforce port list miss", "localhost:1,2", false, true},
		{"evaluate port list miss", "localhost:1,2", true, true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			var options []AllowListOption
			if tc.evaluate {
				options = append(options, WithEvaluateAllowList())
			}
			p, err := NewWithAllowListFile(allowListFile(t, tc.list), options...)
			if err != nil {
				t.Fatal(err)
			}
			defer p.Close()
			for _, method := range []string{http.MethodGet, http.MethodConnect} {
				logs.Reset()
				requestTarget := target
				want := http.StatusNoContent
				if method == http.MethodConnect {
					requestTarget = strings.TrimPrefix(target, "http://")
					// A recorder cannot hijack; reaching this response proves
					// CONNECT passed allow-list enforcement without dialing.
					want = http.StatusNotImplemented
				}
				if tc.wantLog && !tc.evaluate {
					want = http.StatusForbidden
				}
				w := httptest.NewRecorder()
				p.Handler().ServeHTTP(w, httptest.NewRequest(method, requestTarget, nil))
				if w.Code != want {
					t.Errorf("%s: got %d, want %d", method, w.Code, want)
				}
				count := strings.Count(logs.String(), "destination is not in the allow list")
				if (tc.wantLog && count != 1) || (!tc.wantLog && count != 0) {
					t.Errorf("%s: unexpected logs: %s", method, logs.String())
				}
				if tc.wantLog && (!strings.Contains(logs.String(), "host=localhost") || !strings.Contains(logs.String(), "method="+method) || !strings.Contains(logs.String(), "port="+strings.Split(target, ":")[2])) {
					t.Errorf("missing request details: %s", logs.String())
				}
			}
			// Evaluation never bypasses the existing IP-literal restriction.
			w := httptest.NewRecorder()
			p.Handler().ServeHTTP(w, httptest.NewRequest("GET", "http://127.0.0.1/", nil))
			if w.Code != http.StatusForbidden {
				t.Errorf("IP literal: got %d", w.Code)
			}
		})
	}
}

func TestAllowListFile(t *testing.T) {
	p, err := NewWithAllowListFile(allowListFile(t, "# destinations\r\n\r\n EXAMPLE.com.:* \r\nexample.com:*\nlocalhost:*\n"))
	if err != nil {
		t.Fatal(err)
	}
	defer p.Close()
	if len(p.allowList) != 2 {
		t.Fatalf("expected two unique hosts, got %v", p.allowList)
	}
	for _, host := range []string{"example.com", "EXAMPLE.COM", "Example.Com."} {
		if _, ok := p.allowList[allowListKey(host, "*")]; !ok {
			t.Errorf("host %q not matched", host)
		}
	}
}

func TestInvalidAllowListFile(t *testing.T) {
	for _, host := range []string{"example.com", "example.com:80,", "example.com:,443", "example.com:80,,443", "example.com:*,443", "example.com:80,*", "example.com:80,65536", "example.com:80,https", "example.com:80, 443", "127.0.0.1", "::1", "*.example.com:*", "https://example.com", "example.com:", "example.com:0", "example.com:65536", "example.com:https", "example.com:443:80", "127.0.0.1:80", "[::1]:443", "bad host", "bad..test", "example.com # comment", strings.Repeat("a", 70000)} {
		t.Run(host[:min(len(host), 40)], func(t *testing.T) {
			p, err := NewWithAllowListFile(allowListFile(t, "example.com:80\n"+host+"\n"))
			if err == nil {
				p.Close()
				t.Fatal("invalid allow list accepted")
			}
			if p != nil {
				t.Fatal("proxy returned on error")
			}
		})
	}
	if _, err := NewWithAllowListFile(filepath.Join(t.TempDir(), "missing")); err == nil {
		t.Fatal("missing file accepted")
	}
}

func TestAllowListPorts(t *testing.T) {
	p, err := NewWithAllowListFile(allowListFile(t, "EXAMPLE.com.:00080,443,443\nexample.com:8081\nother.test:*\n"))
	if err != nil {
		t.Fatal(err)
	}
	defer p.Close()
	// An allowed HTTP request reaches the transport, without making a network call.
	p.transport.DialContext = func(context.Context, string, string) (net.Conn, error) {
		return nil, errors.New("test dial")
	}
	for _, tc := range []struct {
		method string
		target string
		status int
	}{
		{"GET", "http://example.com/", 502},
		{"GET", "http://EXAMPLE.COM.:80/", 502},
		{"GET", "http://example.com:00080/", 502},
		{"GET", "http://example.com:443/", 502},
		{"GET", "http://example.com:8080/", 403},
		{"GET", "http://example.com:8081/", 502},
		{"GET", "http://other.test:8080/", 502},
		{"CONNECT", "example.com:443", 501},
		{"CONNECT", "EXAMPLE.COM.:00443", 501},
		{"CONNECT", "example.com:80", 501},
		{"CONNECT", "example.com:8443", 403},
		{"CONNECT", "sub.example.com:443", 403},
		{"CONNECT", "other.test:8443", 501},
	} {
		w := httptest.NewRecorder()
		p.Handler().ServeHTTP(w, httptest.NewRequest(tc.method, tc.target, nil))
		if w.Code != tc.status {
			t.Errorf("%s %s: got %d, want %d", tc.method, tc.target, w.Code, tc.status)
		}
	}
}

func TestAllowListRejectsDestinations(t *testing.T) {
	for _, contents := range []string{"example.com:*\n", "# deny all\n"} {
		p, err := NewWithAllowListFile(allowListFile(t, contents))
		if err != nil {
			t.Fatal(err)
		}
		defer p.Close()
		for _, host := range []string{"other.test", "sub.example.com", "example.com.evil.test", "127.0.0.1"} {
			for _, method := range []string{http.MethodGet, http.MethodConnect} {
				target := "http://" + host + "/"
				if method == http.MethodConnect {
					target = host + ":443"
				}
				w := httptest.NewRecorder()
				p.Handler().ServeHTTP(w, httptest.NewRequest(method, target, nil))
				if w.Code != http.StatusForbidden {
					t.Errorf("%s %s: got %d, want 403", method, target, w.Code)
				}
			}
		}
		if len(p.allowList) == 0 {
			w := httptest.NewRecorder()
			p.Handler().ServeHTTP(w, httptest.NewRequest("GET", "http://example.com/", nil))
			if w.Code != http.StatusForbidden {
				t.Errorf("empty list: got %d, want 403", w.Code)
			}
		}
	}
}
