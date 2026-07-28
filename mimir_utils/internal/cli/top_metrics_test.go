package cli

import (
	"testing"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/s3"
)

func TestS3ClientOptions(t *testing.T) {
	if options := s3ClientOptions(""); options != nil {
		t.Fatalf("s3ClientOptions(\"\") = %v, want nil", options)
	}

	options := s3.Options{}
	for _, apply := range s3ClientOptions("http://minio:9000") {
		apply(&options)
	}
	if got := aws.ToString(options.BaseEndpoint); got != "http://minio:9000" {
		t.Fatalf("BaseEndpoint = %q, want %q", got, "http://minio:9000")
	}
	if !options.UsePathStyle {
		t.Fatal("UsePathStyle = false, want true")
	}
}

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
		{"one exbibyte", 1 << 62, "4.0 EiB"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := humanReadableBytes(tt.in); got != tt.out {
				t.Fatalf("humanReadableBytes(%d) = %q, want %q", tt.in, got, tt.out)
			}
		})
	}
}
