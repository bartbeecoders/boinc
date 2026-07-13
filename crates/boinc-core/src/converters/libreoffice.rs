//! PDF ↔ DOCX conversion delegated to headless LibreOffice (`soffice`).
//!
//! v1 decision (plan.md, risk #1): document conversion quality lives or dies
//! on a layout engine; rather than reimplement one, Boinc shells out to
//! LibreOffice when it is installed and reports these conversions unavailable
//! otherwise. PDF → DOCX goes through LibreOffice's PDF import, which
//! recovers text in draw frames rather than flowing paragraphs — usable, but
//! lossy by nature.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::converter::{ConversionOptions, Converter};
use crate::error::ConversionError;
use crate::format::Format;

pub struct LibreOfficeConverter {
    from: Format,
    to: Format,
}

impl LibreOfficeConverter {
    pub fn docx_to_pdf() -> Self {
        Self {
            from: Format::Docx,
            to: Format::Pdf,
        }
    }

    pub fn pdf_to_docx() -> Self {
        Self {
            from: Format::Pdf,
            to: Format::Docx,
        }
    }
}

/// Locate the `soffice` binary: `BOINC_SOFFICE` env override first, then
/// `PATH`, then standard install locations per OS.
pub fn find_soffice() -> Option<PathBuf> {
    if let Some(path) = env::var_os("BOINC_SOFFICE") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }

    let names: &[&str] = if cfg!(windows) {
        &["soffice.exe", "soffice.com"]
    } else {
        &["soffice"]
    };
    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let known: &[&str] = if cfg!(target_os = "macos") {
        &["/Applications/LibreOffice.app/Contents/MacOS/soffice"]
    } else if cfg!(windows) {
        &[
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ]
    } else {
        &[
            "/usr/bin/soffice",
            "/usr/local/bin/soffice",
            "/usr/lib/libreoffice/program/soffice",
        ]
    };
    known.iter().map(PathBuf::from).find(|p| p.is_file())
}

impl Converter for LibreOfficeConverter {
    fn supports(&self) -> (Format, Format) {
        (self.from, self.to)
    }

    fn is_available(&self) -> bool {
        find_soffice().is_some()
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError> {
        let soffice = find_soffice().ok_or_else(|| ConversionError::ToolNotFound {
            tool: "LibreOffice".into(),
            hint: "install LibreOffice or point BOINC_SOFFICE at the soffice binary".into(),
        })?;

        progress(0.0);

        let input = absolute(input)?;

        // Fresh work dir per run: `--convert-to` always names its result
        // after the input, so convert into a private dir and move the file to
        // the requested path afterwards. The throwaway UserInstallation
        // profile keeps headless runs from fighting an already-open
        // LibreOffice instance.
        let work = tempfile::tempdir()?;
        let profile_dir = work.path().join("profile");
        std::fs::create_dir(&profile_dir)?;

        let mut cmd = Command::new(&soffice);
        cmd.arg(format!("-env:UserInstallation={}", file_url(&profile_dir)))
            .args(["--headless", "--norestore"]);
        if self.from == Format::Pdf {
            cmd.arg("--infilter=writer_pdf_import");
        }
        cmd.args(["--convert-to", target_filter(self.to), "--outdir"])
            .arg(work.path())
            .arg(&input);

        let run = cmd.output().map_err(|e| ConversionError::ToolFailed {
            tool: "LibreOffice".into(),
            message: format!("could not run {}: {e}", soffice.display()),
        })?;
        progress(0.9);

        let stem = input.file_stem().unwrap_or_default();
        let produced = work.path().join(stem).with_extension(self.to.extension());
        if !run.status.success() || !produced.is_file() {
            return Err(ConversionError::ToolFailed {
                tool: "LibreOffice".into(),
                message: format!(
                    "{}; {}",
                    run.status,
                    String::from_utf8_lossy(&run.stderr).trim()
                ),
            });
        }

        move_file(&produced, output)?;
        progress(1.0);
        Ok(())
    }
}

/// LibreOffice's `--convert-to` target: an extension, optionally with an
/// explicit export filter.
fn target_filter(to: Format) -> &'static str {
    match to {
        Format::Pdf => "pdf",
        Format::Docx => "docx:MS Word 2007 XML",
        other => other.extension(),
    }
}

fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

/// File URL for `-env:UserInstallation=`; forward slashes work on every OS.
fn file_url(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    if p.starts_with('/') {
        format!("file://{p}")
    } else {
        format!("file:///{p}")
    }
}

/// `rename`, falling back to copy + delete when source and destination are on
/// different filesystems (the temp dir often is).
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)
}
