//go:build linux

// openaiproxy signs in with a ChatGPT subscription and serves a local Responses proxy.
package main

import (
	"bufio"
	"context"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/kdeal/misc/codifica/internal/chatgpt"
	"github.com/kdeal/misc/codifica/internal/httpserver"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	if err := run(ctx, os.Args[1:]); err != nil {
		slog.Error("openaiproxy stopped", "error", err)
		os.Exit(1)
	}
}

func run(ctx context.Context, args []string) (runErr error) {
	if len(args) == 0 || (args[0] != "login" && args[0] != "serve") {
		return errors.New("usage: openaiproxy <login|serve> [-provider openai-codex|openai] [-credentials path]")
	}
	configDir, err := os.UserConfigDir()
	if err != nil {
		return err
	}
	flags := flag.NewFlagSet(args[0], flag.ContinueOnError)
	providerName := flags.String("provider", string(chatgpt.Codex), "subscription provider: openai-codex or openai")
	path := flags.String("credentials", filepath.Join(configDir, "codifica", "auth.json"), "private credential file")
	var paste *bool
	var addr *string
	if args[0] == "login" {
		paste = flags.Bool("paste", false, "paste the complete redirect URL instead of listening on localhost")
	} else {
		addr = flags.String("listen", "127.0.0.1:8083", "loopback listen address")
	}
	if err := flags.Parse(args[1:]); err != nil {
		return err
	}
	if flags.NArg() != 0 {
		return errors.New("unexpected positional arguments")
	}
	provider := chatgpt.Provider(*providerName)
	if !provider.Valid() {
		return errors.New("provider must be openai or openai-codex")
	}
	lock, err := chatgpt.Lock(*path)
	if err != nil {
		return err
	}
	defer lock.Close()
	*path = lock.Path
	store, err := chatgpt.Load(*path)
	if err != nil {
		return err
	}
	// Never follow redirects with credentials or authorization codes.
	noRedirect := func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }
	authClient := &http.Client{Timeout: 30 * time.Second, CheckRedirect: noRedirect}
	if args[0] == "login" {
		ctx, cancel := context.WithTimeout(ctx, 10*time.Minute)
		defer cancel()
		if err := chatgpt.EnsureDeviceID(store); err != nil {
			return err
		}
		if err := store.Save(*path); err != nil {
			return err
		}
		login, err := chatgpt.BeginLogin(provider, store.DeviceID)
		if err != nil {
			return err
		}
		callback, err := awaitCallback(ctx, login, *paste)
		if err != nil {
			return err
		}
		credential, err := login.Exchange(ctx, authClient, callback)
		if err != nil {
			return err
		}
		store.Providers[provider] = credential
		if err := store.Save(*path); err != nil {
			return err
		}
		fmt.Fprintf(os.Stdout, "Signed in to %s. Credentials saved to %s.\n", provider, *path)
		return nil
	}
	host, _, err := net.SplitHostPort(*addr)
	if err != nil || net.ParseIP(host) == nil || !net.ParseIP(host).IsLoopback() {
		return errors.New("-listen must use a literal loopback IP and port")
	}
	if _, ok := store.Providers[provider]; !ok {
		return fmt.Errorf("not signed in to %s; run openaiproxy login -provider %s", provider, provider)
	}
	manager := &chatgpt.Manager{Path: *path, Store: store, Client: authClient}
	// Drain refreshes and retry dirty persistence before releasing the lock.
	defer func() { runErr = errors.Join(runErr, manager.Close()) }()
	// No whole-request deadline: Responses can be long-lived SSE streams.
	client := &http.Client{CheckRedirect: noRedirect}
	server := httpserver.New(*addr, chatgpt.NewHandler(manager, provider, client))
	server.ReadHeaderTimeout = 10 * time.Second
	slog.Info("subscription proxy listening", "address", *addr, "provider", provider)
	return httpserver.Run(ctx, server)
}

func awaitCallback(ctx context.Context, login *chatgpt.Login, paste bool) (string, error) {
	callbacks := make(chan string, 1)
	failures := make(chan error, 1)
	if paste {
		fmt.Fprintf(os.Stdout, "Open this URL in your browser:\n%s\n\nAfter authorization, paste the complete redirect URL (including state and client_id, if present):\n", login.URL)
		go func() {
			scanner := bufio.NewScanner(os.Stdin)
			scanner.Buffer(make([]byte, 4096), 64<<10)
			if scanner.Scan() {
				callbacks <- strings.TrimSpace(scanner.Text())
				return
			}
			failures <- errors.New("could not read redirect URL")
		}()
	} else {
		listener, err := net.Listen("tcp", "127.0.0.1:1455")
		if err != nil {
			return "", fmt.Errorf("listen for OAuth callback (or use -paste): %w", err)
		}
		return serveCallback(ctx, login, listener)
	}
	select {
	case callback := <-callbacks:
		return callback, nil
	case err := <-failures:
		return "", err
	case <-ctx.Done():
		return "", ctx.Err()
	}
}
