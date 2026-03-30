# Visual Bundle v1

`Visual Bundle v1` is a simple, legible interchange format for sparse visual extraction from source video.

It is designed for workflows like Guardian where:

- the producer runs locally before upload
- the goal is to preserve useful visual information, not smooth playback
- the output should be easy for other programs to consume without special codec knowledge

It is also designed to be emitted by an opinionated tool with very little public configuration.

## Goals

- keep the format obvious and inspectable
- avoid custom binary containers as the primary truth
- support direct upload to object storage
- preserve clear provenance back to the original source file

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

This format is intentionally scoped to one input video file per bundle.

That means:

- one input video
- one output bundle
- one natural upload unit

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
  },
  "source_video": {
    "source_relpath": "DCIM/100GOPRO/GX010046.MP4",
    "file_size_bytes": 1686181009,
    "container_format": "mp4",
    "video_codec": "hvc1",
    "duration_ms": 112720,
    "width": 2704,
    "height": 2028,
    "display_width": 2704,
    "display_height": 2028,
    "aspect_ratio": 1.333333,
    "avg_frame_rate": "120000/1001",
    "rotation_degrees": 0,
    "creation_time": "2026-02-05T05:08:39Z",
    "timecode": "21:07:23:023",
    "has_audio": true
  }
}
```

The `source_video` block is for metadata intrinsic to the input media file itself.

That is in scope for the tool.

External sidecars and package-level vendor artifacts are intentionally out of scope for the core format.

## `manifest.jsonl`

`manifest.jsonl` contains one JSON object per frame.

Each line must describe:

- frame identity
- original source linkage
- requested timestamp
- actual timestamp
- where the image lives
- image dimensions

Example line:

```json
{
  "frame_id": "frame_000001",
  "frame_relpath": "frames/00000001.jpg",
  "source_relpath": "DCIM/100GOPRO/GX010046.MP4",
  "requested_ts_ms": 1000,
  "actual_ts_ms": 1001,
  "width": 1920,
  "height": 1080,
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

## Recommended frame fields

- `extractor`
- `extractor_version`
- `source_width`
- `source_height`
- `source_aspect_ratio`
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

- a single tar archive of the bundle, if packaging is needed

Example:

```text
visual-bundle.tar
  bundle.json
  manifest.jsonl
  frames/...
```

Because one input video maps to one bundle, the bundle naturally maps to one upload unit.

The tool itself should not expose sharding as a public option.

If a higher-level embedding system wants to split or aggregate multiple bundles later, that is outside the core tool boundary.

## Consumer contract

A consumer should only need:

1. a tar reader or filesystem access
2. a JSON lines reader
3. JPEG decoding

No video decoding should be required after the bundle is produced.

Consumers should also be able to read the original video geometry and timing metadata directly from `bundle.json`.

The format does not require hashing. If an embedding app wants bundle-level or source-level hashes, it should add them outside the core format.

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
- a single tar archive if upload packaging is needed

## Tool opinionation

This tool should stay intentionally opinionated.

It should have very few public configuration options.

Good fixed defaults:

- `1 fps`
- padded `1920x1080`
- JPEG
- `+/- 0.5s`
- one input video -> one output bundle

The more advanced orchestration concerns should live in the caller, not in this tool.
