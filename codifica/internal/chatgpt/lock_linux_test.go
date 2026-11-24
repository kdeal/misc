//go:build linux

package chatgpt

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

func TestCredentialLock(t *testing.T) {
	path := filepath.Join(t.TempDir(), "private", "auth.json")
	lock, err := Lock(path)
	if err != nil {
		t.Fatal(err)
	}
	defer lock.Close()
	if _, err := Lock(path); err == nil {
		t.Fatal("second writer acquired lock")
	}
	// Relative/canonical directory aliases must not create a separate sidecar.
	alias := filepath.Join(t.TempDir(), "alias")
	if err := os.Symlink(filepath.Dir(path), alias); err != nil {
		t.Fatal(err)
	}
	if _, err := Lock(filepath.Join(alias, "auth.json")); err == nil {
		t.Fatal("directory symlink bypassed lock")
	}
	if err := (&Store{Providers: map[Provider]Credential{}}).Save(path); err != nil {
		t.Fatal(err)
	}
	fileAlias := filepath.Join(t.TempDir(), "auth-link")
	if err := os.Symlink(path, fileAlias); err != nil {
		t.Fatal(err)
	}
	if _, err := Lock(fileAlias); err == nil {
		t.Fatal("file symlink bypassed lock")
	}
	cmd := exec.Command(os.Args[0], "-test.run=^TestCredentialLockChild$")
	cmd.Env = append(os.Environ(), "CODIFICA_TEST_LOCK="+path)
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("child process bypassed lock: %v\n%s", err, out)
	}
	if err := lock.Close(); err != nil {
		t.Fatal(err)
	}
	next, err := Lock(path)
	if err != nil {
		t.Fatalf("lock was not released: %v", err)
	}
	if err := next.Close(); err != nil {
		t.Fatal(err)
	}
	// A cleanly closed sidecar persists but is not a stale lock.
	if _, err := os.Stat(path + ".lock"); err != nil {
		t.Fatal(err)
	}
}

func TestCredentialLockChild(t *testing.T) {
	path := os.Getenv("CODIFICA_TEST_LOCK")
	if path == "" {
		t.Skip("subprocess helper")
	}
	if lock, err := Lock(path); err == nil {
		lock.Close()
		t.Fatal("acquired parent process's credential lock")
	}
}

func TestCredentialLockRejectsParentTraversalAfterSymlink(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(t.TempDir(), "sub")
	if err := os.Mkdir(target, 0700); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(dir, "link")
	if err := os.Symlink(target, link); err != nil {
		t.Fatal(err)
	}
	// Do not use filepath.Join here: it would erase the traversal under test.
	if lock, err := Lock(link + "/../auth.json"); err == nil {
		lock.Close()
		t.Fatal("accepted ambiguous parent traversal")
	}
	if _, err := os.Stat(filepath.Join(dir, "auth.json.lock")); !os.IsNotExist(err) {
		t.Fatal("created a lock at the wrong credential location")
	}
}

func TestCredentialLockRejectsSidecarSymlink(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "auth.json")
	if err := os.Symlink(filepath.Join(dir, "other"), path+".lock"); err != nil {
		t.Fatal(err)
	}
	if lock, err := Lock(path); err == nil {
		lock.Close()
		t.Fatal("followed sidecar symlink")
	}
}
