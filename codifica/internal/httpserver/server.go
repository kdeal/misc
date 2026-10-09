// Package httpserver provides shared HTTP listener and shutdown behavior.
package httpserver

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"time"
)

const shutdownTimeout = 10 * time.Second

// New returns a server using the standard library defaults.
func New(addr string, handler http.Handler) *http.Server {
	return &http.Server{
		Addr:    addr,
		Handler: handler,
	}
}

// Run starts each server independently. Cancellation or a server failure shuts
// down every server. Callers must supply fresh servers with non-nil handlers.
func Run(ctx context.Context, servers ...*http.Server) error {
	if len(servers) == 0 {
		return errors.New("no HTTP servers configured")
	}
	seen := make(map[*http.Server]bool)
	for _, server := range servers {
		if server == nil || server.Handler == nil || seen[server] {
			return errors.New("HTTP servers must be unique, non-nil, and have handlers")
		}
		seen[server] = true
	}

	results := make(chan error, len(servers))
	for _, server := range servers {
		go func() {
			err := server.ListenAndServe()
			if errors.Is(err, http.ErrServerClosed) {
				err = nil
			}
			if err != nil {
				err = fmt.Errorf("serve on %s: %w", server.Addr, err)
			}
			results <- err
		}()
	}

	var runErr error
	remaining := len(servers)
	select {
	case <-ctx.Done():
	case runErr = <-results:
		remaining--
	}

	// Shutdown concurrently so a slow listener doesn't delay the others.
	shutdownCtx, cancel := context.WithTimeout(context.Background(), shutdownTimeout)
	defer cancel()
	shutdownResults := make(chan error, len(servers))
	for _, server := range servers {
		go func() {
			err := server.Shutdown(shutdownCtx)
			if err != nil {
				// Force-close connections if the grace period expires.
				_ = server.Close()
				err = fmt.Errorf("shutdown %s: %w", server.Addr, err)
			}
			shutdownResults <- err
		}()
	}
	for range servers {
		runErr = errors.Join(runErr, <-shutdownResults)
	}
	for range remaining {
		runErr = errors.Join(runErr, <-results)
	}
	return runErr
}
