package main

import (
	"context"
	"flag"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"github.com/kdeal/misc/codifica/internal/forwardproxy"
	"github.com/kdeal/misc/codifica/internal/httpserver"
)

func main() {
	allowList := flag.String("allow-list", "", "path to a file of allowed destinations (hostname:80,443 or hostname:*, one per line)")
	evaluate := flag.Bool("evaluate", false, "log allow-list misses without blocking them (requires -allow-list)")
	flag.Parse()
	if *evaluate && *allowList == "" {
		slog.Error("-evaluate requires -allow-list")
		os.Exit(1)
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	var proxy *forwardproxy.Proxy
	if *allowList != "" {
		var err error
		var options []forwardproxy.AllowListOption
		if *evaluate {
			options = append(options, forwardproxy.WithEvaluateAllowList())
		}
		proxy, err = forwardproxy.NewWithAllowListFile(*allowList, options...)
		if err != nil {
			slog.Error("load allow list", "error", err)
			os.Exit(1)
		}
	} else {
		proxy = forwardproxy.New()
	}
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
