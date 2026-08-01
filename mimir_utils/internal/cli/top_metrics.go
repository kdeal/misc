package cli

import (
	"context"
	"encoding/csv"
	"flag"
	"fmt"
	"os"
	"strconv"
	"text/tabwriter"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/s3"

	"mimir_utils/internal/analyzer"
)

func runTopMetrics(args []string) error {
	fs := flag.NewFlagSet("top-metrics", flag.ContinueOnError)
	s3URI := fs.String("s3-uri", "", "S3 URI containing TSDB blocks")
	endpointURL := fs.String("endpoint-url", "", "Custom S3-compatible endpoint (uses path-style addressing)")
	region := fs.String("region", "", "S3 region (defaults to AWS_REGION, AWS config, or us-east-1 for a custom endpoint)")
	limit := fs.Int("limit", 10, "Number of metrics to display (0 for all)")
	csvFile := fs.String("csv-file", "", "Write all metrics to this CSV file")

	fs.Usage = func() {
		fmt.Fprintf(fs.Output(), `Usage: mimir_utils top-metrics [options]

Options:
`)
		fs.PrintDefaults()
	}

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *s3URI == "" {
		fs.Usage()
		return fmt.Errorf("the -s3-uri flag is required")
	}

	loadOptions := []func(*config.LoadOptions) error{}
	if *region != "" {
		loadOptions = append(loadOptions, config.WithRegion(*region))
	}
	awsConfig, err := config.LoadDefaultConfig(context.Background(), loadOptions...)
	if err != nil {
		return fmt.Errorf("load AWS configuration: %w", err)
	}
	if awsConfig.Region == "" && *endpointURL != "" {
		awsConfig.Region = "us-east-1"
	}

	stats, err := analyzer.TopNMetrics(context.Background(), s3.NewFromConfig(awsConfig, s3ClientOptions(*endpointURL)...), *s3URI)
	if err != nil {
		return err
	}
	if *csvFile != "" {
		if err := writeMetricsCSV(*csvFile, stats); err != nil {
			return fmt.Errorf("write metrics CSV: %w", err)
		}
	}

	if len(stats) == 0 {
		fmt.Println("No metrics found.")
		return nil
	}
	if *limit > 0 && len(stats) > *limit {
		stats = stats[:*limit]
	}

	w := tabwriter.NewWriter(os.Stdout, 0, 2, 2, ' ', 0)
	fmt.Fprintln(w, "METRIC\tBYTES\tSERIES\tCHUNKS\tLAST RECORDED")
	for _, stat := range stats {
		fmt.Fprintf(w, "%s\t%s\t%d\t%d\t%s\n", stat.Name, humanReadableBytes(stat.Bytes), stat.Series, stat.Chunks, formatTimestamp(stat.LastRecorded))
	}
	return w.Flush()
}

func writeMetricsCSV(path string, stats []analyzer.MetricStat) (err error) {
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	defer func() {
		if closeErr := file.Close(); err == nil {
			err = closeErr
		}
	}()

	w := csv.NewWriter(file)
	if err := w.Write([]string{"metric", "bytes", "series", "chunks", "last_recorded"}); err != nil {
		return err
	}
	for _, stat := range stats {
		if err := w.Write([]string{
			stat.Name,
			strconv.FormatInt(stat.Bytes, 10),
			strconv.Itoa(stat.Series),
			strconv.Itoa(stat.Chunks),
			formatTimestamp(stat.LastRecorded),
		}); err != nil {
			return err
		}
	}
	w.Flush()
	return w.Error()
}

func formatTimestamp(timestamp int64) string {
	return time.UnixMilli(timestamp).UTC().Format(time.RFC3339Nano)
}

func s3ClientOptions(endpointURL string) []func(*s3.Options) {
	return []func(*s3.Options){func(options *s3.Options) {
		options.DisableLogOutputChecksumValidationSkipped = true
		if endpointURL != "" {
			options.BaseEndpoint = aws.String(endpointURL)
			options.UsePathStyle = true
		}
	}}
}

func humanReadableBytes(bytes int64) string {
	const unit = 1024.0
	units := []string{"B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"}
	val := float64(bytes)
	exp := 0

	for val >= unit && exp < len(units)-1 {
		val /= unit
		exp++
	}

	if val >= 10 || exp == 0 {
		return fmt.Sprintf("%.0f %s", val, units[exp])
	}
	return fmt.Sprintf("%.1f %s", val, units[exp])
}
