use std::path::PathBuf;

use thiserror::Error;

use crate::format::Format;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("could not determine the format of {}", path.display())]
    UnknownFormat { path: PathBuf },

    #[error("no converter available for {from} to {to}")]
    Unsupported { from: Format, to: Format },

    #[error("output already exists: {}", path.display())]
    OutputExists { path: PathBuf },

    #[error("{tool} is required for this conversion but was not found; {hint}")]
    ToolNotFound { tool: String, hint: String },

    #[error("{tool} failed: {message}")]
    ToolFailed { tool: String, message: String },

    #[error(transparent)]
    Image(#[from] image::ImageError),
}
