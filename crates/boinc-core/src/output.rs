use std::path::{Path, PathBuf};

use crate::format::Format;

/// Where converted files are written when the caller does not name an output
/// path explicitly.
#[derive(Debug, Clone, Default)]
pub struct OutputPolicy {
    /// Directory for outputs; `None` means alongside the input file.
    pub dir: Option<PathBuf>,
}

impl OutputPolicy {
    /// Default output path for converting `input` to `to`: the input's file
    /// stem with the target extension, in `dir` (or next to the input),
    /// suffixed with ` (1)`, ` (2)`, … rather than overwriting an existing
    /// file.
    pub fn output_path(&self, input: &Path, to: Format) -> PathBuf {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("converted");
        let dir = match &self.dir {
            Some(dir) => dir.clone(),
            None => input.parent().map(Path::to_path_buf).unwrap_or_default(),
        };
        non_clobbering(&dir, stem, to.extension())
    }
}

/// First path of the form `dir/stem.ext`, `dir/stem (1).ext`, … that does not
/// exist yet.
fn non_clobbering(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let candidate = dir.join(format!("{stem}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    for n in 1u32.. {
        let candidate = dir.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("a free suffix always exists")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn renames_instead_of_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.png");
        std::fs::write(&input, b"x").unwrap();

        let policy = OutputPolicy::default();
        assert_eq!(
            policy.output_path(&input, Format::Jpg),
            dir.path().join("photo.jpg")
        );

        std::fs::write(dir.path().join("photo.jpg"), b"x").unwrap();
        assert_eq!(
            policy.output_path(&input, Format::Jpg),
            dir.path().join("photo (1).jpg")
        );

        std::fs::write(dir.path().join("photo (1).jpg"), b"x").unwrap();
        assert_eq!(
            policy.output_path(&input, Format::Jpg),
            dir.path().join("photo (2).jpg")
        );
    }

    #[test]
    fn explicit_directory_overrides_input_location() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let input = src.path().join("doc.docx");
        std::fs::write(&input, b"x").unwrap();

        let policy = OutputPolicy {
            dir: Some(dst.path().to_path_buf()),
        };
        assert_eq!(
            policy.output_path(&input, Format::Pdf),
            dst.path().join("doc.pdf")
        );
    }
}
