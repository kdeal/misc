//go:build linux

package main

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCredentialConfigDirectory(t *testing.T) {
	for _, tc := range []string{"default", "xdg", "override"} {
		t.Run(tc, func(t *testing.T) {
			home := t.TempDir()
			t.Setenv("HOME", home)
			t.Setenv("XDG_CONFIG_HOME", "")
			want := filepath.Join(home, ".config", "codifica", "auth.json")
			args := []string{"serve"}
			if tc == "xdg" {
				config := t.TempDir()
				t.Setenv("XDG_CONFIG_HOME", config)
				want = filepath.Join(config, "codifica", "auth.json")
			}
			if tc == "override" {
				want = filepath.Join(t.TempDir(), "custom-auth.json")
				args = append(args, "-credentials", want)
			}
			// An unsigned-in serve acquires the same lock/path as login without opening
			// a network listener or writing any access/refresh credentials.
			err := run(context.Background(), args)
			if err == nil || !strings.Contains(err.Error(), "not signed in") {
				t.Fatalf("unexpected serve error: %v", err)
			}
			if _, err := os.Stat(want + ".lock"); err != nil {
				t.Fatalf("credential lock not in expected config directory: %v", err)
			}
			if _, err := os.Stat(filepath.Join(home, ".codifica")); !os.IsNotExist(err) {
				t.Fatal("created legacy credential directory")
			}
		})
	}
}
