package analyzer

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"hash/crc32"
	"io"
	"net/url"
	"os"
	"sort"
	"strconv"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/aws-sdk-go-v2/service/s3/types"
	"github.com/prometheus/prometheus/model/labels"
	"github.com/prometheus/prometheus/tsdb/chunkenc"
	"github.com/prometheus/prometheus/tsdb/chunks"
	"github.com/prometheus/prometheus/tsdb/index"
)

const chunkFormatV1 = 1

var castagnoliTable = crc32.MakeTable(crc32.Castagnoli)

// MetricStat captures byte usage information for a metric across a set of blocks.
type MetricStat struct {
	Name   string
	Bytes  int64
	Series int
	Chunks int
	// LastRecorded is the latest sample timestamp in Unix milliseconds.
	LastRecorded int64
}

type s3Client interface {
	ListObjectsV2(context.Context, *s3.ListObjectsV2Input, ...func(*s3.Options)) (*s3.ListObjectsV2Output, error)
	HeadObject(context.Context, *s3.HeadObjectInput, ...func(*s3.Options)) (*s3.HeadObjectOutput, error)
	GetObject(context.Context, *s3.GetObjectInput, ...func(*s3.Options)) (*s3.GetObjectOutput, error)
}

// TopNMetrics reads TSDB blocks from an S3 URI and returns the top metrics by bytes used.
func TopNMetrics(ctx context.Context, client s3Client, s3URI string) ([]MetricStat, error) {
	bucket, prefix, err := parseS3URI(s3URI)
	if err != nil {
		return nil, err
	}

	aggregate := map[string]*MetricStat{}
	blockPrefixes, err := findBlockPrefixes(ctx, client, bucket, prefix)
	if err != nil {
		return nil, err
	}

	for _, blockPrefix := range blockPrefixes {
		blockAggregate := map[string]*MetricStat{}
		if err := accumulateBlock(ctx, client, bucket, blockPrefix, blockAggregate); err != nil {
			fmt.Fprintf(os.Stderr, "warning: skipping block %s: %v\n", blockPrefix, err)
			continue
		}
		for name, stat := range blockAggregate {
			if total, ok := aggregate[name]; ok {
				total.Bytes += stat.Bytes
				total.Series += stat.Series
				if stat.Chunks > 0 && (total.Chunks == 0 || stat.LastRecorded > total.LastRecorded) {
					total.LastRecorded = stat.LastRecorded
				}
				total.Chunks += stat.Chunks
				continue
			}
			aggregate[name] = stat
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

	return stats, nil
}

func parseS3URI(rawURI string) (bucket, prefix string, err error) {
	u, err := url.Parse(rawURI)
	if err != nil {
		return "", "", fmt.Errorf("parse S3 URI: %w", err)
	}
	if u.Scheme != "s3" {
		return "", "", fmt.Errorf("S3 URI must use the s3 scheme")
	}
	if u.Host == "" {
		return "", "", fmt.Errorf("S3 URI must include a bucket")
	}
	if u.User != nil || u.RawQuery != "" || u.Fragment != "" {
		return "", "", fmt.Errorf("S3 URI must not include credentials, a query, or a fragment")
	}

	return u.Host, strings.Trim(u.Path, "/"), nil
}

func findBlockPrefixes(ctx context.Context, client s3Client, bucket, prefix string) ([]string, error) {
	basePrefix := withTrailingSlash(prefix)
	blockPrefixes := map[string]struct{}{}
	var continuationToken *string

	for {
		output, err := client.ListObjectsV2(ctx, &s3.ListObjectsV2Input{
			Bucket:            aws.String(bucket),
			Prefix:            aws.String(basePrefix),
			Delimiter:         aws.String("/"),
			ContinuationToken: continuationToken,
		})
		if err != nil {
			return nil, fmt.Errorf("list blocks: %w", err)
		}

		for _, object := range output.Contents {
			if aws.ToString(object.Key) == basePrefix+"meta.json" {
				blockPrefixes[prefix] = struct{}{}
			}
		}
		for _, commonPrefix := range output.CommonPrefixes {
			blockPrefix := strings.TrimSuffix(aws.ToString(commonPrefix.Prefix), "/")
			_, err := client.HeadObject(ctx, &s3.HeadObjectInput{
				Bucket: aws.String(bucket),
				Key:    aws.String(objectKey(blockPrefix, "meta.json")),
			})
			if err == nil {
				blockPrefixes[blockPrefix] = struct{}{}
				continue
			}
			if !isObjectNotFound(err) {
				return nil, fmt.Errorf("check block metadata %q: %w", blockPrefix, err)
			}
		}

		if !aws.ToBool(output.IsTruncated) {
			break
		}
		if output.NextContinuationToken == nil {
			return nil, fmt.Errorf("list blocks: truncated response missing continuation token")
		}
		continuationToken = output.NextContinuationToken
	}

	if len(blockPrefixes) == 0 {
		return nil, fmt.Errorf("no TSDB blocks found in s3://%s/%s", bucket, prefix)
	}

	blocks := make([]string, 0, len(blockPrefixes))
	for blockPrefix := range blockPrefixes {
		blocks = append(blocks, blockPrefix)
	}
	sort.Strings(blocks)
	return blocks, nil
}

func isObjectNotFound(err error) bool {
	var noSuchKey *types.NoSuchKey
	var notFound *types.NotFound
	return errors.As(err, &noSuchKey) || errors.As(err, &notFound)
}

func accumulateBlock(ctx context.Context, client s3Client, bucket, blockPrefix string, aggregate map[string]*MetricStat) error {
	indexBytes, err := getObject(ctx, client, bucket, objectKey(blockPrefix, "index"))
	if err != nil {
		return fmt.Errorf("read index: %w", err)
	}
	indexReader, err := index.NewReader(byteSlice(indexBytes), index.DecodePostingsRaw)
	if err != nil {
		return fmt.Errorf("open index: %w", err)
	}
	defer indexReader.Close()

	chunkReader, err := newS3ChunkReader(ctx, client, bucket, objectKey(blockPrefix, "chunks"))
	if err != nil {
		return fmt.Errorf("open chunks: %w", err)
	}

	name, value := index.AllPostingsKey()
	postings, err := indexReader.Postings(ctx, name, value)
	if err != nil {
		return fmt.Errorf("load postings: %w", err)
	}

	for postings.Next() {
		ref := postings.At()
		var builder labels.ScratchBuilder
		var metas []chunks.Meta

		if err := indexReader.Series(ref, &builder, &metas); err != nil {
			return fmt.Errorf("read series %d: %w", ref, err)
		}

		lset := builder.Labels()
		metricName := lset.Get("__name__")
		if metricName == "" {
			metricName = "(no_metric_name)"
		}

		var seriesBytes int64
		var lastRecorded int64
		for i, meta := range metas {
			chk, _, err := chunkReader.ChunkOrIterable(meta)
			if err != nil {
				return fmt.Errorf("read chunk %d: %w", meta.Ref, err)
			}
			seriesBytes += int64(len(chk.Bytes()))
			if i == 0 || meta.MaxTime > lastRecorded {
				lastRecorded = meta.MaxTime
			}
		}

		stat, ok := aggregate[metricName]
		if !ok {
			stat = &MetricStat{Name: metricName}
			aggregate[metricName] = stat
		}
		stat.Bytes += seriesBytes
		stat.Series++
		if len(metas) > 0 && (stat.Chunks == 0 || lastRecorded > stat.LastRecorded) {
			stat.LastRecorded = lastRecorded
		}
		stat.Chunks += len(metas)
	}

	if err := postings.Err(); err != nil {
		return fmt.Errorf("postings iteration: %w", err)
	}

	return nil
}

func newS3ChunkReader(ctx context.Context, client s3Client, bucket, chunkPrefix string) (*s3ChunkReader, error) {
	keys, err := listObjectKeys(ctx, client, bucket, withTrailingSlash(chunkPrefix))
	if err != nil {
		return nil, err
	}

	segments := make([][]byte, 0, len(keys))
	for _, key := range keys {
		name := strings.TrimPrefix(key, withTrailingSlash(chunkPrefix))
		if strings.Contains(name, "/") {
			continue
		}
		if _, err := strconv.ParseUint(name, 10, 64); err != nil {
			continue
		}

		segment, err := getObject(ctx, client, bucket, key)
		if err != nil {
			return nil, fmt.Errorf("read %s: %w", key, err)
		}
		if err := validateChunkSegment(segment); err != nil {
			return nil, fmt.Errorf("read %s: %w", key, err)
		}
		segments = append(segments, segment)
	}

	if len(segments) == 0 {
		return nil, fmt.Errorf("no chunk segments found")
	}

	return &s3ChunkReader{segments: segments, pool: chunkenc.NewPool()}, nil
}

func listObjectKeys(ctx context.Context, client s3Client, bucket, prefix string) ([]string, error) {
	var keys []string
	var continuationToken *string

	for {
		output, err := client.ListObjectsV2(ctx, &s3.ListObjectsV2Input{
			Bucket:            aws.String(bucket),
			Prefix:            aws.String(prefix),
			ContinuationToken: continuationToken,
		})
		if err != nil {
			return nil, fmt.Errorf("list objects with prefix %q: %w", prefix, err)
		}
		for _, object := range output.Contents {
			keys = append(keys, aws.ToString(object.Key))
		}

		if !aws.ToBool(output.IsTruncated) {
			break
		}
		if output.NextContinuationToken == nil {
			return nil, fmt.Errorf("list objects with prefix %q: truncated response missing continuation token", prefix)
		}
		continuationToken = output.NextContinuationToken
	}

	sort.Strings(keys)
	return keys, nil
}

func getObject(ctx context.Context, client s3Client, bucket, key string) ([]byte, error) {
	output, err := client.GetObject(ctx, &s3.GetObjectInput{
		Bucket: aws.String(bucket),
		Key:    aws.String(key),
	})
	if err != nil {
		return nil, err
	}
	defer output.Body.Close()

	return io.ReadAll(output.Body)
}

func objectKey(prefix, name string) string {
	if prefix == "" {
		return name
	}
	return prefix + "/" + name
}

func withTrailingSlash(prefix string) string {
	if prefix == "" || strings.HasSuffix(prefix, "/") {
		return prefix
	}
	return prefix + "/"
}

type byteSlice []byte

func (b byteSlice) Len() int {
	return len(b)
}

func (b byteSlice) Range(start, end int) []byte {
	return b[start:end]
}

type s3ChunkReader struct {
	segments [][]byte
	pool     chunkenc.Pool
}

func validateChunkSegment(segment []byte) error {
	if len(segment) < chunks.SegmentHeaderSize {
		return fmt.Errorf("invalid segment header")
	}
	if binary.BigEndian.Uint32(segment[:chunks.MagicChunksSize]) != chunks.MagicChunks {
		return fmt.Errorf("invalid chunk segment magic number")
	}
	if segment[chunks.MagicChunksSize] != chunkFormatV1 {
		return fmt.Errorf("invalid chunk segment version %d", segment[chunks.MagicChunksSize])
	}
	return nil
}

func (r *s3ChunkReader) ChunkOrIterable(meta chunks.Meta) (chunkenc.Chunk, chunkenc.Iterable, error) {
	segmentIndex, chunkStart := chunks.BlockChunkRef(meta.Ref).Unpack()
	if segmentIndex >= len(r.segments) {
		return nil, nil, fmt.Errorf("segment index %d out of range", segmentIndex)
	}

	segment := r.segments[segmentIndex]
	if chunkStart+chunks.MaxChunkLengthFieldSize > len(segment) {
		return nil, nil, fmt.Errorf("segment does not include enough bytes for the chunk length")
	}

	chunkLength, lengthSize := binary.Uvarint(segment[chunkStart : chunkStart+chunks.MaxChunkLengthFieldSize])
	if lengthSize <= 0 {
		return nil, nil, fmt.Errorf("read chunk length: invalid varint")
	}

	encodingStart := chunkStart + lengthSize
	chunkDataStart := encodingStart + chunks.ChunkEncodingSize
	chunkDataEnd := chunkDataStart + int(chunkLength)
	chunkEnd := chunkDataEnd + crc32.Size
	if chunkEnd > len(segment) {
		return nil, nil, fmt.Errorf("segment does not include enough bytes for the chunk")
	}

	data := segment[encodingStart:chunkDataEnd]
	wantChecksum := binary.BigEndian.Uint32(segment[chunkDataEnd:chunkEnd])
	if gotChecksum := crc32.Checksum(data, castagnoliTable); gotChecksum != wantChecksum {
		return nil, nil, fmt.Errorf("checksum mismatch expected:%x, actual:%x", wantChecksum, gotChecksum)
	}

	chunk, err := r.pool.Get(chunkenc.Encoding(segment[encodingStart]), segment[chunkDataStart:chunkDataEnd])
	return chunk, nil, err
}
