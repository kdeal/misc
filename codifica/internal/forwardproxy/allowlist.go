package forwardproxy

import (
	"bufio"
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"
)

// AllowListOption configures allow-list handling during proxy construction.
type AllowListOption func(*Proxy)

// WithEvaluateAllowList logs allow-list misses without rejecting them.
// Hostname validation is still enforced.
func WithEvaluateAllowList() AllowListOption {
	return func(p *Proxy) { p.evaluateAllowList = true }
}

// NewWithAllowListFile constructs a proxy restricted to the hostnames in path.
// The file contains one hostname:ports per line; blank lines and lines beginning
// with # are ignored. Hostnames match exactly, ignoring case and a trailing dot.
// Ports must be a comma-separated list of numbers or * to allow all ports.
// Bare hostnames, IP literals, URLs, and hostname wildcards are not accepted.
// An empty file denies
// all destinations. The file is read once, and any error prevents construction.
func NewWithAllowListFile(path string, options ...AllowListOption) (*Proxy, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("open allow list: %w", err)
	}
	defer file.Close()

	hosts := make(map[string]struct{})
	scanner := bufio.NewScanner(file)
	line := 0
	for scanner.Scan() {
		line++
		host := strings.TrimSpace(scanner.Text())
		if host == "" || strings.HasPrefix(host, "#") {
			continue
		}
		entry := host
		host, ports, err := net.SplitHostPort(entry)
		if err != nil || !allowedHost(host) {
			return nil, fmt.Errorf("allow list %q line %d: invalid destination %q (expected hostname:ports)", path, line, entry)
		}
		if ports == "*" {
			hosts[allowListKey(host, "*")] = struct{}{}
			continue
		}
		for _, port := range strings.Split(ports, ",") {
			if !validPort(port) {
				return nil, fmt.Errorf("allow list %q line %d: invalid port %q", path, line, port)
			}
			hosts[allowListKey(host, port)] = struct{}{}
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("read allow list %q: %w", path, err)
	}
	p := newProxy(hosts)
	for _, option := range options {
		option(p)
	}
	return p, nil
}

// allowListKey is called only with * or a validated port.
func allowListKey(host, port string) string {
	if port == "*" {
		return net.JoinHostPort(normalizeHost(host), "*")
	}
	n, _ := strconv.Atoi(port)
	return net.JoinHostPort(normalizeHost(host), strconv.Itoa(n))
}

func (p *Proxy) allowsDestination(host, port string) bool {
	_, anyPort := p.allowList[allowListKey(host, "*")]
	_, exactPort := p.allowList[allowListKey(host, port)]
	return anyPort || exactPort
}

func normalizeHost(host string) string {
	return strings.ToLower(strings.TrimSuffix(host, "."))
}
