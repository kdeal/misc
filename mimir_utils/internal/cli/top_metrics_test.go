package cli

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/s3"

	"mimir_utils/internal/analyzer"
)

func TestS3ClientOptions(t *testing.T) {
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
	if !options.DisableLogOutputChecksumValidationSkipped {
		t.Fatal("DisableLogOutputChecksumValidationSkipped = false, want true")
	}

	defaultOptions := s3.Options{}
	for _, apply := range s3ClientOptions("") {
		apply(&defaultOptions)
	}
	if !defaultOptions.DisableLogOutputChecksumValidationSkipped {
		t.Fatal("default DisableLogOutputChecksumValidationSkipped = false, want true")
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

func TestWriteMetricsCSV(t *testing.T) {
	path := filepath.Join(t.TempDir(), "metrics.csv")
	stats := []analyzer.MetricStat{
		{Name: "requests,total", Bytes: 1536, Series: 12, Chunks: 34, LastRecorded: 1785587696789},
		{Name: "errors_total", Bytes: 42, Series: 2, Chunks: 3, LastRecorded: 1785587700000},
	}

	if err := writeMetricsCSV(path, stats); err != nil {
		t.Fatalf("writeMetricsCSV() error = %v", err)
	}
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read CSV: %v", err)
	}
	want := "metric,bytes,series,chunks,last_recorded\n\"requests,total\",1536,12,34,2026-08-01T12:34:56.789Z\nerrors_total,42,2,3,2026-08-01T12:35:00Z\n"
	if string(got) != want {
		t.Fatalf("CSV contents = %q, want %q", got, want)
	}
}
