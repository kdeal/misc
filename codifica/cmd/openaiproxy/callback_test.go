//go:build linux

package main

import (
	"context"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/kdeal/misc/codifica/internal/chatgpt"
)

type gatedListener struct {
	net.Listener
	writing chan struct{}
	release chan struct{}
	once    sync.Once
}

func (l *gatedListener) Accept() (net.Conn, error) {
	c, err := l.Listener.Accept()
	if err != nil {
		return nil, err
	}
	return &gatedConn{Conn: c, listener: l}, nil
}

type gatedConn struct {
	net.Conn
	listener *gatedListener
}

func (c *gatedConn) Write(b []byte) (int, error) {
	c.listener.once.Do(func() { close(c.listener.writing); <-c.listener.release })
	return c.Conn.Write(b)
}

func TestCallbackWaitsForBrowserAcknowledgment(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	gated := &gatedListener{Listener: listener, writing: make(chan struct{}), release: make(chan struct{})}
	defer func() {
		select {
		case <-gated.release:
		default:
			close(gated.release)
		}
	}()
	login, err := chatgpt.BeginLogin(chatgpt.Codex, "")
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	type result struct {
		url string
		err error
	}
	done := make(chan result, 1)
	go func() { u, err := serveCallback(ctx, login, gated); done <- result{u, err} }()
	browserDone := make(chan error, 1)
	query := "?state=" + login.State + "&code=auth-code"
	go func() {
		client := &http.Client{Timeout: 3 * time.Second}
		req, err := http.NewRequest("GET", "http://"+listener.Addr().String()+"/auth/callback"+query, nil)
		if err != nil {
			browserDone <- err
			return
		}
		req.Host = "127.0.0.1:1455"
		resp, err := client.Do(req)
		if err != nil {
			browserDone <- err
			return
		}
		defer resp.Body.Close()
		b, err := io.ReadAll(resp.Body)
		if err == nil && (resp.StatusCode != 200 || !strings.Contains(string(b), "Authorization received.")) {
			t.Error("missing browser acknowledgment")
		}
		browserDone <- err
	}()
	select {
	case <-gated.writing:
	case <-time.After(3 * time.Second):
		t.Fatal("browser response did not begin")
	}
	// Force the acknowledgment to remain in-flight after the callback is received.
	select {
	case r := <-done:
		t.Fatalf("returned before browser write completed: %+v", r)
	case <-time.After(20 * time.Millisecond):
	}
	close(gated.release)
	select {
	case r := <-done:
		if r.err != nil || r.url != "http://127.0.0.1:1455/auth/callback"+query {
			t.Fatalf("callback result: %+v", r)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("callback shutdown hung")
	}
	if err := <-browserDone; err != nil {
		t.Fatalf("browser acknowledgment truncated: %v", err)
	}
}

func TestCallbackRejectsBadStateWithoutConsumingLogin(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	login, err := chatgpt.BeginLogin(chatgpt.Codex, "")
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	done := make(chan error, 1)
	go func() { _, err := serveCallback(ctx, login, listener); done <- err }()
	client := &http.Client{Timeout: 3 * time.Second}
	head, err := http.NewRequest("HEAD", "http://"+listener.Addr().String()+"/auth/callback?state="+login.State+"&code=auth-code", nil)
	if err != nil {
		t.Fatal(err)
	}
	head.Host = "127.0.0.1:1455"
	probe, err := client.Do(head)
	if err != nil {
		t.Fatal(err)
	}
	probe.Body.Close()
	if probe.StatusCode != http.StatusMethodNotAllowed || probe.Header.Get("Allow") != "GET" {
		t.Fatal("HEAD probe was not rejected without consuming callback")
	}
	for _, tc := range []struct {
		query  string
		host   string
		status int
	}{
		{"?state=wrong&code=auth-code", "127.0.0.1:1455", 400},
		{"?state=" + login.State + "&state=other&code=auth-code", "127.0.0.1:1455", 400},
		{"?state=" + login.State, "127.0.0.1:1455", 400},
		{"?state=" + login.State + "&code=auth-code", "localhost:1455", 400},
		{"?state=" + login.State + "&code=auth-code", "evil.example:1455", 400},
		{"?state=" + login.State + "&error=", "127.0.0.1:1455", 400},
		{"?state=" + login.State + "&error=denied&code=auth-code", "127.0.0.1:1455", 400},
		{"?state=" + login.State + "&code=auth-code", "127.0.0.1:1455", 200},
	} {
		req, err := http.NewRequest("GET", "http://"+listener.Addr().String()+"/auth/callback"+tc.query, nil)
		if err != nil {
			t.Fatal(err)
		}
		req.Host = tc.host
		resp, err := client.Do(req)
		if err != nil {
			t.Fatal(err)
		}
		_, _ = io.Copy(io.Discard, resp.Body)
		resp.Body.Close()
		if resp.StatusCode != tc.status {
			t.Fatalf("callback status = %d, want %d", resp.StatusCode, tc.status)
		}
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
}
