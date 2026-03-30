# lifelog-compression

Experiments and technical notes for compressing lifelog-style visual data before upload.

The current goal is not to preserve smooth playback-quality video. The goal is to preserve useful visual information for downstream memory, search, coverage, and analysis workflows while making local preprocessing and upload much faster.

This repo is becoming a reusable utility that can be embedded into or called from other programs before upload.

The tool is intentionally opinionated:

- one input video file
- one output visual bundle
- preserve intrinsic media metadata from that video
- very few public knobs

## Current default recipe

The strongest practical default found so far is:

- sample at `1 fps`
- extract images, not low-fps video
- use Apple-native `AVAssetImageGenerator` on macOS
- use requested time tolerance of `+/- 0.5s`
- scale to fit inside `1920x1080`
- pad with black bars to a fixed `16:9` canvas
- write JPEG frames
- store a manifest of requested timestamps and actual timestamps

This preserves a regular sparse timeline while staying very fast on Apple Silicon.

## Main findings

### 1. Sparse image extraction beats low-fps video transcoding

For Guardian-style use cases, sparse visual checkpoints are more useful than preserving a conventional video container.

That means the canonical derivative should be:

- timestamped JPEG frames
- plus metadata / manifest

instead of:

- HEVC or H.264 low-fps video

### 2. Apple-native extraction is the current winner

Using a compiled native macOS utility built on `AVAssetImageGenerator` was dramatically faster than the earlier ffmpeg-based pipelines for this use case.

Important note:

- early slow results were distorted by running the benchmark through the `swift` interpreter
- once compiled as a native binary, the same extraction approach became much faster

### 3. Fixed output shape helps

The current preferred output shape is:

- fixed `1920x1080`
- scale-to-fit
- black padding bars instead of cropping

Reasons:

- preserves all source pixels
- makes downstream backend processing simpler
- avoids source-specific layout branching
- keeps the representation tool-friendly

### 4. Proxy media is a major bonus, but not required

Some cameras provide low-resolution companion media such as `.lrv` or similar preview files.

Those are excellent acceleration paths when available, but the core method should still work well on originals so the system is not dependent on vendor-specific support.

## Benchmarks

These are the most important benchmark results so far.

### Light original clip

Source:

- `1920x1440`
- `24 fps`
- HEVC

Compiled Apple-native sparse extraction:

- `1080p`, `+/-0.5s`: about `27.0x realtime`
- `720p`, `+/-0.5s`: about `31.0x realtime`
- `540p`, `+/-0.5s`: about `36.0x realtime`

### Heavy original clip

Source:

- `2704x2028`
- `120 fps`
- HEVC

Compiled Apple-native sparse extraction:

- `1080p`, `+/-0.5s`: about `34.9x realtime`
- `720p`, `+/-0.5s`: about `44.0x realtime`
- `540p`, `+/-0.5s`: about `53.2x realtime`

For the heavy clip, timestamp drift remained small:

- max absolute error: about `0.112s`
- mean absolute error: about `0.056s`

### Proxy result

On a matched proxy / preview file:

- Apple-native extraction at `1080p`, `+/-0.5s`: about `20.0x realtime`

Proxy-first is still valuable, but the main discovery is that the general-purpose original-file path is already fast enough.

## What did not win

### Low-fps HEVC transcoding

Re-encoding to low-fps video was not the best representation or the best speed path for this problem.

### ffmpeg exact-ish decode/filter pipelines

These were often much slower than the compiled Apple-native sparse extraction path.

### Forward-only tolerance as the default

Testing `0 / +1.0s` and `0 / +1.5s` tolerance windows showed:

- faster extraction on some light clips
- but too many duplicated or collapsed actual timestamps

So the best current default remains:

- symmetric `+/- 0.5s`

### video-sampler as the main fast path

`video-sampler` was useful as an exploratory reference, especially for keyframe/dedup ideas, but it was not the raw speed winner for the core sparse extraction task.

## Recommended product behavior

Current recommendation:

1. Run a quick local probe on each source file.
2. If a trusted preview / proxy companion exists, optionally use it.
3. Otherwise, use the general-purpose Apple-native sparse extraction path on the original.
4. Produce:
   - padded `1920x1080` JPEG frames
   - manifest with requested and actual timestamps
5. Upload the sparse visual bundle rather than a recompressed video derivative.

The embedding app or service can decide how to combine or upload many bundles, but that is outside the core tool itself.

## Future work

- define the exact manifest format
- define the local bundle layout
- define the upload shard format
- benchmark folder-scale extraction on large real datasets
- test whether `720p` or `540p` is a better default than `1080p`
- add optional proxy discovery rules for common camera ecosystems

## Related notes

Detailed notes live in [docs/technical-findings.md](docs/technical-findings.md).
The current real-workload benchmark lives in [docs/benchmark-2026-03-30-real-workload.md](docs/benchmark-2026-03-30-real-workload.md).

## Format spec

The first concrete interchange spec lives in [docs/visual-bundle-v1.md](docs/visual-bundle-v1.md).

## Package shape

The repo now includes a Rust crate plus CLI:

```bash
cargo run -- spec
```

Current public commands are intentionally minimal:

- `extract`
- `spec`
- `help`

The implementation is intentionally light right now; the technical findings and bundle spec are the first-class output so far.

The long-term CLI should stay small rather than becoming highly configurable.
