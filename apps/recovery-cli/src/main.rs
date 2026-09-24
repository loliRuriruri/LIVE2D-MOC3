//! `recovery` command line interface.
//!
//! ```text
//! recovery inspect <model.moc3> [--json]
//! recovery export-ir <model.moc3> [--output file] [--compact]
//! recovery validate-ir <model.ir.json> [--json]
//! recovery reconstruct-hierarchy <model.moc3|model.ir.json>
//!          [--json] [--output file] [--strict] [--allow-heuristic]
//!          [--explain node-id] [--max-depth N]
//! recovery recover-keyforms <model.moc3|model.ir.json>
//!          [--json] [--output file] [--strict] [--explain target]
//! ```
//!
//! Nothing in this binary writes to `.moc3` inputs, and no CMO3/PSD output
//! exists yet (later phases).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hierarchy_recovery::{explain as explain_hierarchy, reconstruct, RecoveryPolicy};
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
    /// Reconstruct the hierarchy from a .moc3 file or a canonical IR JSON.
    ReconstructHierarchy {
        /// Path to the input file (.moc3 or .ir.json).
        input: PathBuf,
        /// Emit the recovered project JSON instead of the human report.
        #[arg(long)]
        json: bool,
        /// Write the recovered project JSON to this path.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Exit non-zero when ambiguity, cycles, dangling candidates or
        /// illegal relations are present.
        #[arg(long)]
        strict: bool,
        /// Enable policy-gated heuristic rules (HR-010).
        #[arg(long)]
        allow_heuristic: bool,
        /// Explain one node (canonical id or stored source name).
        #[arg(long, value_name = "NODE")]
        explain: Option<String>,
        /// Maximum tree depth for the human report.
        #[arg(long, value_name = "N", default_value_t = 64)]
        max_depth: usize,
        /// Override the maximum accepted input size in bytes.
        #[arg(long, value_name = "BYTES")]
        max_file_size: Option<u64>,
    },
    /// Recover the semantic keyform model (parameter bindings, grids, forms).
    RecoverKeyforms {
        /// Path to the input file (.moc3 or .ir.json).
        input: PathBuf,
        /// Emit the recovered keyform JSON instead of the human report.
        #[arg(long)]
        json: bool,
        /// Write the recovered keyform JSON to this path.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Exit non-zero on unresolved cardinality, dangling parameters,
        /// non-finite keys, invalid targets or form-span mismatches.
        #[arg(long)]
        strict: bool,
        /// Explain one target, band or grid id.
        #[arg(long, value_name = "TARGET")]
        explain: Option<String>,
        /// Override the maximum accepted input size in bytes.
        #[arg(long, value_name = "BYTES")]
        max_file_size: Option<u64>,
    },
}

/// Diagnostic codes that make `--strict` fail.
const STRICT_FAILURE_CODES: &[&str] = &[
    "ambiguous_parent",
    "hierarchy_cycle",
    "dangling_candidate",
    "illegal_relation",
];

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
        Command::ReconstructHierarchy {
            input,
            json,
            output,
            strict,
            allow_heuristic,
            explain,
            max_depth,
            max_file_size,
        } => run_reconstruct_hierarchy(
            &input,
            json,
            output.as_deref(),
            strict,
            allow_heuristic,
            explain.as_deref(),
            max_depth,
            options_with_limit(max_file_size),
        ),
        Command::RecoverKeyforms {
            input,
            json,
            output,
            strict,
            explain,
            max_file_size,
        } => run_recover_keyforms(
            &input,
            json,
            output.as_deref(),
            strict,
            explain.as_deref(),
            options_with_limit(max_file_size),
        ),
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
    let limit = InspectOptions::default().limits.max_file_size;
    let text = match read_text_file(file, limit) {
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

// ---------------------------------------------------------------------------
// reconstruct-hierarchy
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_reconstruct_hierarchy(
    input: &Path,
    json_output: bool,
    output: Option<&Path>,
    strict: bool,
    allow_heuristic: bool,
    explain: Option<&str>,
    max_depth: usize,
    options: InspectOptions,
) -> ExitCode {
    log::info!("reconstructing hierarchy for {}", input.display());
    let is_ir_json = input
        .extension()
        .map(|extension| extension.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    let model = if is_ir_json {
        let text = match read_text_file(input, options.limits.max_file_size) {
            Ok(text) => text,
            Err(message) => {
                report_generic_error(input, "IoError", &message, None, true, None);
                return ExitCode::FAILURE;
            }
        };
        match import_ir_json(&text) {
            Ok(model) => model,
            Err(error) => {
                report_generic_error(
                    input,
                    error.code,
                    &error.message,
                    Some("the file is not a valid live2d-ir/1 document"),
                    true,
                    None,
                );
                return ExitCode::FAILURE;
            }
        }
    } else {
        match build_ir_from_file(input, &options) {
            Ok(model) => model,
            Err(error) => {
                report_inspect_error(input, &error, true);
                return ExitCode::FAILURE;
            }
        }
    };
    if has_fatal(&model.diagnostics) {
        report_ir_failure(input, &model, true);
        return ExitCode::FAILURE;
    }

    let policy = if allow_heuristic {
        RecoveryPolicy::with_heuristics()
    } else {
        RecoveryPolicy::default()
    };
    let project = reconstruct(&model, &policy);

    // Fatal hierarchy findings mean the recovered project must not be
    // exported or consumed (work order: fatal gate). This runs before any
    // output is written, in every mode.
    if has_fatal(&project.diagnostics) {
        report_hierarchy_failure(input, &project, true);
        return ExitCode::FAILURE;
    }

    // Explain is diagnostic-only and intentionally not strict-gated.
    if let Some(query) = explain {
        return run_explain(input, &project, query, json_output || output.is_some());
    }

    // Strict gating happens before writing so a rejected project never lands
    // on disk.
    if strict {
        let failures: Vec<&str> = STRICT_FAILURE_CODES
            .iter()
            .copied()
            .filter(|code| {
                project
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == *code)
            })
            .collect();
        if !failures.is_empty() {
            let message = format!(
                "strict mode: {} condition(s) present ({}); no output written",
                failures.len(),
                failures.join(", ")
            );
            report_generic_error(
                input,
                "StrictModeViolation",
                &message,
                Some("resolve the reported conditions or rerun without --strict"),
                output.is_none(),
                None,
            );
            return ExitCode::FAILURE;
        }
    }

    match output {
        Some(path) => {
            let text = match serde_json::to_string_pretty(&project) {
                Ok(text) => text,
                Err(error) => {
                    report_generic_error(
                        input,
                        "SerializationFailed",
                        &error.to_string(),
                        None,
                        false,
                        None,
                    );
                    return ExitCode::FAILURE;
                }
            };
            let mut payload = text;
            payload.push('\n');
            if let Err(error) = std::fs::write(path, payload.as_bytes()) {
                eprintln!("error: could not write '{}': {error}", path.display());
                return ExitCode::FAILURE;
            }
            print!(
                "{}",
                hierarchy_recovery::text::render_human(&project, max_depth)
            );
            log::info!("wrote recovered project to {}", path.display());
        }
        None if json_output => match serde_json::to_string_pretty(&project) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                report_generic_error(
                    input,
                    "SerializationFailed",
                    &error.to_string(),
                    None,
                    true,
                    None,
                );
                return ExitCode::FAILURE;
            }
        },
        None => {
            print!(
                "{}",
                hierarchy_recovery::text::render_human(&project, max_depth)
            );
            log::info!(
                "hierarchy: {} node(s), {} resolved, {} unresolved, {} ambiguous, {} cycle(s)",
                project.statistics.nodes,
                project.statistics.resolved,
                project.statistics.unresolved,
                project.statistics.ambiguous,
                project.statistics.cycles
            );
        }
    }

    ExitCode::SUCCESS
}

/// Structured failure for a recovered project that fails hierarchy
/// validation (fatal findings). The diagnostics list is capped so hostile
/// inputs cannot amplify the error output.
fn report_hierarchy_failure(
    input: &Path,
    project: &hierarchy_recovery::RecoveredProject,
    json_output: bool,
) {
    const MAX_REPORTED: usize = 50;
    let fatal: Vec<&live2d_ir::Diagnostic> = project
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Fatal)
        .collect();
    let summary = fatal
        .first()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .unwrap_or_else(|| "unknown fatal diagnostic".to_string());
    let capped: Vec<&live2d_ir::Diagnostic> = fatal.iter().copied().take(MAX_REPORTED).collect();
    let diagnostics = serde_json::to_value(&capped).unwrap_or(json!([]));
    report_generic_error(
        input,
        "HierarchyValidationFailed",
        &format!(
            "hierarchy validation failed with {} fatal diagnostic(s); first: {summary}",
            fatal.len()
        ),
        Some("the recovered project is invalid and was not written; please report a reconstruction bug"),
        json_output,
        Some(diagnostics),
    );
}

fn run_explain(
    input: &Path,
    project: &hierarchy_recovery::RecoveredProject,
    query: &str,
    json_output: bool,
) -> ExitCode {
    let node = project
        .nodes
        .iter()
        .find(|node| node.id.as_str() == query)
        .or_else(|| {
            project
                .nodes
                .iter()
                .find(|node| node.source_name.as_deref() == Some(query))
        });
    let Some(node) = node else {
        report_generic_error(
            input,
            "UnknownNode",
            &format!("node '{query}' was not found in the recovered project"),
            Some(
                "use the canonical node id (for example artmesh:000042) or the stored source name",
            ),
            json_output,
            None,
        );
        return ExitCode::FAILURE;
    };
    if json_output {
        let evidence = node
            .trace
            .selected
            .as_ref()
            .and_then(|selected| project.graph.edges.get(selected.evidence_index))
            .map(|edge| {
                json!({
                    "kind": format!("{:?}", edge.kind),
                    "rule": edge.rule,
                    "confidence": edge.confidence,
                    "note": edge.note,
                })
            });
        let payload = json!({
            "node": node.id,
            "kind": node.kind,
            "status": node.status,
            "selected": node.trace.selected,
            "tied_candidates": node.trace.tied_candidates,
            "rejected": node.trace.rejected,
            "evidence": evidence,
            "note": node.trace.note,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("error: failed to serialize explain output: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else if let Some(text) = explain_hierarchy(project, query) {
        print!("{text}");
    }
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// recover-keyforms
// ---------------------------------------------------------------------------

fn run_recover_keyforms(
    input: &Path,
    json_output: bool,
    output: Option<&Path>,
    strict: bool,
    explain: Option<&str>,
    options: InspectOptions,
) -> ExitCode {
    log::info!("recovering keyforms for {}", input.display());
    let is_ir_json = input
        .extension()
        .map(|extension| extension.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    let model = if is_ir_json {
        let text = match read_text_file(input, options.limits.max_file_size) {
            Ok(text) => text,
            Err(message) => {
                report_generic_error(input, "IoError", &message, None, true, None);
                return ExitCode::FAILURE;
            }
        };
        match import_ir_json(&text) {
            Ok(model) => model,
            Err(error) => {
                report_generic_error(
                    input,
                    error.code,
                    &error.message,
                    Some("the file is not a valid live2d-ir/1 document"),
                    true,
                    None,
                );
                return ExitCode::FAILURE;
            }
        }
    } else {
        match build_ir_from_file(input, &options) {
            Ok(model) => model,
            Err(error) => {
                report_inspect_error(input, &error, true);
                return ExitCode::FAILURE;
            }
        }
    };
    if has_fatal(&model.diagnostics) {
        report_ir_failure(input, &model, true);
        return ExitCode::FAILURE;
    }

    let project = reconstruct(&model, &RecoveryPolicy::default());
    if has_fatal(&project.diagnostics) {
        report_generic_error(
            input,
            "HierarchyValidationFailed",
            "the recovered project is invalid; keyforms cannot be cross-checked",
            Some("run reconstruct-hierarchy to inspect the fatal findings first"),
            true,
            None,
        );
        return ExitCode::FAILURE;
    }

    let document = recovery_core::recover_keyforms(&model, Some(&project));
    if has_fatal(&document.diagnostics) {
        let fatal: Vec<&live2d_ir::Diagnostic> = document
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Fatal)
            .collect();
        let summary = fatal
            .first()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .unwrap_or_else(|| "unknown fatal diagnostic".to_string());
        report_generic_error(
            input,
            "KeyformValidationFailed",
            &format!(
                "keyform validation failed with {} fatal diagnostic(s); first: {summary}",
                fatal.len()
            ),
            Some("the recovered keyform document is invalid and was not written; please report a recovery bug"),
            true,
            None,
        );
        return ExitCode::FAILURE;
    }

    if let Some(query) = explain {
        return run_keyform_explain(input, &document, query, json_output || output.is_some());
    }

    if strict {
        let violations = keyform_recovery::strict_violations(&document);
        if !violations.is_empty() {
            report_generic_error(
                input,
                "StrictModeViolation",
                &format!(
                    "strict mode: {} condition(s) present ({}); no output written",
                    violations.len(),
                    violations.join(", ")
                ),
                Some("resolve the reported conditions or rerun without --strict"),
                output.is_none(),
                None,
            );
            return ExitCode::FAILURE;
        }
    }

    match output {
        Some(path) => {
            let text = match recovery_core::export_keyforms_json(&document, true) {
                Ok(text) => text,
                Err(error) => {
                    report_generic_error(input, error.code, &error.message, None, false, None);
                    return ExitCode::FAILURE;
                }
            };
            let mut payload = text;
            payload.push('\n');
            if let Err(error) = std::fs::write(path, payload.as_bytes()) {
                eprintln!("error: could not write '{}': {error}", path.display());
                return ExitCode::FAILURE;
            }
            print!("{}", keyform_recovery::render_human(&document));
            log::info!("wrote recovered keyforms to {}", path.display());
        }
        None if json_output => match recovery_core::export_keyforms_json(&document, true) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                report_generic_error(input, error.code, &error.message, None, true, None);
                return ExitCode::FAILURE;
            }
        },
        None => {
            print!("{}", keyform_recovery::render_human(&document));
            log::info!(
                "keyforms: {} target(s), {} binding(s), {} stored form(s), {} unresolved",
                document.statistics.targets,
                document.statistics.bindings,
                document.statistics.stored_keyforms,
                document.statistics.unresolved_entries
            );
        }
    }

    ExitCode::SUCCESS
}

fn run_keyform_explain(
    input: &Path,
    document: &keyform_recovery::RecoveredKeyformModel,
    query: &str,
    json_output: bool,
) -> ExitCode {
    let trace = document.traces.iter().find(|trace| {
        trace.target.id_text() == query || trace.band == query || trace.grid == query
    });
    let Some(trace) = trace else {
        report_generic_error(
            input,
            "UnknownKeyformTarget",
            &format!("'{query}' was not found in the recovered keyform model"),
            Some("use a target id, a band id (band:...) or a grid id (grid:...)"),
            json_output,
            None,
        );
        return ExitCode::FAILURE;
    };
    if json_output {
        match serde_json::to_string_pretty(trace) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("error: failed to serialize explain output: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else if let Some(text) = keyform_recovery::explain(document, query) {
        print!("{text}");
    }
    ExitCode::SUCCESS
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

fn read_text_file(file: &Path, limit: u64) -> Result<String, String> {
    let metadata = std::fs::metadata(file).map_err(|error| error.to_string())?;
    if metadata.len() > limit {
        return Err(format!(
            "'{}' is {} bytes which exceeds the configured limit of {limit} bytes",
            file.display(),
            metadata.len()
        ));
    }
    let mut bytes = std::fs::read(file).map_err(|error| error.to_string())?;
    // Accept UTF-8 BOMs from Windows editors/tools.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
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
