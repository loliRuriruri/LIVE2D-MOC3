//! `recovery-reference`: dev-only differential validation CLI (AGENT.3.5).
//!
//! ```text
//! recovery-reference compare model.moc3 \
//!     --provider ours --provider py-moc3 [--json] [--output FILE]
//! recovery-reference mock-provider <kind> <model.moc3>   (hidden, test-only)
//! ```

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use reference_harness::providers::{mock_mutation_by_name, ProviderConfig};
use reference_harness::{report, run_compare};

#[derive(Parser)]
#[command(
    name = "recovery-reference",
    version,
    about = "Dev-only differential validation harness (not a production tool)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare independent MOC3 implementations on one input.
    Compare {
        /// Input file (.moc3).
        input: PathBuf,
        /// Provider to run (repeatable; `ours` is always included).
        #[arg(long = "provider", value_name = "NAME")]
        providers: Vec<String>,
        /// Emit the canonical JSON report instead of the human report.
        #[arg(long)]
        json: bool,
        /// Write the canonical JSON report to this path.
        #[arg(long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Hard timeout per external process (seconds).
        #[arg(long, value_name = "SECS", default_value_t = 30)]
        timeout: u64,
        /// Maximum accepted external stdout (bytes).
        #[arg(long, value_name = "BYTES", default_value_t = 8 * 1024 * 1024)]
        output_limit: usize,
    },
    /// Emit a mutated snapshot for harness tests (hidden).
    #[command(hide = true)]
    MockProvider {
        /// Mutation kind.
        kind: String,
        /// Input file.
        input: PathBuf,
        /// Provider name to advertise.
        #[arg(long, default_value = "mock")]
        name: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Compare {
            input,
            providers,
            json,
            output,
            timeout,
            output_limit,
        } => run_compare_command(
            &input,
            &providers,
            json,
            output.as_deref(),
            timeout,
            output_limit,
        ),
        Command::MockProvider { kind, input, name } => run_mock(&kind, &input, &name),
    }
}

fn run_compare_command(
    input: &std::path::Path,
    providers: &[String],
    json_output: bool,
    output: Option<&std::path::Path>,
    timeout: u64,
    output_limit: usize,
) -> ExitCode {
    let config = ProviderConfig {
        timeout: Duration::from_secs(timeout.max(1)),
        output_limit: output_limit.max(1024),
    };
    let our_commit = detect_our_commit();
    let report = match run_compare(input, providers, &config, &our_commit) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let payload = if json_output || output.is_some() {
        match report::to_json(&report) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("error: failed to serialize report: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        String::new()
    };

    match output {
        Some(path) => {
            let mut bytes = payload.clone();
            bytes.push('\n');
            if let Err(error) = std::fs::write(path, bytes.as_bytes()) {
                eprintln!("error: could not write '{}': {error}", path.display());
                return ExitCode::FAILURE;
            }
            print!("{}", report::render_human(&report));
        }
        None if json_output => println!("{payload}"),
        None => print!("{}", report::render_human(&report)),
    }
    ExitCode::SUCCESS
}

fn run_mock(kind: &str, input: &std::path::Path, name: &str) -> ExitCode {
    // Process-behavior mocks come first; they do not produce snapshots.
    match kind {
        "crash" => {
            eprintln!("mock provider crash");
            return ExitCode::from(3);
        }
        "hang" => {
            // Tests pass --timeout 1; sleeping is enough to trip it.
            std::thread::sleep(Duration::from_secs(30));
            return ExitCode::SUCCESS;
        }
        "oversize" => {
            let chunk = "x".repeat(1024 * 1024);
            for _ in 0..64 {
                print!("{chunk}");
            }
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let Some(mutation) = mock_mutation_by_name(kind) else {
        eprintln!("error: unknown mock kind '{kind}'");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(input) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let options = recovery_core::InspectOptions::default();
    let model = match recovery_core::build_ir_from_file(input, &options) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let project =
        hierarchy_recovery::reconstruct(&model, &hierarchy_recovery::RecoveryPolicy::default());
    let ours =
        reference_harness::providers::snapshot_from_model(&model, Some(&project), input, &bytes);
    let mut snapshot = reference_harness::providers::mock_snapshot(&ours, name, mutation);
    snapshot.provider = name.to_string();
    match serde_json::to_string_pretty(&snapshot) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn detect_our_commit() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "worktree".to_string(),
    }
}
