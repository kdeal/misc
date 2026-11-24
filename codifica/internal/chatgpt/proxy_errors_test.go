package chatgpt

import (
	"context"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func directManager() *Manager {
	return &Manager{Store: &Store{Providers: map[Provider]Credential{OpenAI: {Access: "token", Refresh: "refresh", ClientID: "client", Expires: time.Now().Add(time.Hour)}}}}
}

func TestDirectNonStreamingJSON(t *testing.T) {
	for _, body := range []string{`{"model":"test","input":"Hello"}`, `{"model":"test","input":"Hello","stream":false}`} {
		client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
			if r.Header.Get("Accept") != "application/json" {
				t.Error("non-streaming request advertised SSE")
			}
			got, err := io.ReadAll(r.Body)
			if err != nil || string(got) != body {
				t.Error("direct body was changed")
			}
			resp := response(200, `{"id":"response-1","output":[]}`)
			resp.Header.Set("Content-Type", "application/json")
			return resp, nil
		})}
		req := httptest.NewRequest("POST", "/v1/responses", strings.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
		w := httptest.NewRecorder()
		NewHandler(directManager(), OpenAI, client).ServeHTTP(w, req)
		if w.Code != 200 || w.Header().Get("Content-Type") != "application/json" || w.Body.String() != `{"id":"response-1","output":[]}` {
			t.Fatalf("bad JSON response: %d %s", w.Code, w.Body)
		}
	}
}

func TestModelRequestRetainsCallerCancellation(t *testing.T) {
	started := make(chan struct{})
	canceled := make(chan struct{})
	client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
		close(started)
		<-r.Context().Done()
		close(canceled)
		return nil, r.Context().Err()
	})}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	req := httptest.NewRequest("POST", "/v1/responses", strings.NewReader(`{"model":"test","input":"Hello"}`)).WithContext(ctx)
	req.Header.Set("Content-Type", "application/json")
	done := make(chan struct{})
	go func() {
		defer close(done)
		NewHandler(directManager(), OpenAI, client).ServeHTTP(httptest.NewRecorder(), req)
	}()
	select {
	case <-started:
	case <-time.After(3 * time.Second):
		t.Fatal("model request did not start")
	}
	cancel()
	select {
	case <-canceled:
	case <-time.After(3 * time.Second):
		t.Fatal("caller cancellation did not reach model request")
	}
	select {
	case <-done:
	case <-time.After(3 * time.Second):
		t.Fatal("handler did not finish")
	}
}

type errorReader struct{}

func (errorReader) Read([]byte) (int, error) { return 0, io.ErrUnexpectedEOF }

func TestBrokenUpstreamAbortsDownstream(t *testing.T) {
	for _, mediaType := range []string{"text/event-stream", "application/json"} {
		t.Run(mediaType, func(t *testing.T) {
			partial := strings.Repeat("partial response\n", 1024)
			client := &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) {
				resp := response(200, "")
				resp.Header.Set("Content-Type", mediaType)
				resp.Body = io.NopCloser(io.MultiReader(strings.NewReader(partial), errorReader{}))
				return resp, nil
			})}
			server := httptest.NewServer(NewHandler(directManager(), OpenAI, client))
			defer server.Close()
			ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
			defer cancel()
			req, err := http.NewRequestWithContext(ctx, "POST", server.URL+"/v1/responses", strings.NewReader(`{"model":"test","input":"Hello"}`))
			if err != nil {
				t.Fatal(err)
			}
			req.Header.Set("Content-Type", "application/json")
			resp, err := server.Client().Do(req)
			if err != nil {
				t.Fatalf("expected committed headers and partial body before abort: %v", err)
			}
			defer resp.Body.Close()
			b, err := io.ReadAll(resp.Body)
			if err == nil {
				t.Fatalf("truncated upstream became a successful EOF (%d bytes)", len(b))
			}
			if len(b) == 0 || !strings.HasPrefix(partial, string(b)) {
				t.Fatalf("unexpected partial response: %q (%v)", b, err)
			}
		})
	}
}
