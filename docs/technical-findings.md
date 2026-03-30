# Technical Findings

## Problem framing

We do not primarily need normal video playback.

We need:

- visual checkpoints
- time coverage
- source linkage
- fast local preprocessing
- fast upload
- a representation that is easy for backend and analysis tools to consume

That changes the optimization target significantly.

## Why sparse frames are the right representation

A low-fps video container still pays many costs that are not central to the product:

- video muxing
- GOP / inter-frame structure
- playback semantics
- extra codec complexity

Sparse frames are simpler:

- each frame is individually addressable
- timestamps are explicit
- downstream processing is easier
- upload packaging is simpler

## Why fixed `16:9` with padding

We chose:

- `1920x1080`
- scale to fit
- black padding bars

instead of cropping or variable aspect ratios.

This is a good systems choice because it:

- preserves all source content
- keeps downstream ML / OCR / thumbnail processing simple
- avoids weird shape branching
- gives the UI a consistent canvas

## Apple-native extraction notes

The current benchmark utility uses:

- `AVAssetImageGenerator`
- `appliesPreferredTrackTransform = true`
- `maximumSize`
- explicit time tolerances

The winning default so far is:

- interval: `1.0s`
- tolerance before: `0.5s`
- tolerance after: `0.5s`

This gave strong speed with low timestamp error.

### Why the compiled version mattered

Running the experiment through `swift` script mode included compilation / startup overhead. Compiling the benchmark into a native binary removed that distortion and revealed the actual extractor performance.

That suggests product code should use:

- a compiled native implementation

not:

- repeated shelling out to `swift` script mode

## Intrinsic metadata boundary

The tool should preserve metadata intrinsic to the input video file, for example:

- original width and height
- original display aspect ratio
- frame rate
- codec
- duration
- rotation / transform
- creation time if present
- timecode if present
- audio presence

That is the right scope for a reusable extraction utility.

External companion files and vendor package metadata should be handled by the caller if needed.

## Tolerance findings

### Symmetric `+/-0.5s`

Best current default because it:

- remains fast
- preserves dense one-per-second coverage
- avoids collapsing multiple requests onto the same actual frame too often

### Forward-only windows

Testing:

- `0 / +1.0s`
- `0 / +1.5s`

showed:

- faster extraction on some lighter clips
- but too many duplicated actual timestamps

So this is not the default even though it can be attractive for speed.

## Resolution findings

Even though the current working default is still documented as padded `1080p`, smaller outputs are very compelling:

- `720p` gives large speedups
- `540p` gives even larger speedups

This means the final product default may eventually shift lower than `1080p`, but for now `1080p` remains a straightforward high-confidence baseline.

## Proxy findings

Proxy / preview media can still be extremely valuable:

- faster local decode
- smaller source bytes
- often enough fidelity for sparse visual checkpoints

However, the core lesson is now:

- proxies are an optimization
- not a requirement

That is important because it means we do not need camera-specific support to get a good system off the ground.

## Practical next architecture

Likely output bundle:

- `frames/`
- `manifest`

Potential upload packaging:

- one tar per bundle

Potential manifest contents:

- original file path
- original source width / height
- original aspect ratio
- requested timestamp
- actual timestamp
- frame filename
- extraction settings
- width / height

## Current strongest hypothesis

The best near-term system is:

- native compiled macOS sparse extractor
- `1 fps`
- `1920x1080` padded JPEGs
- `+/-0.5s` tolerance
- optional proxy acceleration
- one-video-per-bundle manifest-based upload

## Packaging direction

The package surface should be Rust-first:

- Rust crate for the public interface
- Rust CLI for manual use and testing
- a small native macOS extractor hidden behind that interface

This keeps the tool easy to embed in:

- desktop apps
- backend jobs
- other local tooling

while still allowing the actual extraction backend to evolve.

## Current implementation note

The package currently avoids source and frame hashing on purpose.

Reason:

- hashes are not intrinsic media metadata
- they materially slow the shipped end-to-end path
- the embedding app can add them later if it actually needs them
