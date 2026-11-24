package chatgpt

import (
	"context"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

type roundTripFunc func(*http.Request) (*http.Response, error)

func (f roundTripFunc) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }
func response(status int, body string) *http.Response {
	return &http.Response{StatusCode: status, Header: make(http.Header), Body: io.NopCloser(strings.NewReader(body))}
}

func TestLoginFlows(t *testing.T) {
	for _, p := range []Provider{OpenAI, Codex} {
		t.Run(string(p), func(t *testing.T) {
			login, err := BeginLogin(p, "stable-host-id")
			if err != nil {
				t.Fatal(err)
			}
			u, _ := url.Parse(login.URL)
			q := u.Query()
			hash := sha256.Sum256([]byte(login.verifier))
			if q.Get("code_challenge") != base64.RawURLEncoding.EncodeToString(hash[:]) || q.Get("state") != login.State || q.Get("redirect_uri") != RedirectURI {
				t.Fatal("invalid PKCE parameters")
			}
			clientID := codexClientID
			if p == OpenAI {
				clientID = "issued-client"
				if u.Path != "/api/accounts/authorize" || q.Get("client_id") != "dynamic_agent_client" || q.Get("ext_agent_host_id") != "stable-host-id" || !strings.Contains(q.Get("scope"), "chatgpt.tokens.use.direct") {
					t.Fatal("invalid direct authorization parameters")
				}
			} else if u.Path != "/oauth/authorize" || q.Get("client_id") != codexClientID {
				t.Fatal("invalid Codex authorization parameters")
			}
			var calls int
			client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
				calls++
				if r.URL.String() != p.tokenURL() || r.Method != "POST" {
					t.Errorf("wrong token endpoint: %s", r.URL)
				}
				if err := r.ParseForm(); err != nil {
					t.Fatal(err)
				}
				if r.Form.Get("client_id") != clientID || r.Form.Get("grant_type") != "authorization_code" || r.Form.Get("code_verifier") != login.verifier || r.Form.Get("code") != "auth-code" || r.Form.Get("redirect_uri") != RedirectURI {
					t.Errorf("invalid exchange form")
				}
				if (r.Form.Get("resource") == resource) != (p == OpenAI) {
					t.Error("resource is only for direct flow")
				}
				return response(200, `{"access_token":"access","refresh_token":"refresh","expires_in":3600,"scope":"scope"}`), nil
			})}
			for _, callback := range []string{
				RedirectURI + "?code=auth-code&state=wrong&client_id=issued-client",
				"https://evil.example/auth/callback?state=" + login.State,
				RedirectURI + "?state=" + login.State + "&error=access_denied",
				RedirectURI + "?state=" + login.State + "&code=a&code=b",
			} {
				if _, err := login.Exchange(context.Background(), client, callback); err == nil {
					t.Error("accepted invalid callback")
				}
			}
			if calls != 0 {
				t.Fatal("invalid callback reached token endpoint")
			}
			c, err := login.Exchange(context.Background(), client, RedirectURI+"?code=auth-code&state="+login.State+"&client_id=issued-client")
			if err != nil {
				t.Fatal(err)
			}
			if c.ClientID != clientID || c.Access != "access" || c.Refresh != "refresh" || time.Until(c.Expires) < 59*time.Minute {
				t.Fatalf("bad credential: %#v", c)
			}
		})
	}
}

func TestRefreshPersistsAndSerializes(t *testing.T) {
	for _, p := range []Provider{OpenAI, Codex} {
		t.Run(string(p), func(t *testing.T) {
			var calls atomic.Int32
			clientID := codexClientID
			if p == OpenAI {
				clientID = "issued-client"
			}
			m := &Manager{Path: filepath.Join(t.TempDir(), "private", "auth.json"), Store: &Store{Providers: map[Provider]Credential{p: {Access: "expired", Refresh: "old-refresh", ClientID: clientID, Expires: time.Now().Add(-time.Hour)}}}}
			m.Client = &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
				calls.Add(1)
				_ = r.ParseForm()
				if r.URL.String() != p.tokenURL() || r.Form.Get("grant_type") != "refresh_token" || r.Form.Get("refresh_token") != "old-refresh" || r.Form.Get("client_id") != clientID {
					t.Error("wrong refresh endpoint or form")
				}
				if (r.Form.Get("resource") == resource) != (p == OpenAI) {
					t.Error("wrong refresh resource")
				}
				return response(200, `{"access_token":"new-access","refresh_token":"rotated","expires_in":3600}`), nil
			})}
			var wg sync.WaitGroup
			for range 10 {
				wg.Go(func() {
					token, err := m.Token(context.Background(), p)
					if err != nil || token != "new-access" {
						t.Errorf("token: %s, %v", token, err)
					}
				})
			}
			wg.Wait()
			if calls.Load() != 1 {
				t.Fatalf("refresh count = %d", calls.Load())
			}
			saved, err := Load(m.Path)
			if err != nil {
				t.Fatal(err)
			}
			if saved.Providers[p].Refresh != "rotated" || saved.Providers[p].ClientID != clientID {
				t.Fatal("rotation not persisted")
			}
			info, err := os.Stat(m.Path)
			if err != nil {
				t.Fatal(err)
			}
			if info.Mode().Perm() != 0600 {
				t.Fatalf("credential permissions %o", info.Mode().Perm())
			}
		})
	}
}

func TestRefreshFailureDoesNotLeakSecrets(t *testing.T) {
	client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
		return response(400, `{"error":"secret-refresh-token"}`), nil
	})}
	_, err := requestToken(context.Background(), client, Codex, url.Values{}, Credential{})
	if err == nil || strings.Contains(err.Error(), "secret") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestRefreshRetainsTokenWhenNotRotated(t *testing.T) {
	client := &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) {
		return response(200, `{"access_token":"new","expires_in":3600}`), nil
	})}
	c, err := requestToken(context.Background(), client, Codex, url.Values{}, Credential{Refresh: "old", ClientID: codexClientID, Scope: "scope"})
	if err != nil {
		t.Fatal(err)
	}
	if c.Refresh != "old" || c.Scope != "scope" || c.ClientID != codexClientID {
		t.Fatal("lost retained credential fields")
	}
}

func TestPersistenceFailureBlocksUseUntilSaved(t *testing.T) {
	dir := t.TempDir()
	blocker := filepath.Join(dir, "not-a-directory")
	if err := os.WriteFile(blocker, []byte("block"), 0600); err != nil {
		t.Fatal(err)
	}
	calls := 0
	m := &Manager{
		Path:  filepath.Join(blocker, "auth.json"),
		Store: &Store{Providers: map[Provider]Credential{Codex: {Access: "expired", Refresh: "old", ClientID: codexClientID}}},
		Client: &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) {
			calls++
			return response(200, `{"access_token":"new","refresh_token":"rotated","expires_in":3600}`), nil
		})},
	}
	for range 2 {
		if token, err := m.Token(context.Background(), Codex); err == nil || token != "" {
			t.Fatal("used unpersisted credentials")
		}
	}
	m.Path = filepath.Join(dir, "auth.json")
	token, err := m.Token(context.Background(), Codex)
	if err != nil || token != "new" || calls != 1 {
		t.Fatalf("rotation lost: token=%s calls=%d err=%v", token, calls, err)
	}
	saved, err := Load(m.Path)
	if err != nil || saved.Providers[Codex].Refresh != "rotated" {
		t.Fatal("did not persist rotation on retry")
	}
}

func TestCallbackGrantValidation(t *testing.T) {
	for _, p := range []Provider{OpenAI, Codex} {
		t.Run(string(p), func(t *testing.T) {
			login, err := BeginLogin(p, "host")
			if err != nil {
				t.Fatal(err)
			}
			base := "http://127.0.0.1:1455/auth/callback?state=" + login.State
			for _, tc := range []struct {
				suffix string
				valid  bool
			}{
				{"", false},
				{"&code=", false},
				{"&code=a&code=b", false},
				{"&error=denied", true},
				{"&error=", false},
				{"&error=denied&error=other", false},
				{"&error=denied&code=a", false},
				{"&code=a", p == Codex},
				{"&code=a&client_id=issued", true},
			} {
				if err := login.ValidateCallback(base + tc.suffix); (err == nil) != tc.valid {
					t.Errorf("callback %q: valid=%v err=%v", tc.suffix, tc.valid, err)
				}
			}
		})
	}
}

func TestAccountID(t *testing.T) {
	payload := base64.RawURLEncoding.EncodeToString([]byte(`{"https://api.openai.com/auth":{"chatgpt_account_id":"account-123"}}`))
	got, err := AccountID("header." + payload + ".signature")
	if err != nil || got != "account-123" {
		t.Fatalf("%s, %v", got, err)
	}
	for _, token := range []string{"bad", "a.b.c", "a." + base64.RawURLEncoding.EncodeToString([]byte(`{}`)) + ".c"} {
		if _, err := AccountID(token); err == nil {
			t.Error(fmt.Sprintf("accepted %s", token))
		}
	}
}
