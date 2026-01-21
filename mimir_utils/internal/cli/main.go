package cli

import (
	"errors"
	"flag"
	"fmt"
)

// RootUsage prints a helpful summary of the available subcommands.
func RootUsage() {
	fmt.Fprintf(flag.CommandLine.Output(), `mimir_utils is a collection of small tools.

Usage:
  mimir_utils <subcommand> [options]

Available subcommands:
  top-metrics    Analyze TSDB blocks and print the metrics using the most bytes.

`)
}

// Execute parses the subcommand and invokes it with the provided arguments.
func Execute(args []string) error {
	if len(args) == 0 {
		RootUsage()
		return errors.New("no subcommand specified")
	}

	switch args[0] {
	case "top-metrics":
		return runTopMetrics(args[1:])
	case "help", "-h", "--help":
		RootUsage()
		return nil
	default:
		RootUsage()
		return fmt.Errorf("unknown subcommand %q", args[0])
	}
}
