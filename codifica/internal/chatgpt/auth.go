// Package chatgpt implements the two distinct ChatGPT subscription OAuth flows.
package chatgpt

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

type Provider string

const (
	OpenAI        Provider = "openai"
	Codex         Provider = "openai-codex"
	codexClientID          = "app_EMoamEEZ73f0CkXaXp7hrann"
	RedirectURI            = "http://127.0.0.1:1455/auth/callback"
	resource               = "https://api.openai.com/v1"
)

func (p Provider) Valid() bool { return p == OpenAI || p == Codex }
func (p Provider) tokenURL() string {
	if p == OpenAI {
		return "https://auth.openai.com/api/accounts/oauth/token"
	}
	return "https://auth.openai.com/oauth/token"
}

type Credential struct {
	Access   string    `json:"access"`
	Refresh  string    `json:"refresh"`
	ClientID string    `json:"client_id"`
	Scope    string    `json:"scope,omitempty"`
	Expires  time.Time `json:"expires"`
}

type Store struct {
	DeviceID  string                  `json:"device_id"`
	Providers map[Provider]Credential `json:"providers"`
}

func Load(path string) (*Store, error) {
	b, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return &Store{Providers: make(map[Provider]Credential)}, nil
	}
	if err != nil {
		return nil, err
	}
	var s Store
	if err := json.Unmarshal(b, &s); err != nil {
		return nil, fmt.Errorf("decode credentials: %w", err)
	}
	if s.Providers == nil {
		s.Providers = make(map[Provider]Credential)
	}
	return &s, nil
}

// Save atomically replaces the credential file; tokens never go in logs or URLs.
func (s *Store) Save(path string) error {
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return err
	}
	b, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return err
	}
	f, err := os.CreateTemp(filepath.Dir(path), ".auth-*")
	if err != nil {
		return err
	}
	defer os.Remove(f.Name())
	if _, err := f.Write(b); err != nil {
		f.Close()
		return err
	}
	if err := f.Sync(); err != nil {
		f.Close()
		return err
	}
	if err := f.Close(); err != nil {
		return err
	}
	return os.Rename(f.Name(), path)
}

func randomValue() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}

// Login holds a single short-lived PKCE transaction. Never reuse one across logins.
type Login struct {
	Provider Provider
	State    string
	verifier string
	URL      string
}

func BeginLogin(p Provider, deviceID string) (*Login, error) {
	if !p.Valid() {
		return nil, errors.New("unknown provider")
	}
	verifier, err := randomValue()
	if err != nil {
		return nil, err
	}
	state, err := randomValue()
	if err != nil {
		return nil, err
	}
	nonce, err := randomValue()
	if err != nil {
		return nil, err
	}
	hash := sha256.Sum256([]byte(verifier))
	q := url.Values{
		"response_type": {"code"}, "redirect_uri": {RedirectURI},
		"state": {state}, "code_challenge": {base64.RawURLEncoding.EncodeToString(hash[:])},
		"code_challenge_method": {"S256"},
	}
	endpoint := "https://auth.openai.com/oauth/authorize"
	if p == OpenAI {
		endpoint = "https://auth.openai.com/api/accounts/authorize"
		q.Set("client_id", "dynamic_agent_client")
		q.Set("agent_name_hint", "Codifica")
		q.Set("ext_agent_host_id", deviceID)
		q.Set("resource", resource)
		q.Set("scope", "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct")
		q.Set("nonce", nonce)
	} else {
		q.Set("client_id", codexClientID)
		q.Set("scope", "openid profile email offline_access")
		q.Set("id_token_add_organizations", "true")
		q.Set("codex_cli_simplified_flow", "true")
	}
	return &Login{Provider: p, State: state, verifier: verifier, URL: endpoint + "?" + q.Encode()}, nil
}

func EnsureDeviceID(s *Store) error {
	if s.DeviceID != "" {
		return nil
	}
	id, err := randomValue()
	if err == nil {
		s.DeviceID = id
	}
	return err
}

// ValidateCallback checks location, state and grant shape without exchanging a
// code. A valid provider denial is allowed here so the CLI can finish promptly.
func (l *Login) ValidateCallback(callback string) error {
	_, err := l.callbackValues(callback)
	return err
}

func (l *Login) callbackValues(callback string) (url.Values, error) {
	u, err := url.Parse(callback)
	if err != nil {
		return nil, errors.New("invalid callback URL")
	}
	expected, _ := url.Parse(RedirectURI)
	if u.Scheme != expected.Scheme || u.Host != expected.Host || u.Path != expected.Path || u.Fragment != "" || u.User != nil {
		return nil, errors.New("unexpected callback URL")
	}
	q, err := url.ParseQuery(u.RawQuery)
	if err != nil || len(q["state"]) != 1 || q.Get("state") != l.State {
		return nil, errors.New("OAuth state mismatch")
	}
	if values, ok := q["error"]; ok {
		if len(values) != 1 || values[0] == "" || len(q["code"]) != 0 {
			return nil, errors.New("invalid authorization error callback")
		}
		return q, nil
	}
	if len(q["code"]) != 1 || q.Get("code") == "" {
		return nil, errors.New("missing authorization code")
	}
	if l.Provider == OpenAI && (len(q["client_id"]) != 1 || q.Get("client_id") == "") {
		return nil, errors.New("missing issued client ID")
	}
	return q, nil
}

func (l *Login) Exchange(ctx context.Context, client *http.Client, callback string) (Credential, error) {
	q, err := l.callbackValues(callback)
	if err != nil {
		return Credential{}, err
	}
	if q.Get("error") != "" {
		return Credential{}, errors.New("authorization was denied")
	}
	clientID := codexClientID
	if l.Provider == OpenAI {
		clientID = q.Get("client_id")
	}
	form := url.Values{"grant_type": {"authorization_code"}, "client_id": {clientID}, "code": {q.Get("code")}, "code_verifier": {l.verifier}, "redirect_uri": {RedirectURI}}
	if l.Provider == OpenAI {
		form.Set("resource", resource)
	}
	return requestToken(ctx, client, l.Provider, form, Credential{ClientID: clientID})
}

func requestToken(ctx context.Context, client *http.Client, p Provider, form url.Values, old Credential) (Credential, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, p.tokenURL(), strings.NewReader(form.Encode()))
	if err != nil {
		return Credential{}, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	resp, err := client.Do(req)
	if err != nil {
		return Credential{}, errors.New("OAuth token request failed")
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return Credential{}, fmt.Errorf("OAuth token endpoint returned HTTP %d; try logging in again", resp.StatusCode)
	}
	var t struct {
		Access  string `json:"access_token"`
		Refresh string `json:"refresh_token"`
		Expires int64  `json:"expires_in"`
		Scope   string `json:"scope"`
	}
	if err := json.NewDecoder(io.LimitReader(resp.Body, 1<<20)).Decode(&t); err != nil {
		return Credential{}, errors.New("invalid OAuth token response")
	}
	if t.Access == "" || t.Expires <= 0 || t.Expires > 365*24*60*60 {
		return Credential{}, errors.New("OAuth response missing access token or valid expiry")
	}
	if t.Refresh == "" {
		t.Refresh = old.Refresh
	}
	if t.Refresh == "" {
		return Credential{}, errors.New("OAuth response missing refresh token")
	}
	if t.Scope == "" {
		t.Scope = old.Scope
	}
	return Credential{Access: t.Access, Refresh: t.Refresh, ClientID: old.ClientID, Scope: t.Scope, Expires: time.Now().Add(time.Duration(t.Expires) * time.Second)}, nil
}

// Manager serializes refreshes and persists rotated refresh tokens before use.
// The caller must hold the credential file lock for its lifetime and call Close
// before releasing it, including when the HTTP shutdown deadline is exceeded.
type Manager struct {
	mu     sync.Mutex
	dirty  bool
	closed bool
	Path   string
	Store  *Store
	Client *http.Client
}

func (m *Manager) Token(ctx context.Context, p Provider) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.closed {
		return "", errors.New("credential manager closed")
	}
	if err := ctx.Err(); err != nil {
		return "", err
	}
	if !p.Valid() {
		return "", errors.New("unknown provider")
	}
	c, ok := m.Store.Providers[p]
	if !ok || c.Access == "" || c.Refresh == "" || c.ClientID == "" {
		return "", errors.New("not signed in; run openaiproxy login first")
	}
	if m.dirty {
		if err := m.Store.Save(m.Path); err != nil {
			return "", fmt.Errorf("persist refreshed credentials: %w", err)
		}
		m.dirty = false
	}
	if time.Until(c.Expires) > time.Minute {
		return c.Access, nil
	}
	form := url.Values{"grant_type": {"refresh_token"}, "refresh_token": {c.Refresh}, "client_id": {c.ClientID}}
	if p == OpenAI {
		form.Set("resource", resource)
	}
	// Once a refresh starts, the provider may consume the old token. Finish and
	// persist independently of the initiating HTTP caller's cancellation.
	refreshCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 30*time.Second)
	defer cancel()
	next, err := requestToken(refreshCtx, m.Client, p, form, c)
	if err != nil {
		return "", err
	}
	m.Store.Providers[p] = next
	m.dirty = true
	if err := m.Store.Save(m.Path); err != nil {
		return "", fmt.Errorf("persist refreshed credentials: %w", err)
	}
	m.dirty = false
	return next.Access, nil
}

// Close waits for an in-flight refresh/persist to finish and rejects future
// calls. It must run before the process releases its credential file lock.
func (m *Manager) Close() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.closed = true
	if m.dirty {
		if err := m.Store.Save(m.Path); err != nil {
			return fmt.Errorf("persist refreshed credentials on shutdown: %w", err)
		}
		m.dirty = false
	}
	return nil
}

// AccountID reads routing metadata, not proof of token validity. The upstream
// verifies the bearer token; we never use this unverified claim for local auth.
func AccountID(token string) (string, error) {
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return "", errors.New("invalid Codex access token")
	}
	b, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return "", errors.New("invalid Codex JWT payload")
	}
	var payload struct {
		Auth struct {
			AccountID string `json:"chatgpt_account_id"`
		} `json:"https://api.openai.com/auth"`
	}
	if json.Unmarshal(b, &payload) != nil || payload.Auth.AccountID == "" || strings.ContainsAny(payload.Auth.AccountID, "\r\n") {
		return "", errors.New("Codex token has no account ID")
	}
	return payload.Auth.AccountID, nil
}
