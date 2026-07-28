package cli

import (
	"context"
	"flag"
	"fmt"
	"os"
	"text/tabwriter"

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

	stats, err := analyzer.TopNMetrics(context.Background(), s3.NewFromConfig(awsConfig, s3ClientOptions(*endpointURL)...), *s3URI, *limit)
	if err != nil {
		return err
	}

	if len(stats) == 0 {
		fmt.Println("No metrics found.")
		return nil
	}

	w := tabwriter.NewWriter(os.Stdout, 0, 2, 2, ' ', 0)
	fmt.Fprintln(w, "METRIC\tBYTES\tSERIES\tCHUNKS")
	for _, stat := range stats {
		fmt.Fprintf(w, "%s\t%s\t%d\t%d\n", stat.Name, humanReadableBytes(stat.Bytes), stat.Series, stat.Chunks)
	}
	return w.Flush()
}

func s3ClientOptions(endpointURL string) []func(*s3.Options) {
	if endpointURL == "" {
		return nil
	}

	return []func(*s3.Options){func(options *s3.Options) {
		options.BaseEndpoint = aws.String(endpointURL)
		options.UsePathStyle = true
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
