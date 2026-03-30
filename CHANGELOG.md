# Changelog

## 0.1.2 - 2026-03-30

Metadata layout standardization release.

Highlights:

- standardize bundle metadata storage under `metadata/`
- add common `metadata/source-metadata.json` across video bundles
- rename the raw probe artifact to `metadata/source-probe.json`
- keep `metadata/source-mdls.txt` and `metadata/source-xattrs.txt`

## 0.1.1 - 2026-03-30

Metadata preservation patch release.

Highlights:

- keep raw source metadata artifacts in each visual bundle:
  - `metadata/source-metadata.json`
  - `metadata/source-probe.json`
  - `metadata/source-mdls.txt`
  - `metadata/source-xattrs.txt`
- expose the metadata artifact paths in `bundle.json`
- update CLI/docs to make metadata preservation explicit

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
