use std::path::PathBuf;

use crate::converter::ConversionOptions;
use crate::error::ConversionError;
use crate::format::{Format, detect_format};
use crate::output::OutputPolicy;
use crate::registry::ConverterRegistry;

/// A conversion job: what to convert and where the result should go.
#[derive(Debug, Clone)]
pub struct ConversionRequest {
    pub input: PathBuf,
    pub to: Format,
    /// Explicit output path; fails if it already exists. `None` derives the
    /// path from `policy` (which renames rather than overwrites).
    pub output: Option<PathBuf>,
    pub policy: OutputPolicy,
    pub options: ConversionOptions,
}

impl ConversionRequest {
    pub fn new(input: impl Into<PathBuf>, to: Format) -> Self {
        Self {
            input: input.into(),
            to,
            output: None,
            policy: OutputPolicy::default(),
            options: ConversionOptions::default(),
        }
    }
}

/// A completed conversion.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub from: Format,
    pub to: Format,
    pub input: PathBuf,
    pub output: PathBuf,
}

/// Detect the input format, pick a converter from `registry`, resolve the
/// output path, and run the conversion.
pub fn convert(
    registry: &ConverterRegistry,
    request: &ConversionRequest,
    progress: &mut dyn FnMut(f32),
) -> Result<ConversionResult, ConversionError> {
    let from = detect_format(&request.input)?;
    let converter = registry
        .get(from, request.to)
        .filter(|c| c.is_available())
        .ok_or(ConversionError::Unsupported {
            from,
            to: request.to,
        })?;

    let output = match &request.output {
        Some(path) => {
            if path.exists() {
                return Err(ConversionError::OutputExists { path: path.clone() });
            }
            path.clone()
        }
        None => request.policy.output_path(&request.input, request.to),
    };

    converter.convert(&request.input, &output, &request.options, progress)?;

    Ok(ConversionResult {
        from,
        to: request.to,
        input: request.input.clone(),
        output,
    })
}
