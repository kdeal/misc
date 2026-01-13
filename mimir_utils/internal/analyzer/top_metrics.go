package analyzer

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/prometheus/prometheus/v3/model/labels"
	"github.com/prometheus/prometheus/v3/tsdb/chunkenc"
	"github.com/prometheus/prometheus/v3/tsdb/chunks"
	"github.com/prometheus/prometheus/v3/tsdb/index"
)

// MetricStat captures byte usage information for a metric across a set of blocks.
type MetricStat struct {
	Name   string
	Bytes  int64
	Series int
	Chunks int
}

// TopNMetrics walks the provided directory for TSDB blocks and returns the top metrics by bytes used.
func TopNMetrics(root string, limit int) ([]MetricStat, error) {
	aggregate := map[string]*MetricStat{}

	blockDirs, err := findBlockDirs(root)
	if err != nil {
		return nil, err
	}

	for _, blockDir := range blockDirs {
		if err := accumulateBlock(blockDir, aggregate); err != nil {
			return nil, fmt.Errorf("block %s: %w", blockDir, err)
		}
	}

	stats := make([]MetricStat, 0, len(aggregate))
	for _, stat := range aggregate {
		stats = append(stats, *stat)
	}

	sort.Slice(stats, func(i, j int) bool {
		if stats[i].Bytes == stats[j].Bytes {
			return stats[i].Name < stats[j].Name
		}
		return stats[i].Bytes > stats[j].Bytes
	})

	if limit > 0 && len(stats) > limit {
		stats = stats[:limit]
	}

	return stats, nil
}

func findBlockDirs(root string) ([]string, error) {
	entries, err := os.ReadDir(root)
	if err != nil {
		return nil, err
	}

	var blocks []string
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		dirPath := filepath.Join(root, entry.Name())
		if _, err := os.Stat(filepath.Join(dirPath, "meta.json")); err == nil {
			blocks = append(blocks, dirPath)
		}
	}

	if len(blocks) == 0 {
		return nil, fmt.Errorf("no TSDB blocks found in %s", root)
	}

	return blocks, nil
}

func accumulateBlock(blockDir string, aggregate map[string]*MetricStat) error {
	indexPath := filepath.Join(blockDir, "index")
	chunkDir := filepath.Join(blockDir, "chunks")

	indexReader, err := index.NewFileReader(indexPath)
	if err != nil {
		return fmt.Errorf("open index: %w", err)
	}
	defer indexReader.Close()

	pool := chunkenc.NewPool()
	chunkReader, err := chunks.NewDirReader(chunkDir, pool)
	if err != nil {
		return fmt.Errorf("open chunks: %w", err)
	}
	defer chunkReader.Close()

	name, value := index.AllPostingsKey()
	postings, err := indexReader.Postings(name, value)
	if err != nil {
		return fmt.Errorf("load postings: %w", err)
	}

	for postings.Next() {
		ref := postings.At()
		var lset labels.Labels
		var metas []chunks.Meta

		if err := indexReader.Series(ref, &lset, &metas); err != nil {
			return fmt.Errorf("read series %d: %w", ref, err)
		}

		metricName := lset.Get("__name__")
		if metricName == "" {
			metricName = "(no_metric_name)"
		}

		var seriesBytes int64
		for _, meta := range metas {
			chk, err := chunkReader.Chunk(meta.Ref)
			if err != nil {
				if strings.Contains(err.Error(), "reference") {
					return fmt.Errorf("chunk %d: %w", meta.Ref, err)
				}
				return fmt.Errorf("read chunk %d: %w", meta.Ref, err)
			}
			seriesBytes += int64(len(chk.Bytes()))
		}

		stat, ok := aggregate[metricName]
		if !ok {
			stat = &MetricStat{Name: metricName}
			aggregate[metricName] = stat
		}
		stat.Bytes += seriesBytes
		stat.Series++
		stat.Chunks += len(metas)
	}

	if err := postings.Err(); err != nil {
		return fmt.Errorf("postings iteration: %w", err)
	}

	return nil
}
