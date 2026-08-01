package analyzer

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"hash/crc32"
	"testing"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/aws-sdk-go-v2/service/s3/types"
	"github.com/prometheus/prometheus/tsdb/chunkenc"
	"github.com/prometheus/prometheus/tsdb/chunks"
)

func TestParseS3URI(t *testing.T) {
	tests := []struct {
		name    string
		uri     string
		bucket  string
		prefix  string
		wantErr bool
	}{
		{name: "bucket root", uri: "s3://metrics", bucket: "metrics"},
		{name: "bucket prefix", uri: "s3://metrics/tenant-a/blocks/", bucket: "metrics", prefix: "tenant-a/blocks"},
		{name: "wrong scheme", uri: "file:///tmp/blocks", wantErr: true},
		{name: "missing bucket", uri: "s3:///blocks", wantErr: true},
		{name: "query", uri: "s3://metrics/blocks?versionId=1", wantErr: true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			bucket, prefix, err := parseS3URI(tt.uri)
			if tt.wantErr {
				if err == nil {
					t.Fatal("expected an error")
				}
				return
			}
			if err != nil {
				t.Fatalf("parseS3URI() error = %v", err)
			}
			if bucket != tt.bucket || prefix != tt.prefix {
				t.Fatalf("parseS3URI() = (%q, %q), want (%q, %q)", bucket, prefix, tt.bucket, tt.prefix)
			}
		})
	}
}

func TestFindBlockPrefixes(t *testing.T) {
	client := &fakeS3Client{
		listOutput: &s3.ListObjectsV2Output{
			Contents: []types.Object{{Key: aws.String("tenant/meta.json")}},
			CommonPrefixes: []types.CommonPrefix{
				{Prefix: aws.String("tenant/01ARZ3NDEKTSV4RRFFQ69G5FAV/")},
				{Prefix: aws.String("tenant/not-a-block/")},
			},
		},
		headErrors: map[string]error{
			"tenant/not-a-block/meta.json": &types.NotFound{},
		},
	}

	got, err := findBlockPrefixes(context.Background(), client, "metrics", "tenant")
	if err != nil {
		t.Fatalf("findBlockPrefixes() error = %v", err)
	}
	want := []string{"tenant", "tenant/01ARZ3NDEKTSV4RRFFQ69G5FAV"}
	if !equalStrings(got, want) {
		t.Fatalf("findBlockPrefixes() = %q, want %q", got, want)
	}
}

func TestTopNMetricsSkipsUnreadableBlock(t *testing.T) {
	client := &fakeS3Client{
		listOutput: &s3.ListObjectsV2Output{
			Contents: []types.Object{{Key: aws.String("tenant/meta.json")}},
		},
		getErrors: map[string]error{
			"tenant/index": errors.New("S3 GetObject: SlowDownRead"),
		},
	}

	stats, err := TopNMetrics(context.Background(), client, "s3://metrics/tenant")
	if err != nil {
		t.Fatalf("TopNMetrics() error = %v", err)
	}
	if len(stats) != 0 {
		t.Fatalf("TopNMetrics() = %v, want no metrics", stats)
	}
}

func TestS3ChunkReaderChunk(t *testing.T) {
	chunk := chunkenc.NewXORChunk()
	chunkData := chunk.Bytes()
	data := append([]byte{byte(chunk.Encoding())}, chunkData...)
	segment := make([]byte, chunks.SegmentHeaderSize)
	binary.BigEndian.PutUint32(segment, chunks.MagicChunks)
	segment[chunks.MagicChunksSize] = chunkFormatV1

	length := make([]byte, binary.MaxVarintLen32)
	length = length[:binary.PutUvarint(length, uint64(len(chunkData)))]
	segment = append(segment, length...)
	segment = append(segment, data...)
	checksum := make([]byte, 4)
	binary.BigEndian.PutUint32(checksum, crc32.Checksum(data, castagnoliTable))
	segment = append(segment, checksum...)

	reader := &s3ChunkReader{segments: [][]byte{segment}, pool: chunkenc.NewPool()}
	got, _, err := reader.ChunkOrIterable(chunks.Meta{Ref: chunks.ChunkRef(chunks.NewBlockChunkRef(0, chunks.SegmentHeaderSize))})
	if err != nil {
		t.Fatalf("ChunkOrIterable() error = %v", err)
	}
	if !bytes.Equal(got.Bytes(), chunkData) {
		t.Fatalf("ChunkOrIterable() bytes = %x, want %x", got.Bytes(), chunkData)
	}
}

type fakeS3Client struct {
	listOutput *s3.ListObjectsV2Output
	headErrors map[string]error
	getErrors  map[string]error
}

func (f *fakeS3Client) ListObjectsV2(_ context.Context, _ *s3.ListObjectsV2Input, _ ...func(*s3.Options)) (*s3.ListObjectsV2Output, error) {
	return f.listOutput, nil
}

func (f *fakeS3Client) HeadObject(_ context.Context, input *s3.HeadObjectInput, _ ...func(*s3.Options)) (*s3.HeadObjectOutput, error) {
	if err := f.headErrors[aws.ToString(input.Key)]; err != nil {
		return nil, err
	}
	return &s3.HeadObjectOutput{}, nil
}

func (f *fakeS3Client) GetObject(_ context.Context, input *s3.GetObjectInput, _ ...func(*s3.Options)) (*s3.GetObjectOutput, error) {
	if err := f.getErrors[aws.ToString(input.Key)]; err != nil {
		return nil, err
	}
	return nil, nil
}

func equalStrings(got, want []string) bool {
	if len(got) != len(want) {
		return false
	}
	for i := range got {
		if got[i] != want[i] {
			return false
		}
	}
	return true
}
