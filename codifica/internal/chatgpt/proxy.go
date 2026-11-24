package chatgpt

import (
	"bytes"
	"encoding/json"
	"io"
	"mime"
	"net/http"
	"strings"
)

// NewHandler exposes only the Responses endpoint, never a general-purpose
// authenticated proxy. Keep this unauthenticated listener on loopback.
func NewHandler(manager *Manager, provider Provider, client *http.Client) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) { _, _ = io.WriteString(w, "ok\n") })
	mux.HandleFunc("POST /v1/responses", func(w http.ResponseWriter, r *http.Request) {
		// Block browser cross-origin requests to the local credential-bearing proxy.
		if r.Header.Get("Origin") != "" {
			http.Error(w, "browser requests are not allowed", http.StatusForbidden)
			return
		}
		mediaType, _, err := mime.ParseMediaType(r.Header.Get("Content-Type"))
		if err != nil || mediaType != "application/json" {
			http.Error(w, "Content-Type must be application/json", http.StatusUnsupportedMediaType)
			return
		}
		body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 8<<20))
		if err != nil {
			http.Error(w, "request body exceeds limit or could not be read", http.StatusRequestEntityTooLarge)
			return
		}
		var params map[string]json.RawMessage
		if json.Unmarshal(body, &params) != nil || params == nil {
			http.Error(w, "expected a JSON object", http.StatusBadRequest)
			return
		}
		endpoint := "https://api.openai.com/v1/responses"
		if provider == Codex {
			endpoint = "https://chatgpt.com/backend-api/codex/responses"
			// Codex is streaming-only and does not support stored responses.
			params["store"] = json.RawMessage(`false`)
			params["stream"] = json.RawMessage(`true`)
			if _, ok := params["instructions"]; !ok {
				params["instructions"] = json.RawMessage(`""`)
			}
			body, _ = json.Marshal(params)
		}
		token, err := manager.Token(r.Context(), provider)
		if err != nil {
			http.Error(w, "credentials unavailable; check sign-in or refresh credentials", http.StatusServiceUnavailable)
			return
		}
		req, err := http.NewRequestWithContext(r.Context(), http.MethodPost, endpoint, bytes.NewReader(body))
		if err != nil {
			http.Error(w, "could not build upstream request", http.StatusInternalServerError)
			return
		}
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("Authorization", "Bearer "+token)
		var stream bool
		_ = json.Unmarshal(params["stream"], &stream)
		if stream {
			req.Header.Set("Accept", "text/event-stream")
		} else {
			req.Header.Set("Accept", "application/json")
		}
		if provider == Codex {
			accountID, err := AccountID(token)
			if err != nil {
				http.Error(w, "Codex credentials are missing account metadata; sign in again", http.StatusServiceUnavailable)
				return
			}
			req.Header.Set("chatgpt-account-id", accountID)
		}
		resp, err := client.Do(req)
		if err != nil {
			http.Error(w, "upstream request failed", http.StatusBadGateway)
			return
		}
		defer resp.Body.Close()
		// Deliberately do not forward caller credentials/cookies or upstream cookies.
		for _, name := range []string{"Content-Type", "X-Request-ID", "Retry-After"} {
			if value := resp.Header.Get(name); value != "" {
				w.Header().Set(name, value)
			}
		}
		w.Header().Set("Cache-Control", "no-store")
		w.WriteHeader(resp.StatusCode)
		if strings.HasPrefix(resp.Header.Get("Content-Type"), "text/event-stream") {
			buf := make([]byte, 32*1024)
			for {
				n, readErr := resp.Body.Read(buf)
				if n > 0 {
					if _, err := w.Write(buf[:n]); err != nil {
						panic(http.ErrAbortHandler)
					}
					if err := http.NewResponseController(w).Flush(); err != nil {
						panic(http.ErrAbortHandler)
					}
				}
				if readErr != nil {
					if readErr != io.EOF {
						panic(http.ErrAbortHandler)
					}
					return
				}
			}
		}
		if _, err := io.Copy(w, resp.Body); err != nil {
			// Status is already committed. Abort rather than turn a truncated
			// upstream response into a successful downstream EOF.
			panic(http.ErrAbortHandler)
		}
	})
	return mux
}
