use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractRequest {
    pub input: PathBuf,
    pub output_dir: PathBuf,
}

impl ExtractRequest {
    pub fn new(input: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractDefaults {
    pub target_fps_numerator: u32,
    pub target_fps_denominator: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub tolerance_before_ms: u32,
    pub tolerance_after_ms: u32,
    pub image_format: ImageFormat,
}

impl Default for ExtractDefaults {
    fn default() -> Self {
        Self {
            target_fps_numerator: 1,
            target_fps_denominator: 1,
            canvas_width: 1920,
            canvas_height: 1080,
            tolerance_before_ms: 500,
            tolerance_after_ms: 500,
            image_format: ImageFormat::Jpeg,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractResult {
    pub bundle_dir: PathBuf,
    pub frame_count: usize,
}

#[derive(Debug, Serialize)]
struct ProducerMetadata {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct SamplingMetadata {
    requested_interval_ms: u32,
    tolerance_before_ms: u32,
    tolerance_after_ms: u32,
}

#[derive(Debug, Serialize)]
struct ImageMetadata {
    format: String,
    canvas_width: u32,
    canvas_height: u32,
    fit_mode: String,
    padding: String,
}

#[derive(Debug, Serialize)]
struct SourceVideoMetadata {
    source_relpath: String,
    file_size_bytes: u64,
    container_format: String,
    video_codec: String,
    duration_ms: u64,
    width: u32,
    height: u32,
    display_width: u32,
    display_height: u32,
    aspect_ratio: f64,
    avg_frame_rate: String,
    rotation_degrees: i64,
    creation_time: Option<String>,
    timecode: Option<String>,
    has_audio: bool,
}

#[derive(Debug, Serialize)]
struct BundleMetadata {
    format: String,
    format_version: u32,
    bundle_id: String,
    created_at_unix_ms: u128,
    producer: ProducerMetadata,
    sampling: SamplingMetadata,
    image: ImageMetadata,
    source_video: SourceVideoMetadata,
}

#[derive(Debug, Serialize)]
struct ManifestRow {
    frame_id: String,
    frame_relpath: String,
    source_relpath: String,
    requested_ts_ms: u64,
    actual_ts_ms: u64,
    width: u32,
    height: u32,
    source_width: u32,
    source_height: u32,
    source_aspect_ratio: f64,
    content_rect_x: u32,
    content_rect_y: u32,
    content_rect_width: u32,
    content_rect_height: u32,
    extractor: String,
    extractor_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeExtractPayload {
    source_video: NativeSourceVideoMetadata,
    frames: Vec<NativeFrameRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSourceVideoMetadata {
    file_size_bytes: u64,
    container_format: String,
    video_codec: String,
    duration_ms: u64,
    width: u32,
    height: u32,
    display_width: u32,
    display_height: u32,
    aspect_ratio: f64,
    avg_frame_rate: String,
    rotation_degrees: i64,
    creation_time: Option<String>,
    timecode: Option<String>,
    has_audio: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeFrameRecord {
    frame_relpath: String,
    requested_ts_ms: u64,
    actual_ts_ms: u64,
}

fn bundle_id() -> Result<(String, u128), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    let millis = now.as_millis();
    Ok((format!("vb_{millis:x}"), millis))
}

fn content_rect(
    source_width: u32,
    source_height: u32,
    canvas_width: u32,
    canvas_height: u32,
) -> (u32, u32, u32, u32) {
    let scale = f64::min(
        canvas_width as f64 / source_width as f64,
        canvas_height as f64 / source_height as f64,
    );
    let w = (source_width as f64 * scale).round() as u32;
    let h = (source_height as f64 * scale).round() as u32;
    let x = (canvas_width - w) / 2;
    let y = (canvas_height - h) / 2;
    (x, y, w, h)
}

fn helper_source_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("swift")
        .join("apple_native_extract.swift")
}

fn helper_binary_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("apple-native-extractor")
}

fn ensure_native_extractor() -> Result<PathBuf, String> {
    let source = helper_source_path();
    let binary = helper_binary_path();

    let needs_compile = if !binary.exists() {
        true
    } else {
        let source_mtime = fs::metadata(&source)
            .and_then(|meta| meta.modified())
            .map_err(|err| format!("failed to stat {}: {err}", source.display()))?;
        let binary_mtime = fs::metadata(&binary)
            .and_then(|meta| meta.modified())
            .map_err(|err| format!("failed to stat {}: {err}", binary.display()))?;
        source_mtime > binary_mtime
    };

    if needs_compile {
        let parent = binary
            .parent()
            .ok_or_else(|| format!("bad helper binary path: {}", binary.display()))?;
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        let output = Command::new("swiftc")
            .args([
                "-O",
                source
                    .to_str()
                    .ok_or_else(|| format!("non-utf8 helper source path: {}", source.display()))?,
                "-o",
                binary
                    .to_str()
                    .ok_or_else(|| format!("non-utf8 helper binary path: {}", binary.display()))?,
            ])
            .output()
            .map_err(|err| format!("failed to compile native extractor: {err}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("swiftc failed: {stderr}"));
        }
    }

    Ok(binary)
}

fn run_native_extract(
    request: &ExtractRequest,
    defaults: &ExtractDefaults,
) -> Result<NativeExtractPayload, String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        let _ = defaults;
        return Err("lifelog-compression extract currently requires macOS".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let binary = ensure_native_extractor()?;
        let interval_seconds =
            f64::from(defaults.target_fps_denominator) / f64::from(defaults.target_fps_numerator);
        let output = Command::new(binary)
            .args([
                "--input",
                request
                    .input
                    .to_str()
                    .ok_or_else(|| format!("non-utf8 input path: {}", request.input.display()))?,
                "--output-dir",
                request.output_dir.to_str().ok_or_else(|| {
                    format!("non-utf8 output dir path: {}", request.output_dir.display())
                })?,
                "--interval-seconds",
                &interval_seconds.to_string(),
                "--tolerance-before-ms",
                &defaults.tolerance_before_ms.to_string(),
                "--tolerance-after-ms",
                &defaults.tolerance_after_ms.to_string(),
                "--canvas-width",
                &defaults.canvas_width.to_string(),
                "--canvas-height",
                &defaults.canvas_height.to_string(),
                "--jpeg-quality",
                "0.75",
            ])
            .output()
            .map_err(|err| format!("failed to run native extractor: {err}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("native extractor failed: {stderr}"));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|err| format!("failed to parse native extractor json: {err}"))
    }
}

pub fn extract(request: &ExtractRequest) -> Result<ExtractResult, String> {
    if !request.input.exists() {
        return Err(format!("input does not exist: {}", request.input.display()));
    }
    if !request.input.is_file() {
        return Err(format!("input is not a file: {}", request.input.display()));
    }

    let defaults = ExtractDefaults::default();
    if request.output_dir.exists() {
        fs::remove_dir_all(&request.output_dir)
            .map_err(|err| format!("failed to remove {}: {err}", request.output_dir.display()))?;
    }
    fs::create_dir_all(&request.output_dir)
        .map_err(|err| format!("failed to create {}: {err}", request.output_dir.display()))?;

    let native = run_native_extract(request, &defaults)?;
    let source = SourceVideoMetadata {
        source_relpath: request.input.to_string_lossy().to_string(),
        file_size_bytes: native.source_video.file_size_bytes,
        container_format: native.source_video.container_format,
        video_codec: native.source_video.video_codec,
        duration_ms: native.source_video.duration_ms,
        width: native.source_video.width,
        height: native.source_video.height,
        display_width: native.source_video.display_width,
        display_height: native.source_video.display_height,
        aspect_ratio: native.source_video.aspect_ratio,
        avg_frame_rate: native.source_video.avg_frame_rate,
        rotation_degrees: native.source_video.rotation_degrees,
        creation_time: native.source_video.creation_time,
        timecode: native.source_video.timecode,
        has_audio: native.source_video.has_audio,
    };

    let (content_rect_x, content_rect_y, content_rect_width, content_rect_height) = content_rect(
        source.width,
        source.height,
        defaults.canvas_width,
        defaults.canvas_height,
    );

    let manifest_path = request.output_dir.join("manifest.jsonl");
    let manifest_file = File::create(&manifest_path)
        .map_err(|err| format!("failed to create {}: {err}", manifest_path.display()))?;
    let mut manifest = BufWriter::new(manifest_file);

    for (index, row) in native.frames.iter().enumerate() {
        let manifest_row = ManifestRow {
            frame_id: format!("frame_{:08}", index + 1),
            frame_relpath: row.frame_relpath.clone(),
            source_relpath: source.source_relpath.clone(),
            requested_ts_ms: row.requested_ts_ms,
            actual_ts_ms: row.actual_ts_ms,
            width: defaults.canvas_width,
            height: defaults.canvas_height,
            source_width: source.width,
            source_height: source.height,
            source_aspect_ratio: source.aspect_ratio,
            content_rect_x,
            content_rect_y,
            content_rect_width,
            content_rect_height,
            extractor: "avasset-image-generator".to_string(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        serde_json::to_writer(&mut manifest, &manifest_row)
            .map_err(|err| format!("failed to write manifest row: {err}"))?;
        manifest
            .write_all(b"\n")
            .map_err(|err| format!("failed to finish manifest row: {err}"))?;
    }
    manifest
        .flush()
        .map_err(|err| format!("failed to flush manifest: {err}"))?;

    let (id, created_at_unix_ms) = bundle_id()?;
    let bundle = BundleMetadata {
        format: "visual-bundle".to_string(),
        format_version: 1,
        bundle_id: id,
        created_at_unix_ms,
        producer: ProducerMetadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        sampling: SamplingMetadata {
            requested_interval_ms: 1000,
            tolerance_before_ms: defaults.tolerance_before_ms,
            tolerance_after_ms: defaults.tolerance_after_ms,
        },
        image: ImageMetadata {
            format: "jpeg".to_string(),
            canvas_width: defaults.canvas_width,
            canvas_height: defaults.canvas_height,
            fit_mode: "contain".to_string(),
            padding: "black".to_string(),
        },
        source_video: source,
    };

    let bundle_path = request.output_dir.join("bundle.json");
    let bundle_file = File::create(&bundle_path)
        .map_err(|err| format!("failed to create {}: {err}", bundle_path.display()))?;
    serde_json::to_writer_pretty(bundle_file, &bundle)
        .map_err(|err| format!("failed to write bundle metadata: {err}"))?;

    Ok(ExtractResult {
        bundle_dir: request.output_dir.clone(),
        frame_count: native.frames.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::content_rect;

    #[test]
    fn content_rect_letterboxes_four_three_into_sixteen_nine() {
        let rect = content_rect(1920, 1440, 1920, 1080);
        assert_eq!(rect, (240, 0, 1440, 1080));
    }

    #[test]
    fn content_rect_keeps_native_sixteen_nine_full_frame() {
        let rect = content_rect(1920, 1080, 1920, 1080);
        assert_eq!(rect, (0, 0, 1920, 1080));
    }
}
