//go:build linux

package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"os"
	"sync/atomic"
	"time"

	"github.com/kdeal/misc/codifica/internal/chatgpt"
)

func serveCallback(ctx context.Context, login *chatgpt.Login, listener net.Listener) (string, error) {
	callbacks := make(chan string, 1)
	failures := make(chan error, 1)
	var accepted atomic.Bool
	mux := http.NewServeMux()
	mux.HandleFunc("GET /auth/callback", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("Referrer-Policy", "no-referrer")
		// ServeMux's GET pattern also matches HEAD, but a probe must not consume
		// the one-time authorization result.
		if r.Method != http.MethodGet {
			w.Header().Set("Allow", http.MethodGet)
			http.Error(w, "callback requires GET", http.StatusMethodNotAllowed)
			return
		}
		// Validate the actual authority, not a reconstructed canonical host.
		// Only a valid grant or provider denial may consume the one-time slot.
		callback := "http://" + r.Host + r.URL.RequestURI()
		if err := login.ValidateCallback(callback); err != nil {
			http.Error(w, "invalid OAuth callback", http.StatusBadRequest)
			return
		}
		if !accepted.CompareAndSwap(false, true) {
			http.Error(w, "callback already received", http.StatusConflict)
			return
		}
		callbacks <- callback
		_, _ = fmt.Fprintln(w, "Authorization received. Return to your terminal to check the sign-in result.")
	})
	server := &http.Server{Handler: mux, ReadHeaderTimeout: 5 * time.Second, WriteTimeout: 5 * time.Second}
	defer func() {
		// The callback is delivered before the handler writes its acknowledgment.
		// Let that response finish even if the login context has been canceled.
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := server.Shutdown(shutdownCtx); err != nil {
			_ = server.Close()
		}
	}()
	go func() {
		if err := server.Serve(listener); err != nil && !errors.Is(err, http.ErrServerClosed) {
			failures <- err
		}
	}()
	fmt.Fprintf(os.Stdout, "Open this URL in your browser:\n%s\n\nWaiting for the localhost callback...\n", login.URL)
	select {
	case callback := <-callbacks:
		return callback, nil
	case err := <-failures:
		return "", err
	case <-ctx.Done():
		return "", ctx.Err()
	}
}
