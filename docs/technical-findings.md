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

- tar shards

Potential manifest contents:

- original file path
- original source id
- requested timestamp
- actual timestamp
- frame filename
- extraction settings
- frame byte size
- width / height

## Current strongest hypothesis

The best near-term system is:

- native compiled macOS sparse extractor
- `1 fps`
- `1920x1080` padded JPEGs
- `+/-0.5s` tolerance
- optional proxy acceleration
- manifest-based upload
