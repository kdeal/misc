package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"github.com/kdeal/misc/codifica/internal/httpserver"
	"github.com/kdeal/misc/codifica/internal/web"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	server := httpserver.New("127.0.0.1:8080", web.NewHandler())
	if err := httpserver.Run(ctx, server); err != nil {
		slog.Error("service stopped", "error", err)
		os.Exit(1)
	}
}
