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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItemResult {
    pub input: PathBuf,
    pub bundle_dir: PathBuf,
    pub frame_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExtractResult {
    pub output_root: PathBuf,
    pub item_count: usize,
    pub total_frame_count: usize,
    pub items: Vec<BatchItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProducerMetadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamplingMetadata {
    pub requested_interval_ms: u32,
    pub tolerance_before_ms: u32,
    pub tolerance_after_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageMetadata {
    pub format: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub fit_mode: String,
    pub padding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceVideoMetadata {
    pub source_relpath: String,
    pub file_size_bytes: u64,
    pub container_format: String,
    pub video_codec: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub display_width: u32,
    pub display_height: u32,
    pub aspect_ratio: f64,
    pub avg_frame_rate: String,
    pub rotation_degrees: i64,
    pub creation_time: Option<String>,
    pub timecode: Option<String>,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleMetadata {
    pub format: String,
    pub format_version: u32,
    pub bundle_id: String,
    pub created_at_unix_ms: u128,
    pub producer: ProducerMetadata,
    pub sampling: SamplingMetadata,
    pub image: ImageMetadata,
    pub source_video: SourceVideoMetadata,
    pub metadata_artifacts: MetadataArtifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetadataArtifacts {
    pub ffprobe_relpath: String,
    pub mdls_relpath: String,
    pub xattrs_relpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestRow {
    pub frame_id: String,
    pub frame_relpath: String,
    pub source_relpath: String,
    pub requested_ts_ms: u64,
    pub actual_ts_ms: u64,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub source_aspect_ratio: f64,
    pub content_rect_x: u32,
    pub content_rect_y: u32,
    pub content_rect_width: u32,
    pub content_rect_height: u32,
    pub extractor: String,
    pub extractor_version: String,
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

fn run_command(program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`{program}` failed: {stderr}"));
    }
    Ok(output.stdout)
}

fn run_command_allow_failure(program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    Ok(output.stdout)
}

fn write_source_metadata_artifacts(input: &Path, output_dir: &Path) -> Result<MetadataArtifacts, String> {
    let input_str = input
        .to_str()
        .ok_or_else(|| format!("non-utf8 input path: {}", input.display()))?;

    let ffprobe_json = run_command(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-show_programs",
            "-print_format",
            "json",
            input_str,
        ],
    )?;
    fs::write(output_dir.join("source-ffprobe.json"), ffprobe_json)
        .map_err(|err| format!("failed to write ffprobe metadata: {err}"))?;

    let mdls_text = run_command("mdls", &[input_str])?;
    fs::write(output_dir.join("source-mdls.txt"), mdls_text)
        .map_err(|err| format!("failed to write mdls metadata: {err}"))?;

    let xattrs_text = run_command_allow_failure("xattr", &["-l", input_str])?;
    fs::write(output_dir.join("source-xattrs.txt"), xattrs_text)
        .map_err(|err| format!("failed to write xattr metadata: {err}"))?;

    Ok(MetadataArtifacts {
        ffprobe_relpath: "source-ffprobe.json".to_string(),
        mdls_relpath: "source-mdls.txt".to_string(),
        xattrs_relpath: "source-xattrs.txt".to_string(),
    })
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
    let metadata_artifacts = write_source_metadata_artifacts(&request.input, &request.output_dir)?;
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
        metadata_artifacts,
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

pub fn extract_to_dir(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<ExtractResult, String> {
    let request = ExtractRequest::new(input, output_dir);
    extract(&request)
}

pub fn load_bundle_metadata(bundle_dir: impl AsRef<Path>) -> Result<BundleMetadata, String> {
    let path = bundle_dir.as_ref().join("bundle.json");
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn load_manifest(bundle_dir: impl AsRef<Path>) -> Result<Vec<ManifestRow>, String> {
    let path = bundle_dir.as_ref().join("manifest.jsonl");
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ManifestRow>(line)
                .map_err(|err| format!("failed to parse manifest row in {}: {err}", path.display()))
        })
        .collect()
}

fn is_supported_video_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        &ext.to_ascii_lowercase()[..],
        "mp4" | "mov" | "m4v" | "avi" | "mkv" | "mts" | "m2ts" | "lrv" | "mpg" | "mpeg"
    )
}

fn collect_video_inputs(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_video_inputs(&path, out)?;
        } else if path.is_file() && is_supported_video_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn bundle_dir_for_input(input_root: &Path, input: &Path, output_root: &Path) -> Result<PathBuf, String> {
    let relative = input.strip_prefix(input_root).map_err(|err| {
        format!(
            "failed to compute relative path for {} under {}: {err}",
            input.display(),
            input_root.display()
        )
    })?;
    let mut bundle_dir = output_root.join(relative);
    bundle_dir.set_extension("");
    Ok(bundle_dir)
}

pub fn extract_directory_to_dir(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<BatchExtractResult, String> {
    let input_root = input_root.as_ref();
    let output_root = output_root.as_ref();

    if !input_root.exists() {
        return Err(format!("input root does not exist: {}", input_root.display()));
    }
    if !input_root.is_dir() {
        return Err(format!("input root is not a directory: {}", input_root.display()));
    }

    if output_root.exists() {
        fs::remove_dir_all(output_root)
            .map_err(|err| format!("failed to remove {}: {err}", output_root.display()))?;
    }
    fs::create_dir_all(output_root)
        .map_err(|err| format!("failed to create {}: {err}", output_root.display()))?;

    let mut inputs = Vec::new();
    collect_video_inputs(input_root, &mut inputs)?;
    inputs.sort();

    let mut items = Vec::with_capacity(inputs.len());
    let mut total_frame_count = 0;

    for input in inputs {
        let bundle_dir = bundle_dir_for_input(input_root, &input, output_root)?;
        let result = extract_to_dir(&input, &bundle_dir)?;
        total_frame_count += result.frame_count;
        items.push(BatchItemResult {
            input,
            bundle_dir: result.bundle_dir,
            frame_count: result.frame_count,
        });
    }

    Ok(BatchExtractResult {
        output_root: output_root.to_path_buf(),
        item_count: items.len(),
        total_frame_count,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::{BundleMetadata, ImageMetadata, ManifestRow, MetadataArtifacts, ProducerMetadata, SamplingMetadata, SourceVideoMetadata, bundle_dir_for_input, content_rect, is_supported_video_file, load_bundle_metadata, load_manifest};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir().join(format!("lifelog-compression-{name}-{millis}"))
    }

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

    #[test]
    fn load_bundle_metadata_reads_bundle_json() {
        let dir = temp_dir("bundle");
        fs::create_dir_all(&dir).unwrap();
        let bundle = BundleMetadata {
            format: "visual-bundle".to_string(),
            format_version: 1,
            bundle_id: "vb_test".to_string(),
            created_at_unix_ms: 1,
            producer: ProducerMetadata {
                name: "lifelog-compression".to_string(),
                version: "0.1.0".to_string(),
            },
            sampling: SamplingMetadata {
                requested_interval_ms: 1000,
                tolerance_before_ms: 500,
                tolerance_after_ms: 500,
            },
            image: ImageMetadata {
                format: "jpeg".to_string(),
                canvas_width: 1920,
                canvas_height: 1080,
                fit_mode: "contain".to_string(),
                padding: "black".to_string(),
            },
            source_video: SourceVideoMetadata {
                source_relpath: "clip.mp4".to_string(),
                file_size_bytes: 123,
                container_format: "mp4".to_string(),
                video_codec: "hvc1".to_string(),
                duration_ms: 1000,
                width: 1920,
                height: 1080,
                display_width: 1920,
                display_height: 1080,
                aspect_ratio: 16.0 / 9.0,
                avg_frame_rate: "24.000".to_string(),
                rotation_degrees: 0,
                creation_time: None,
                timecode: None,
                has_audio: true,
            },
            metadata_artifacts: MetadataArtifacts {
                ffprobe_relpath: "source-ffprobe.json".to_string(),
                mdls_relpath: "source-mdls.txt".to_string(),
                xattrs_relpath: "source-xattrs.txt".to_string(),
            },
        };
        fs::write(
            dir.join("bundle.json"),
            serde_json::to_string_pretty(&bundle).unwrap(),
        )
        .unwrap();

        let loaded = load_bundle_metadata(&dir).unwrap();
        assert_eq!(loaded, bundle);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_manifest_reads_jsonl_rows() {
        let dir = temp_dir("manifest");
        fs::create_dir_all(&dir).unwrap();
        let row = ManifestRow {
            frame_id: "frame_00000001".to_string(),
            frame_relpath: "frames/00000001.jpg".to_string(),
            source_relpath: "clip.mp4".to_string(),
            requested_ts_ms: 0,
            actual_ts_ms: 0,
            width: 1920,
            height: 1080,
            source_width: 1920,
            source_height: 1080,
            source_aspect_ratio: 16.0 / 9.0,
            content_rect_x: 0,
            content_rect_y: 0,
            content_rect_width: 1920,
            content_rect_height: 1080,
            extractor: "avasset-image-generator".to_string(),
            extractor_version: "0.1.0".to_string(),
        };
        fs::write(
            dir.join("manifest.jsonl"),
            format!("{}\n", serde_json::to_string(&row).unwrap()),
        )
        .unwrap();

        let loaded = load_manifest(&dir).unwrap();
        assert_eq!(loaded, vec![row]);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn supports_common_video_extensions_case_insensitively() {
        assert!(is_supported_video_file(Path::new("clip.MP4")));
        assert!(is_supported_video_file(Path::new("clip.mov")));
        assert!(is_supported_video_file(Path::new("clip.LRV")));
        assert!(!is_supported_video_file(Path::new("clip.jpg")));
    }

    #[test]
    fn batch_bundle_dir_preserves_relative_structure() {
        let input_root = Path::new("/tmp/input");
        let input = Path::new("/tmp/input/DCIM/Camera02/VID_001.MP4");
        let output_root = Path::new("/tmp/output");
        let bundle_dir = bundle_dir_for_input(input_root, input, output_root).unwrap();
        assert_eq!(bundle_dir, Path::new("/tmp/output/DCIM/Camera02/VID_001"));
    }
}
