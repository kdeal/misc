package chatgpt

import (
	"context"
	"net/http"
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"
)

func TestRefreshFinishesAfterCallerCancellation(t *testing.T) {
	accepted := make(chan struct{})
	release := make(chan struct{})
	defer func() {
		select {
		case <-release:
		default:
			close(release)
		}
	}()
	var calls atomic.Int32
	m := &Manager{
		Path:  filepath.Join(t.TempDir(), "auth.json"),
		Store: &Store{Providers: map[Provider]Credential{Codex: {Access: "expired", Refresh: "old", ClientID: "client"}}},
		Client: &http.Client{Transport: roundTripFunc(func(r *http.Request) (*http.Response, error) {
			calls.Add(1)
			// The server has accepted the grant and consumed the old refresh token.
			close(accepted)
			if deadline, ok := r.Context().Deadline(); !ok || time.Until(deadline) > 30*time.Second {
				t.Error("refresh lacks its own bounded context")
			}
			select {
			case <-r.Context().Done():
				return nil, r.Context().Err()
			case <-release:
				return response(200, `{"access_token":"new","refresh_token":"rotated","expires_in":3600}`), nil
			}
		})},
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() { _, err := m.Token(ctx, Codex); done <- err }()
	select {
	case <-accepted:
	case <-time.After(3 * time.Second):
		t.Fatal("refresh did not start")
	}
	cancel()
	close(release)
	select {
	case err := <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("refresh did not finish")
	}
	saved, err := Load(m.Path)
	if err != nil {
		t.Fatal(err)
	}
	if saved.Providers[Codex].Refresh != "rotated" {
		t.Fatal("replacement refresh token lost after disconnect")
	}
	token, err := m.Token(context.Background(), Codex)
	if err != nil || token != "new" || calls.Load() != 1 {
		t.Fatalf("subsequent request re-exchanged grant: %s %v calls=%d", token, err, calls.Load())
	}
	// A request already canceled before a refresh starts should not consume a grant.
	if _, err := m.Token(ctx, Codex); err != context.Canceled {
		t.Fatalf("canceled request: %v", err)
	}
	m.Close()
	if _, err := m.Token(context.Background(), Codex); err == nil {
		t.Fatal("closed manager accepted request")
	}
}

func TestManagerCloseRetriesDirtyPersistence(t *testing.T) {
	for _, recoverStorage := range []bool{true, false} {
		t.Run(map[bool]string{true: "recovered", false: "still-failing"}[recoverStorage], func(t *testing.T) {
			dir := t.TempDir()
			blocker := filepath.Join(dir, "blocker")
			if err := os.WriteFile(blocker, []byte("not a directory"), 0600); err != nil {
				t.Fatal(err)
			}
			m := &Manager{
				Path:  filepath.Join(blocker, "auth.json"),
				Store: &Store{Providers: map[Provider]Credential{Codex: {Access: "expired", Refresh: "old", ClientID: "client"}}},
				Client: &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) {
					return response(200, `{"access_token":"new","refresh_token":"rotated","expires_in":3600}`), nil
				})},
			}
			if _, err := m.Token(context.Background(), Codex); err == nil {
				t.Fatal("expected persistence failure")
			}
			if recoverStorage {
				if err := os.Remove(blocker); err != nil {
					t.Fatal(err)
				}
			}
			err := m.Close()
			if !recoverStorage {
				if err == nil {
					t.Fatal("shutdown hid unsaved refresh token")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			saved, err := Load(m.Path)
			if err != nil || saved.Providers[Codex].Refresh != "rotated" {
				t.Fatal("shutdown lost rotated refresh token")
			}
		})
	}
}

func TestManagerCloseDrainsRefresh(t *testing.T) {
	accepted := make(chan struct{})
	release := make(chan struct{})
	defer func() {
		select {
		case <-release:
		default:
			close(release)
		}
	}()
	m := &Manager{
		Path:  filepath.Join(t.TempDir(), "auth.json"),
		Store: &Store{Providers: map[Provider]Credential{Codex: {Access: "expired", Refresh: "old", ClientID: "client"}}},
		Client: &http.Client{Transport: roundTripFunc(func(*http.Request) (*http.Response, error) {
			close(accepted)
			<-release
			return response(200, `{"access_token":"new","refresh_token":"rotated","expires_in":3600}`), nil
		})},
	}
	finished := make(chan error, 1)
	go func() { _, err := m.Token(context.Background(), Codex); finished <- err }()
	select {
	case <-accepted:
	case <-time.After(3 * time.Second):
		t.Fatal("refresh did not start")
	}
	closed := make(chan struct{})
	go func() { m.Close(); close(closed) }()
	select {
	case <-closed:
		t.Fatal("Close did not wait for in-flight refresh")
	case <-time.After(20 * time.Millisecond):
	}
	close(release)
	select {
	case <-closed:
	case <-time.After(3 * time.Second):
		t.Fatal("Close did not finish")
	}
	if err := <-finished; err != nil {
		t.Fatal(err)
	}
	saved, err := Load(m.Path)
	if err != nil || saved.Providers[Codex].Refresh != "rotated" {
		t.Fatal("Close returned before persistence")
	}
}
