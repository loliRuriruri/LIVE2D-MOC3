//! Build metadata for `--version --verbose` style output (AGENT.5.3).
//!
//! Emits the git commit (short), build profile and target triple so the
//! release binary can report exactly what it was built from. Missing git is
//! not an error (source tarball builds report `unknown`).

use std::process::Command;

fn main() {
    let git = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_GIT={git}");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_PROFILE={profile}");
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_TARGET={target}");
    // Rerun when the git HEAD moves (commit provenance), in addition to the
    // default "any package file changed" behaviour.
    let head = std::path::Path::new("../../.git/HEAD");
    if head.exists() {
        println!("cargo:rerun-if-changed=../../.git/HEAD");
    }
}
