use std::path::{Path, PathBuf};

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
}

pub fn extract(_request: &ExtractRequest) -> Result<ExtractResult, String> {
    Err(
        "extract is not implemented yet; see docs/visual-bundle-v1.md for the current contract"
            .to_string(),
    )
}
