package cli

import (
	"flag"
	"fmt"
	"math"
	"os"
	"text/tabwriter"

	"mimir_utils/internal/analyzer"
)

func runTopMetrics(args []string) error {
	fs := flag.NewFlagSet("top-metrics", flag.ContinueOnError)
	dir := fs.String("dir", "", "Directory containing TSDB blocks")
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

	if *dir == "" {
		fs.Usage()
		return fmt.Errorf("the -dir flag is required")
	}

	stats, err := analyzer.TopNMetrics(*dir, *limit)
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

func humanReadableBytes(bytes int64) string {
	const unit = 1024.0
	units := []string{"B", "KiB", "MiB", "GiB", "TiB", "PiB"}
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
