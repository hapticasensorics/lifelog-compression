# Visual Bundle v1

`Visual Bundle v1` is a simple, legible interchange format for sparse visual extraction from source video.

It is designed for workflows like Guardian where:

- the producer runs locally before upload
- the goal is to preserve useful visual information, not smooth playback
- the output should be easy for other programs to consume without special codec knowledge

## Goals

- keep the format obvious and inspectable
- avoid custom binary containers as the primary truth
- support direct upload to object storage
- support sharding for large imports
- preserve exact provenance back to the original source file

## Non-goals

- preserving a normal video playback experience
- preserving every source frame
- representing the output as a single monolithic database file

## Canonical bundle layout

```text
visual-bundle/
  bundle.json
  manifest.jsonl
  frames/
    00000001.jpg
    00000002.jpg
    00000003.jpg
    ...
```

The canonical truth is:

- `bundle.json`
- `manifest.jsonl`
- `frames/*.jpg`

Everything else is optional.

## Canonical image format

Current default:

- format: `JPEG`
- canvas: `1920x1080`
- fit mode: scale to fit
- padding: black bars
- cadence target: `1 fps`

The producer may choose a different sampling interval or canvas size, but it must record those choices in metadata.

## `bundle.json`

`bundle.json` holds bundle-level metadata.

Example:

```json
{
  "format": "visual-bundle",
  "format_version": 1,
  "bundle_id": "vb_01hxyz...",
  "created_at": "2026-03-30T12:00:00Z",
  "producer": {
    "name": "lifelog-compression",
    "version": "0.1.0"
  },
  "sampling": {
    "requested_interval_ms": 1000,
    "tolerance_before_ms": 500,
    "tolerance_after_ms": 500
  },
  "image": {
    "format": "jpeg",
    "canvas_width": 1920,
    "canvas_height": 1080,
    "fit_mode": "contain",
    "padding": "black"
  },
  "source": {
    "source_id": "source_...",
    "owned_source_id": "owned_source_...",
    "source_type_slug": "gopro"
  }
}
```

## `manifest.jsonl`

`manifest.jsonl` contains one JSON object per frame.

Each line must describe:

- frame identity
- original source linkage
- requested timestamp
- actual timestamp
- where the image lives
- image dimensions
- content hash

Example line:

```json
{
  "frame_id": "frame_000001",
  "frame_relpath": "frames/00000001.jpg",
  "source_relpath": "DCIM/100GOPRO/GX010046.MP4",
  "source_content_hash": "sha256:...",
  "requested_ts_ms": 1000,
  "actual_ts_ms": 1001,
  "width": 1920,
  "height": 1080,
  "content_hash": "sha256:...",
  "extractor": "avasset-image-generator",
  "extractor_version": "0.1.0"
}
```

## Required frame fields

Each manifest row must include:

- `frame_id`
- `frame_relpath`
- `source_relpath`
- `requested_ts_ms`
- `actual_ts_ms`
- `width`
- `height`
- `content_hash`

## Recommended frame fields

- `source_content_hash`
- `extractor`
- `extractor_version`
- `content_rect_x`
- `content_rect_y`
- `content_rect_width`
- `content_rect_height`
- `notes`

## Optional convenience artifacts

These are not canonical truth:

- `preview.mp4`
- contact sheets
- thumbnails
- SQLite caches

They may be present, but consumers must be able to process the bundle without them.

## Local index / cache

A producer may keep a local SQLite index for:

- incremental rebuilds
- retries
- local coverage calculations
- deduplication

But the canonical interchange format remains:

- JSON lines manifest
- ordinary image files

This keeps the format readable without special tooling.

## Upload / transport shape

Local bundle layout and upload transport are separate concerns.

Recommended transport:

- `tar` shards

Each shard should contain the same ordinary files:

```text
shard-0001.tar
  bundle.json
  manifest.jsonl
  frames/...
```

In practice, large imports may split the bundle across multiple shards. In that case:

- each shard must contain a shard-local `manifest.jsonl`
- the union of all shard manifests defines the logical bundle

## Sharding guidance

Recommended shard target:

- `128 MB` to `256 MB`

Reasons:

- large enough to avoid too many tiny uploads
- small enough to retry cheaply
- works well for direct object-store upload

JPEGs are already compressed, so the tar archive should usually remain uncompressed.

## Consumer contract

A consumer should only need:

1. a tar reader or filesystem access
2. a JSON lines reader
3. JPEG decoding

No video decoding should be required after the bundle is produced.

## Compatibility rule

If a future version changes semantics, it must:

- increment `format_version`
- keep old required fields intact when possible
- document migration rules explicitly

## Current recommendation

For now, a producer should emit:

- padded `1920x1080` JPEG frames
- `1 fps` target cadence
- `+/- 0.5s` tolerance
- `bundle.json`
- `manifest.jsonl`
- tar shards for upload
