package httpserver

import (
	"context"
	"net"
	"net/http"
	"testing"
	"time"
)

func TestRunMultipleListeners(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	started := make(chan string, 2)
	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	})
	servers := []*http.Server{
		New("127.0.0.1:0", handler),
		New("127.0.0.1:0", handler),
	}
	for _, server := range servers {
		server.BaseContext = func(listener net.Listener) context.Context {
			started <- listener.Addr().String()
			return context.Background()
		}
	}
	done := make(chan error, 1)
	go func() { done <- Run(ctx, servers...) }()
	client := &http.Client{Timeout: time.Second}
	defer client.CloseIdleConnections()
	for range servers {
		select {
		case addr := <-started:
			response, err := client.Get("http://" + addr)
			if err != nil {
				t.Fatal(err)
			}
			_ = response.Body.Close()
			if response.StatusCode != http.StatusNoContent {
				t.Fatalf("unexpected status: %d", response.StatusCode)
			}
		case <-time.After(5 * time.Second):
			t.Fatal("server did not start")
		}
	}
	cancel()
	select {
	case err := <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("servers did not stop")
	}
}

func TestRunRejectsInvalidServers(t *testing.T) {
	for _, servers := range [][]*http.Server{nil, {nil}, {New("127.0.0.1:0", nil)}} {
		if err := Run(context.Background(), servers...); err == nil {
			t.Fatal("expected a configuration error")
		}
	}
}

func TestRunBindFailure(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	handler := http.NewServeMux()
	if err := Run(context.Background(), New("127.0.0.1:0", handler), New(listener.Addr().String(), handler)); err == nil {
		t.Fatal("expected an occupied port to fail startup")
	}
}
