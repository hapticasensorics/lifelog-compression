# Changelog

## 0.1.0 - 2026-03-30

Initial public release.

Highlights:

- opinionated `extract` command for one video to one visual bundle
- simple `batch` command for recursive directory extraction
- native macOS extraction via `AVAssetImageGenerator`
- Rust library entry points:
  - `extract_to_dir`
  - `extract_directory_to_dir`
  - `load_bundle_metadata`
  - `load_manifest`
- `visual-bundle v1` format docs
- closed-loop validation script and benchmark notes
