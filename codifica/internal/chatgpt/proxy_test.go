package chatgpt

import (
	"encoding/base64"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestFirstAuthenticatedRequest(t *testing.T) {
	for _, p := range []Provider{OpenAI, Codex} {
		t.Run(string(p), func(t *testing.T) {
			token := "header." + base64.RawURLEncoding.EncodeToString([]byte(`{"https://api.openai.com/auth":{"chatgpt_account_id":"account-123"}}`)) + ".signature"
			manager := &Manager{Store: &Store{Providers: map[Provider]Credential{p: {Access: token, Refresh: "refresh", ClientID: "client", Expires: time.Now().Add(time.Hour)}}}}
			called := false
			client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
				called = true
				expected := "https://api.openai.com/v1/responses"
				if p == Codex {
					expected = "https://chatgpt.com/backend-api/codex/responses"
				}
				if r.URL.String() != expected || r.Header.Get("Authorization") != "Bearer "+token {
					t.Error("wrong endpoint or bearer token")
				}
				if r.Header.Get("Cookie") != "" || r.Header.Get("X-Untrusted") != "" {
					t.Error("forwarded caller headers")
				}
				var body map[string]any
				if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
					t.Fatal(err)
				}
				input, _ := json.Marshal(body["input"])
				if body["model"] != "test-model" || string(input) != `[{"content":[{"text":"Hello","type":"input_text"}],"role":"user","type":"message"}]` {
					t.Error("lost request input")
				}
				if r.Header.Get("Accept") != "text/event-stream" {
					t.Error("streaming request did not advertise SSE")
				}
				if p == Codex {
					if r.Header.Get("chatgpt-account-id") != "account-123" || body["stream"] != true || body["store"] != false || body["instructions"] != "" {
						t.Error("missing Codex request requirements")
					}
				} else if r.Header.Get("chatgpt-account-id") != "" {
					t.Error("direct flow received Codex header")
				}
				resp := response(200, "data: {\"type\":\"response.completed\"}\n\n")
				resp.Header.Set("Content-Type", "text/event-stream")
				resp.Header.Set("Set-Cookie", "secret=value")
				return resp, nil
			})}
			h := NewHandler(manager, p, client)
			req := httptest.NewRequest("POST", "/v1/responses", strings.NewReader(`{"model":"test-model","stream":true,"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}]}`))
			req.Header.Set("Content-Type", "application/json")
			req.Header.Set("Authorization", "Bearer caller-token")
			req.Header.Set("Cookie", "untrusted=value")
			req.Header.Set("X-Untrusted", "value")
			w := httptest.NewRecorder()
			h.ServeHTTP(w, req)
			if !called || w.Code != 200 || !w.Flushed || !strings.Contains(w.Body.String(), "response.completed") || w.Header().Get("Set-Cookie") != "" {
				t.Fatalf("bad streaming response: %d %s", w.Code, w.Body)
			}
		})
	}
}

func TestProxyRejectsInvalidRequests(t *testing.T) {
	client := &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) { t.Fatal("unexpected upstream call"); return nil, nil })}
	h := NewHandler(&Manager{Store: &Store{Providers: map[Provider]Credential{}}}, Codex, client)
	for _, tc := range []struct {
		method, path, body, contentType, origin string
		status                                  int
	}{
		{"POST", "/v1/responses", `{}`, "application/json", "https://evil.example", 403},
		{"POST", "/v1/responses", `{}`, "text/plain", "", 415},
		{"POST", "/v1/responses", `null`, "application/json", "", 400},
		{"POST", "/v1/responses", `broken`, "application/json", "", 400},
		{"POST", "/v1/responses", `{}`, "application/json", "", 503},
		{"POST", "/v1/chat/completions", `{}`, "application/json", "", 404},
	} {
		r := httptest.NewRequest(tc.method, tc.path, strings.NewReader(tc.body))
		r.Header.Set("Content-Type", tc.contentType)
		r.Header.Set("Origin", tc.origin)
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		if w.Code != tc.status {
			t.Errorf("%s %s: got %d, want %d", tc.method, tc.path, w.Code, tc.status)
		}
	}
	w := httptest.NewRecorder()
	h.ServeHTTP(w, httptest.NewRequest("GET", "/healthz", nil))
	b, _ := io.ReadAll(w.Result().Body)
	if w.Code != 200 || string(b) != "ok\n" {
		t.Fatal("health failed")
	}
}
