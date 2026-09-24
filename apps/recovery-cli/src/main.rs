//! `recovery` command line interface.
//!
//! AGENT.2 exposes three read-only commands:
//!
//! ```text
//! recovery inspect <model.moc3> [--json]
//! recovery export-ir <model.moc3> [--output file] [--compact]
//! recovery validate-ir <model.ir.json> [--json]
//! ```
//!
//! Nothing in this binary writes to `.moc3` inputs, and no CMO3/PSD output
//! exists yet (later phases).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use live2d_ir::{count_severity, has_fatal, Live2DModel, Severity};
use recovery_core::{
    build_ir_from_file, export_ir_json, import_ir_json, inspect_file, InspectError, InspectOptions,
};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "recovery",
    version,
    about = "Live2D recovery toolkit (inspection and IR phases)"
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
    /// Export a .moc3 file as canonical normalized IR JSON.
    ExportIr {
        /// Path to the .moc3 file.
        file: PathBuf,
        /// Write the JSON to this path instead of stdout.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Emit compact single-line JSON (default is pretty JSON).
        #[arg(long)]
        compact: bool,
        /// Override the maximum accepted file size in bytes.
        #[arg(long, value_name = "BYTES")]
        max_file_size: Option<u64>,
    },
    /// Validate a canonical IR JSON file produced by `export-ir`.
    ValidateIr {
        /// Path to the IR JSON file.
        file: PathBuf,
        /// Emit the diagnostics as JSON instead of text.
        #[arg(long)]
        json: bool,
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
        } => run_inspect(&file, json, options_with_limit(max_file_size)),
        Command::ExportIr {
            file,
            output,
            compact,
            max_file_size,
        } => run_export_ir(
            &file,
            output.as_deref(),
            compact,
            options_with_limit(max_file_size),
        ),
        Command::ValidateIr { file, json } => run_validate_ir(&file, json),
    }
}

fn options_with_limit(max_file_size: Option<u64>) -> InspectOptions {
    let mut options = InspectOptions::default();
    if let Some(limit) = max_file_size {
        options.limits.max_file_size = limit;
    }
    options
}

// ---------------------------------------------------------------------------
// inspect
// ---------------------------------------------------------------------------

fn run_inspect(file: &Path, json_output: bool, options: InspectOptions) -> ExitCode {
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
            report_inspect_error(file, &error, json_output);
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// export-ir
// ---------------------------------------------------------------------------

fn run_export_ir(
    file: &Path,
    output: Option<&Path>,
    compact: bool,
    options: InspectOptions,
) -> ExitCode {
    log::info!("exporting IR for {}", file.display());
    let model = match build_ir_from_file(file, &options) {
        Ok(model) => model,
        Err(error) => {
            report_inspect_error(file, &error, output.is_none());
            return ExitCode::FAILURE;
        }
    };
    let machine_mode = output.is_none();
    if has_fatal(&model.diagnostics) {
        report_ir_failure(file, &model, machine_mode);
        return ExitCode::FAILURE;
    }
    let diagnostic_count = model.diagnostics.len();
    let text = match export_ir_json(&model, !compact) {
        Ok(text) => text,
        Err(error) => {
            report_ir_export_error(file, error.code, &error.message, output.is_none());
            return ExitCode::FAILURE;
        }
    };
    match output {
        Some(path) => {
            let mut payload = text;
            payload.push('\n');
            if let Err(error) = std::fs::write(path, payload.as_bytes()) {
                eprintln!("error: could not write '{}': {error}", path.display());
                return ExitCode::FAILURE;
            }
            log::info!(
                "wrote {} ({} diagnostic(s) recorded, none fatal)",
                path.display(),
                diagnostic_count
            );
        }
        None => {
            println!("{text}");
            log::info!(
                "exported IR ({} diagnostic(s) recorded, none fatal)",
                diagnostic_count
            );
        }
    }
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// validate-ir
// ---------------------------------------------------------------------------

fn run_validate_ir(file: &Path, json_output: bool) -> ExitCode {
    log::info!("validating IR {}", file.display());
    let text = match read_text_file(file) {
        Ok(text) => text,
        Err(message) => {
            report_generic_error(file, "IoError", &message, None, json_output, None);
            return ExitCode::FAILURE;
        }
    };
    let model = match import_ir_json(&text) {
        Ok(model) => model,
        Err(error) => {
            report_generic_error(
                file,
                error.code,
                &error.message,
                Some("the file is not a valid live2d-ir/1 document"),
                json_output,
                None,
            );
            return ExitCode::FAILURE;
        }
    };
    let mut diagnostics = model.diagnostics.clone();
    let validation = live2d_ir::validate_ir(&model);
    for diagnostic in validation {
        if !diagnostics.contains(&diagnostic) {
            diagnostics.push(diagnostic);
        }
    }
    let valid = !has_fatal(&diagnostics);
    if json_output {
        let payload = json!({
            "file": file.display().to_string(),
            "valid": valid,
            "diagnostics": diagnostics,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("error: failed to serialize diagnostics: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        print!("{}", render_diagnostics(&diagnostics));
    }
    if valid {
        log::info!("IR is valid ({} diagnostic(s) recorded)", diagnostics.len());
        ExitCode::SUCCESS
    } else {
        log::error!("IR is invalid");
        ExitCode::FAILURE
    }
}

fn render_diagnostics(diagnostics: &[live2d_ir::Diagnostic]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "IR diagnostics: {} total ({} fatal, {} recoverable, {} warning, {} info)",
        diagnostics.len(),
        count_severity(diagnostics, Severity::Fatal),
        count_severity(diagnostics, Severity::Recoverable),
        count_severity(diagnostics, Severity::Warning),
        count_severity(diagnostics, Severity::Info)
    );
    for diagnostic in diagnostics {
        let entity = match &diagnostic.entity {
            Some(entity) => format!("{entity:?}"),
            None => "-".to_string(),
        };
        let _ = writeln!(
            out,
            "  [{:?}] {} (entity: {}) : {}",
            diagnostic.severity, diagnostic.code, entity, diagnostic.message
        );
    }
    out
}

fn read_text_file(file: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(file).map_err(|error| error.to_string())?;
    let limit = InspectOptions::default().limits.max_file_size;
    if metadata.len() > limit {
        return Err(format!(
            "'{}' is {} bytes which exceeds the configured limit of {limit} bytes",
            file.display(),
            metadata.len()
        ));
    }
    let bytes = std::fs::read(file).map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| format!("file is not valid UTF-8: {error}"))
}

// ---------------------------------------------------------------------------
// error reporting
// ---------------------------------------------------------------------------

fn report_inspect_error(file: &Path, error: &InspectError, json_output: bool) {
    report_generic_error(
        file,
        error.code(),
        &error.to_string(),
        error.suggestion(),
        json_output,
        None,
    );
}

fn report_ir_failure(file: &Path, model: &Live2DModel, json_output: bool) {
    let fatal: Vec<&live2d_ir::Diagnostic> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Fatal)
        .collect();
    let summary = fatal
        .first()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .unwrap_or_else(|| "unknown fatal diagnostic".to_string());
    let diagnostics = serde_json::to_value(&model.diagnostics).unwrap_or(json!([]));
    report_generic_error(
        file,
        "IrValidationFailed",
        &format!(
            "IR validation failed with {} fatal diagnostic(s); first: {summary}",
            fatal.len()
        ),
        Some("fix the reported diagnostics in the source data or report a parser bug"),
        json_output,
        Some(diagnostics),
    );
}

fn report_ir_export_error(file: &Path, code: &'static str, message: &str, json_output: bool) {
    report_generic_error(
        file,
        code,
        message,
        Some("the IR could not be serialized; inspect validate-ir output for details"),
        json_output,
        None,
    );
}

fn report_generic_error(
    file: &Path,
    code: &str,
    message: &str,
    suggestion: Option<&str>,
    json_output: bool,
    diagnostics: Option<serde_json::Value>,
) {
    if json_output {
        let mut error = json!({
            "code": code,
            "message": message,
            "suggestion": suggestion,
            "file": file.display().to_string(),
        });
        if let (Some(value), Some(map)) = (diagnostics, error.as_object_mut()) {
            map.insert("diagnostics".to_string(), value);
        }
        let payload = json!({ "error": error });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(_) => eprintln!("error: {message}"),
        }
    } else {
        eprintln!("error: {message}");
        if let Some(suggestion) = suggestion {
            eprintln!("suggestion: {suggestion}");
        }
    }
}
