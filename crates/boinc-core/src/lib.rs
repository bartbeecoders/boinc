//! Core conversion engine for Boinc.
//!
//! Holds the [`Converter`] trait, the [`ConverterRegistry`], format
//! detection, output-path policy, and the conversion pipeline. UI- and
//! OS-agnostic by design: nothing in this crate may depend on Floem or
//! platform integration code.
//!
//! See this crate's README for how to add a new conversion.

mod converter;
mod error;
mod format;
mod output;
mod pipeline;
mod registry;

pub mod converters;

pub use converter::{ConversionOptions, Converter};
pub use error::ConversionError;
pub use format::{Format, detect_format};
pub use output::OutputPolicy;
pub use pipeline::{ConversionRequest, ConversionResult, convert};
pub use registry::ConverterRegistry;

/// Crate version, surfaced by the CLI and app for `--version` output.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::version().is_empty());
    }
}
