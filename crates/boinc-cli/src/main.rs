//! `boinc` CLI: scriptable conversions, and the entry point OS context menus
//! invoke.
//!
//! Exit codes: 0 all conversions succeeded, 1 at least one failed, 2 usage
//! error (clap's default).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boinc_core::{
    ConversionError, ConversionRequest, ConversionResult, ConverterRegistry, Format, OutputPolicy,
    convert, detect_format,
};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "boinc",
    version,
    about = "Convert files from one format to another"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert one or more files to a target format
    Convert(ConvertArgs),
    /// List supported conversions, or the targets available for a file
    ListConversions(ListArgs),
}

#[derive(Args)]
struct ConvertArgs {
    /// Input file(s)
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Target format: png, jpg, pdf, docx
    #[arg(long, value_parser = parse_format)]
    to: Format,

    /// Output file path (single input only; must not exist yet)
    #[arg(long, conflicts_with = "out_dir")]
    out: Option<PathBuf>,

    /// Directory for outputs (default: next to each input)
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// JPEG quality, 1-100
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,

    /// Background color for flattening transparency, as RRGGBB hex
    #[arg(long, value_parser = parse_color)]
    background: Option<[u8; 3]>,

    /// Emit machine-readable JSON lines on stdout instead of text
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ListArgs {
    /// List the target formats available for this file
    file: Option<PathBuf>,

    /// Include conversions whose backing tool is not installed
    #[arg(long)]
    all: bool,

    /// Emit machine-readable JSON on stdout instead of text
    #[arg(long)]
    json: bool,
}

fn parse_format(s: &str) -> Result<Format, String> {
    Format::from_extension(s).ok_or_else(|| {
        format!(
            "unknown format {s:?}; expected one of: {}",
            Format::ALL.map(Format::extension).join(", ")
        )
    })
}

fn parse_color(s: &str) -> Result<[u8; 3], String> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected RRGGBB hex, e.g. ffffff".into());
    }
    let byte = |i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string());
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let registry = ConverterRegistry::with_defaults();
    match cli.command {
        Command::Convert(args) => run_convert(&registry, &args),
        Command::ListConversions(args) => run_list(&registry, &args),
    }
}

fn run_convert(registry: &ConverterRegistry, args: &ConvertArgs) -> ExitCode {
    if args.out.is_some() && args.inputs.len() > 1 {
        eprintln!("error: --out requires a single input; use --out-dir for batches");
        return ExitCode::from(2);
    }

    let reporter = if args.json {
        Reporter::Json
    } else {
        Reporter::Text
    };
    let mut failed = false;

    for input in &args.inputs {
        let mut request = ConversionRequest::new(input, args.to);
        request.output = args.out.clone();
        request.policy = OutputPolicy {
            dir: args.out_dir.clone(),
        };
        request.options.jpeg_quality = args.quality;
        if let Some(background) = args.background {
            request.options.background = background;
        }

        let mut last_pct = -1;
        let result = convert(registry, &request, &mut |p| {
            let pct = (p * 100.0).round() as i32;
            if pct != last_pct {
                last_pct = pct;
                reporter.progress(input, pct);
            }
        });

        match result {
            Ok(result) => reporter.success(&result),
            Err(err) => {
                failed = true;
                reporter.failure(input, &err);
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_list(registry: &ConverterRegistry, args: &ListArgs) -> ExitCode {
    match &args.file {
        Some(file) => {
            let from = match detect_format(file) {
                Ok(from) => from,
                Err(err) => {
                    eprintln!("error: {err}");
                    return ExitCode::FAILURE;
                }
            };
            let targets: Vec<Format> = if args.all {
                registry
                    .pairs()
                    .into_iter()
                    .filter(|(f, _)| *f == from)
                    .map(|(_, to)| to)
                    .collect()
            } else {
                registry.available_targets(from)
            };
            if args.json {
                let json = serde_json::json!({
                    "file": file.display().to_string(),
                    "format": from.extension(),
                    "targets": targets.iter().map(|t| t.extension()).collect::<Vec<_>>(),
                });
                println!("{json}");
            } else {
                for to in targets {
                    println!("{from} -> {to}");
                }
            }
        }
        None => {
            let mut rows: Vec<(Format, Format, bool)> = registry
                .pairs()
                .into_iter()
                .map(|(from, to)| {
                    let available = registry.get(from, to).is_some_and(|c| c.is_available());
                    (from, to, available)
                })
                .collect();
            if !args.all {
                rows.retain(|(_, _, available)| *available);
            }
            if args.json {
                let list: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|(from, to, available)| {
                        serde_json::json!({
                            "from": from.extension(),
                            "to": to.extension(),
                            "available": available,
                        })
                    })
                    .collect();
                println!("{}", serde_json::Value::Array(list));
            } else {
                for (from, to, available) in rows {
                    if available {
                        println!("{from} -> {to}");
                    } else {
                        println!("{from} -> {to} (unavailable)");
                    }
                }
            }
        }
    }
    ExitCode::SUCCESS
}

/// Where results and progress go: human-readable text (results on stdout,
/// progress and errors on stderr) or JSON lines on stdout.
enum Reporter {
    Text,
    Json,
}

impl Reporter {
    fn progress(&self, input: &Path, pct: i32) {
        match self {
            Reporter::Text => {
                eprint!("\r{}: {pct}%", input.display());
                if pct == 100 {
                    eprintln!();
                }
                let _ = std::io::stderr().flush();
            }
            Reporter::Json => {
                let json = serde_json::json!({
                    "event": "progress",
                    "input": input.display().to_string(),
                    "percent": pct,
                });
                println!("{json}");
            }
        }
    }

    fn success(&self, result: &ConversionResult) {
        match self {
            Reporter::Text => {
                println!("{} -> {}", result.input.display(), result.output.display());
            }
            Reporter::Json => {
                let json = serde_json::json!({
                    "event": "converted",
                    "input": result.input.display().to_string(),
                    "output": result.output.display().to_string(),
                    "from": result.from.extension(),
                    "to": result.to.extension(),
                });
                println!("{json}");
            }
        }
    }

    fn failure(&self, input: &Path, err: &ConversionError) {
        match self {
            Reporter::Text => eprintln!("\rerror: {}: {err}", input.display()),
            Reporter::Json => {
                let json = serde_json::json!({
                    "event": "error",
                    "input": input.display().to_string(),
                    "message": err.to_string(),
                });
                println!("{json}");
            }
        }
    }
}
