//! `recovery` command line interface.
//!
//! AGENT.1 exposes a single read-only command:
//!
//! ```text
//! recovery inspect model.moc3 [--json]
//! ```

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use recovery_core::{inspect_file, InspectError, InspectOptions};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "recovery",
    version,
    about = "Live2D project recovery toolkit (read-only inspection phase)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read a .moc3 file and print a structural inspection report.
    Inspect {
        /// Path to the .moc3 file.
        file: PathBuf,
        /// Emit the machine-readable JSON report instead of the text report.
        #[arg(long)]
        json: bool,
        /// Override the maximum accepted file size in bytes.
        #[arg(long, value_name = "BYTES")]
        max_file_size: Option<u64>,
    },
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Inspect {
            file,
            json,
            max_file_size,
        } => run_inspect(&file, json, max_file_size),
    }
}

fn run_inspect(file: &Path, json_output: bool, max_file_size: Option<u64>) -> ExitCode {
    let mut options = InspectOptions::default();
    if let Some(limit) = max_file_size {
        options.limits.max_file_size = limit;
    }

    log::info!("inspecting {}", file.display());
    match inspect_file(file, &options) {
        Ok(report) => {
            if json_output {
                match serde_json::to_string_pretty(&report) {
                    Ok(text) => println!("{text}"),
                    Err(error) => {
                        eprintln!("error: failed to serialize report: {error}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                print!("{}", moc3_ingest::text::render_human(&report));
            }
            log::info!(
                "ok: {} parameter(s), {} part(s), {} art mesh(es), {} deformer(s)",
                report.model.parameters.len(),
                report.model.parts.len(),
                report.model.art_meshes.len(),
                report.model.deformers.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            report_error(file, &error, json_output);
            ExitCode::FAILURE
        }
    }
}

fn report_error(file: &Path, error: &InspectError, json_output: bool) {
    if json_output {
        let payload = json!({
            "error": {
                "code": error.code(),
                "message": error.to_string(),
                "offset": error.offset(),
                "suggestion": error.suggestion(),
                "file": file.display().to_string(),
            }
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(_) => eprintln!("error: {error}"),
        }
    } else {
        eprintln!("error: {error}");
        if let Some(suggestion) = error.suggestion() {
            eprintln!("suggestion: {suggestion}");
        }
    }
}
