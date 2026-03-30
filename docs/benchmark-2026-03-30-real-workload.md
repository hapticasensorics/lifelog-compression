# Real Workload Benchmark

Date:

- `2026-03-30`

Method:

- compiled native Apple extractor
- `AVAssetImageGenerator`
- `1 fps`
- padded `1920x1080` JPEG output
- tolerance before: `0.5s`
- tolerance after: `0.5s`

## Workload shape

### `Camera02`

Observed local dataset:

- `461` video files
- about `176.319 GB`
- about `10.263 h`
- all `24 fps`
- dimensions:
  - `1920x1440`: `148`
  - `1920x1080`: `313`

Representative sample used for benchmark:

- `30` clips total
- `10` clips from the `1920x1440` group
- `20` clips from the `1920x1080` group
- total duration: `450s`

### `100GOPRO`

Observed local dataset:

- `6` video files
- about `4.502 GB`
- about `0.131 h`
- mixed `60 fps` and `120 fps`

Benchmark used:

- all `6` clips
- total duration: about `471.371s`

## Results

### `Camera02` originals

- files: `30`
- total duration: `450s`
- total input: `2.291 GB`
- total output: `0.191 GB`
- total frames: `450`
- elapsed: `19.843s`
- aggregate speed: about `22.68x realtime`

### `Camera02` matched proxies

- files: `30`
- total duration: `450s`
- total input: `0.502 GB`
- total output: `0.162 GB`
- total frames: `450`
- elapsed: `13.846s`
- aggregate speed: about `32.50x realtime`

### `100GOPRO` originals

- files: `6`
- total duration: about `471.371s`
- total input: `4.502 GB`
- total output: `0.173 GB`
- total frames: `474`
- elapsed: `15.449s`
- aggregate speed: about `30.51x realtime`

## Interpretation

These numbers are the most important current result in the repo.

They show that the chosen default:

- sparse extraction
- native Apple path
- `1 fps`
- padded `1080p`

already performs well on a representative real workload.

Important takeaways:

1. The general-purpose original-file path is strong enough.
   - `Camera02` originals: about `22.68x realtime`
   - `100GOPRO` originals: about `30.51x realtime`

2. Proxies still help, but they are optional.
   - `Camera02` proxies: about `32.50x realtime`

3. This is fast enough to be practical before upload.

## Current default recommendation

Keep the current default:

- compiled native Apple extraction
- `1 fps`
- padded `1920x1080`
- JPEG
- `+/-0.5s` tolerance

And treat proxy detection as an accelerator rather than a dependency.
