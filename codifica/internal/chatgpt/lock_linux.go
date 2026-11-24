//go:build linux

package chatgpt

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

// FileLock excludes other CLI login/serve processes before they load the store.
// Path is canonicalized so relative paths and symlink aliases share the lock.
// The sidecar must not be deleted: replacing it would bypass an existing lock.
type FileLock struct {
	Path string
	file *os.File
}

func Lock(path string) (*FileLock, error) {
	// Cleaning link/../file before resolving symlinks can select a different
	// account's store. Reject ambiguous parent traversal rather than normalize it.
	for _, component := range strings.Split(path, string(os.PathSeparator)) {
		if component == ".." {
			return nil, errors.New("credential path must not contain '..' components")
		}
	}
	path, err := filepath.Abs(path)
	if err != nil {
		return nil, err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return nil, err
	}
	parent, err := filepath.EvalSymlinks(filepath.Dir(path))
	if err != nil {
		return nil, err
	}
	path = filepath.Join(parent, filepath.Base(path))
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		path = resolved
	} else if !errors.Is(err, os.ErrNotExist) {
		return nil, err
	} else if _, err := os.Lstat(path); err == nil {
		return nil, errors.New("credential path is a dangling symlink")
	}
	// O_NOFOLLOW prevents a sidecar symlink from redirecting the lock elsewhere.
	fd, err := syscall.Open(path+".lock", syscall.O_CREAT|syscall.O_RDWR|syscall.O_CLOEXEC|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return nil, fmt.Errorf("open credential lock: %w", err)
	}
	f := os.NewFile(uintptr(fd), path+".lock")
	if err := syscall.Flock(fd, syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		f.Close()
		if errors.Is(err, syscall.EWOULDBLOCK) {
			return nil, errors.New("credentials are in use by another login or serving process; stop it first")
		}
		return nil, fmt.Errorf("lock credentials: %w", err)
	}
	return &FileLock{Path: path, file: f}, nil
}

// Close releases the advisory lock; the OS also releases it on process exit.
func (l *FileLock) Close() error { return l.file.Close() }
