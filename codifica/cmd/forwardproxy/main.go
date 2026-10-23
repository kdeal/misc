package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"github.com/kdeal/misc/codifica/internal/forwardproxy"
	"github.com/kdeal/misc/codifica/internal/httpserver"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	proxy := forwardproxy.New()
	server := httpserver.New("127.0.0.1:8081", proxy.Handler())
	admin := httpserver.New("127.0.0.1:8082", proxy.AdminHandler())
	// Shutdown does not close hijacked connections; close them explicitly.
	server.RegisterOnShutdown(func() { _ = proxy.Close() })
	err := httpserver.Run(ctx, server, admin)
	_ = proxy.Close()
	if err != nil {
		slog.Error("service stopped", "error", err)
		os.Exit(1)
	}
}
