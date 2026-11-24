package chatgpt

import (
	"context"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

// These literal fixtures pin the supplied Pi wire contract, rather than deriving
// expected endpoints/client IDs/redirect URIs from production constants. The
// Codex loopback URI is also used by official Codex source (see README).
// They are protocol regression tests, not proof of live provider acceptance.
func TestOAuthWireFixtures(t *testing.T) {
	for _, p := range []Provider{OpenAI, Codex} {
		t.Run(string(p), func(t *testing.T) {
			var fixture struct {
				AuthorizeURL  string     `json:"authorize_url"`
				TokenURL      string     `json:"token_url"`
				Authorization url.Values `json:"authorization"`
				Exchange      url.Values `json:"exchange"`
				Refresh       url.Values `json:"refresh"`
			}
			b, err := os.ReadFile(filepath.Join("testdata", string(p)+".json"))
			if err != nil {
				t.Fatal(err)
			}
			if err := json.Unmarshal(b, &fixture); err != nil {
				t.Fatal(err)
			}
			login, err := BeginLogin(p, "stable-host-id")
			if err != nil {
				t.Fatal(err)
			}
			u, err := url.Parse(login.URL)
			if err != nil {
				t.Fatal(err)
			}
			actual := u.Query()
			if actual.Get("state") == "" || actual.Get("code_challenge") == "" {
				t.Fatal("missing random PKCE parameters")
			}
			fixture.Authorization.Set("state", login.State)
			hash := sha256.Sum256([]byte(login.verifier))
			fixture.Authorization.Set("code_challenge", base64.RawURLEncoding.EncodeToString(hash[:]))
			if p == OpenAI {
				if actual.Get("nonce") == "" {
					t.Fatal("missing nonce")
				}
				fixture.Authorization.Set("nonce", actual.Get("nonce"))
			}
			if !reflect.DeepEqual(actual, fixture.Authorization) {
				t.Errorf("authorization query mismatch: got %v want %v", actual, fixture.Authorization)
			}
			u.RawQuery = ""
			if u.String() != fixture.AuthorizeURL {
				t.Errorf("authorization endpoint = %s", u)
			}
			fixture.Exchange.Set("code_verifier", login.verifier)
			calls := 0
			client := &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
				calls++
				want := fixture.Exchange
				if calls == 2 {
					want = fixture.Refresh
				}
				if r.URL.String() != fixture.TokenURL || r.Method != "POST" || r.Header.Get("Content-Type") != "application/x-www-form-urlencoded" {
					t.Error("wrong token request endpoint/method/content type")
				}
				if err := r.ParseForm(); err != nil {
					t.Fatal(err)
				}
				if !reflect.DeepEqual(r.PostForm, want) {
					t.Errorf("grant form mismatch: got %v want %v", r.PostForm, want)
				}
				if calls == 1 {
					return response(200, `{"access_token":"first-access","refresh_token":"first-refresh","expires_in":3600}`), nil
				}
				return response(200, `{"access_token":"next-access","refresh_token":"next-refresh","expires_in":3600}`), nil
			})}
			credential, err := login.Exchange(context.Background(), client, "http://127.0.0.1:1455/auth/callback?state="+login.State+"&code=auth-code&client_id=issued-client")
			if err != nil {
				t.Fatal(err)
			}
			credential.Expires = time.Time{}
			manager := &Manager{Path: filepath.Join(t.TempDir(), "auth.json"), Store: &Store{Providers: map[Provider]Credential{p: credential}}, Client: client}
			if token, err := manager.Token(context.Background(), p); err != nil || token != "next-access" {
				t.Fatalf("refresh failed: %s %v", token, err)
			}
			if calls != 2 {
				t.Fatalf("token calls = %d", calls)
			}
		})
	}
}
