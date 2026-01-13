package cli

import "testing"

func TestHumanReadableBytes(t *testing.T) {
	tests := []struct {
		name string
		in   int64
		out  string
	}{
		{"zero bytes", 0, "0 B"},
		{"single byte", 1, "1 B"},
		{"just below kibibyte", 1023, "1023 B"},
		{"one kibibyte", 1024, "1.0 KiB"},
		{"fractional kibibyte", 1536, "1.5 KiB"},
		{"ten kibibytes", 10 * 1024, "10 KiB"},
		{"one mebibyte", 1024 * 1024, "1.0 MiB"},
		{"many gibibytes", 25 * 1024 * 1024 * 1024, "25 GiB"},
		{"overflow past units", 1 << 62, "4 PiB"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := humanReadableBytes(tt.in); got != tt.out {
				t.Fatalf("humanReadableBytes(%d) = %q, want %q", tt.in, got, tt.out)
			}
		})
	}
}
