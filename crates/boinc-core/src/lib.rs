//! Core conversion engine for Boinc.
//!
//! Will hold the `Converter` trait, the converter registry, and the
//! conversion pipeline (Phase 1 of `plan.md`). UI- and OS-agnostic by design:
//! nothing in this crate may depend on Floem or platform integration code.

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
