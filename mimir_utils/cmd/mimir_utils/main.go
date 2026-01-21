package main

import (
	"flag"
	"fmt"
	"os"

	"mimir_utils/internal/cli"
)

func main() {
	flag.Usage = cli.RootUsage

	if err := cli.Execute(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}
