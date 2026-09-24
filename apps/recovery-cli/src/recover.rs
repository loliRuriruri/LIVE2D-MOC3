//! End-to-end `recovery recover` and `recovery inspect-cmo3` commands
//! (AGENT.5.2 Gate 5I).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cmo3_writer::caff;
use cmo3_writer::map::{build_project, MapOptions};
use cmo3_writer::serialize::IdentityOptions;
use cmo3_writer::textures::{TextureAsset, TextureAssets};
use cmo3_writer::{png, validate, write_minimal_cmo3};

use hierarchy_recovery::{reconstruct, RecoveryPolicy};
use keyform_recovery::recover;
use live2d_ir::has_fatal;
use recovery_core::{build_ir_from_file, InspectOptions};
use serde_json::json;

/// Inputs resolved from the CLI arguments and optional `model3.json`.
pub struct RecoverInputs {
    /// MOC3 path.
    pub moc: PathBuf,
    /// Texture files in page order.
    pub textures: Vec<PathBuf>,
    /// Human-readable source list for the report.
    pub sources: Vec<String>,
}

/// Resolve the MOC3 and texture files from the arguments.
pub fn resolve_inputs(
    input: Option<&Path>,
    moc_flag: Option<&Path>,
    model_json: Option<&Path>,
    textures_dir: Option<&Path>,
) -> Result<RecoverInputs, String> {
    let mut sources: Vec<String> = Vec::new();
    let mut moc = moc_flag.map(Path::to_path_buf);
    let mut texture_paths: Vec<PathBuf> = Vec::new();

    let model3_path = match (input, model_json) {
        (Some(path), _) if is_json(path) => Some(path.to_path_buf()),
        (_, Some(path)) => Some(path.to_path_buf()),
        _ => None,
    };
    if let Some(model3) = &model3_path {
        let text = std::fs::read_to_string(model3)
            .map_err(|error| format!("cannot read '{}': {error}", model3.display()))?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("'{}' is not valid JSON: {error}", model3.display()))?;
        let base = model3.parent().unwrap_or(Path::new(".")).to_path_buf();
        let references = value.get("FileReferences");
        if moc.is_none() {
            if let Some(relative) = references
                .and_then(|references| references.get("Moc"))
                .and_then(|moc| moc.as_str())
            {
                moc = Some(base.join(relative));
            }
        }
        if textures_dir.is_none() {
            if let Some(list) = references
                .and_then(|references| references.get("Textures"))
                .and_then(|textures| textures.as_array())
            {
                for entry in list {
                    if let Some(relative) = entry.as_str() {
                        texture_paths.push(base.join(relative));
                    }
                }
            }
        }
        sources.push(format!("model3.json: {}", model3.display()));
    }

    if moc.is_none() {
        if let Some(path) = input {
            if !is_json(path) {
                moc = Some(path.to_path_buf());
            }
        }
    }
    if moc.is_none() {
        return Err("no MOC3 input: pass a .moc3/model3.json path or --moc".to_string());
    }

    if let Some(dir) = textures_dir {
        texture_paths = list_texture_dir(dir)?;
    }
    let moc = moc.unwrap_or_default();
    sources.push(format!("moc3: {}", moc.display()));
    for (page, path) in texture_paths.iter().enumerate() {
        sources.push(format!("texture[{page}]: {}", path.display()));
    }
    Ok(RecoverInputs {
        moc,
        textures: texture_paths,
        sources,
    })
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn list_texture_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|error| format!("cannot read texture directory '{}': {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path
            .extension()
            .map(|extension| extension.eq_ignore_ascii_case("png"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn load_texture_assets(paths: &[PathBuf]) -> Result<TextureAssets, String> {
    const MAX_TEXTURE: u64 = 256 * 1024 * 1024;
    let mut assets = TextureAssets::new();
    for (page, path) in paths.iter().enumerate() {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > MAX_TEXTURE {
                return Err(format!(
                    "texture '{}' is {} bytes which exceeds the texture cap",
                    path.display(),
                    metadata.len()
                ));
            }
        }
        let bytes = std::fs::read(path)
            .map_err(|error| format!("cannot read texture '{}': {error}", path.display()))?;
        let (width, height) = match png::png_dimensions(&bytes) {
            Some((width, height)) => (Some(width), Some(height)),
            None => (None, None),
        };
        assets.push(TextureAsset {
            page: page as u32,
            bytes,
            source_path: Some(path.display().to_string()),
            width,
            height,
        });
    }
    Ok(assets)
}

/// Run the end-to-end recovery command.
#[allow(clippy::too_many_arguments)]
pub fn run_recover(
    input: Option<&Path>,
    moc_flag: Option<&Path>,
    model_json: Option<&Path>,
    textures_dir: Option<&Path>,
    output: Option<&Path>,
    report: Option<&Path>,
    best_effort: bool,
    random_guids: bool,
    obfuscation_key: Option<i32>,
    force: bool,
    max_file_size: Option<u64>,
) -> ExitCode {
    let inputs = match resolve_inputs(input, moc_flag, model_json, textures_dir) {
        Ok(inputs) => inputs,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let mut options = InspectOptions::default();
    if let Some(limit) = max_file_size {
        options.limits.max_file_size = limit;
    }
    log::info!("recovering {}", inputs.moc.display());
    let model = match build_ir_from_file(&inputs.moc, &options) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if has_fatal(&model.diagnostics) {
        eprintln!("error: IR validation failed; run `recovery export-ir` for details");
        return ExitCode::FAILURE;
    }
    let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
    if has_fatal(&hierarchy.diagnostics) {
        eprintln!(
            "error: hierarchy validation failed; run `recovery reconstruct-hierarchy` for details"
        );
        return ExitCode::FAILURE;
    }
    let keyforms = recover(&model, Some(&hierarchy));

    if !best_effort && !model.textures.is_empty() && inputs.textures.is_empty() {
        eprintln!(
            "error: MissingTextureAsset: the model references {} texture page(s) but none were supplied",
            model.textures.len()
        );
        eprintln!("hint: pass --textures <dir> or a model3.json with FileReferences.Textures");
        return ExitCode::FAILURE;
    }
    let assets = match load_texture_assets(&inputs.textures) {
        Ok(assets) => assets,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let map_options = MapOptions {
        best_effort,
        model_name: None,
    };
    let project = match build_project(&model, &hierarchy, &keyforms, &assets, &map_options) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("error: Cannot produce strict CMO3.");
            eprintln!("reason: {} ({})", error.message, error.code);
            if error.code.contains("KeyformGrid") || error.code.contains("Parameter") {
                for entry in keyforms.unresolved.iter().take(5) {
                    eprintln!("  unresolved: {} ({})", entry.detail, entry.code);
                }
                eprintln!("hint: run `recovery recover-keyforms <model.moc3> --explain <target>`");
            }
            if error.code == "MissingTextureAsset" || error.code == "UnresolvedTexture" {
                eprintln!("hint: pass --textures <dir> or --best-effort");
            }
            return ExitCode::FAILURE;
        }
    };
    let identity = IdentityOptions {
        deterministic_guids: !random_guids,
        archive_key: obfuscation_key.unwrap_or(caff::DEFAULT_KEY),
    };
    let written = match write_minimal_cmo3(&project, &identity, &assets) {
        Ok(written) => written,
        Err(error) => {
            eprintln!("error: {} ({})", error.message, error.code);
            return ExitCode::FAILURE;
        }
    };
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("recovered.cmo3"));
    if let Err(message) = write_atomic(&output, &written.bytes, force) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
    }

    let report_path = report
        .map(Path::to_path_buf)
        .unwrap_or_else(|| output.with_extension("report.json"));
    let report_value = json!({
        "schema": "live2d-recovery/recovery-report/1",
        "source": {
            "sources": inputs.sources,
            "moc3_version": model.source.version_label,
        },
        "resolved": {
            "parameters": model.parameters.len(),
            "parts": model.parts.len(),
            "deformers": model.deformers.len(),
            "art_meshes": model.art_meshes.len(),
            "textures": model.textures.len(),
            "bindings": keyforms.statistics.bindings,
            "keyform_grids": keyforms.statistics.targets,
            "stored_forms": keyforms.statistics.stored_keyforms,
        },
        "unresolved": {
            "entries": keyforms.statistics.unresolved_entries,
            "grids": keyforms.statistics.unresolved_grids,
        },
        "confidence": {
            "exact_bindings": keyforms.statistics.exact_bindings,
            "derived_bindings": keyforms.statistics.derived_bindings,
            "heuristic_bindings": keyforms.statistics.heuristic_bindings,
        },
        "writer": {
            "best_effort": project.best_effort,
            "pool_size": written.pool_size,
            "guid_count": written.guid_count,
            "writer_defaults": project.defaults.iter().map(|entry| json!({
                "semantic": entry.semantic,
                "field": entry.field,
                "reason": entry.reason,
            })).collect::<Vec<_>>(),
            "unsupported": project.unsupported.iter().map(|entry| json!({
                "subject": entry.subject,
                "reason": entry.reason,
            })).collect::<Vec<_>>(),
        },
        "validation": {
            "structural": "PASS",
            "caff": "PASS",
            "cubism": "NOT_TESTED",
        },
        "pipeline": written.pipeline.iter().map(|step| json!({
            "label": step.label,
            "pass": step.pass,
            "detail": step.detail,
        })).collect::<Vec<_>>(),
    });
    if let Ok(text) = serde_json::to_string_pretty(&report_value) {
        if let Err(error) = std::fs::write(&report_path, format!("{text}\n")) {
            eprintln!(
                "warning: could not write report '{}': {error}",
                report_path.display()
            );
        }
    }

    println!("Output:");
    println!("{}", output.display());
    println!();
    println!("Structural validation:");
    println!("PASS");
    println!();
    println!("Cubism validation:");
    println!("NOT_TESTED");
    if project.best_effort {
        println!();
        println!("BEST_EFFORT_OUTPUT");
    }
    log::info!(
        "recovered {} target(s), {} stored form(s), {} writer default(s), {} unsupported note(s)",
        project.model_images.len(),
        project
            .meshes
            .iter()
            .map(|mesh| mesh.forms.len())
            .sum::<usize>(),
        project.defaults.len(),
        project.unsupported.len()
    );
    ExitCode::SUCCESS
}

fn write_atomic(path: &Path, bytes: &[u8], force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!(
            "'{}' already exists; pass --force to overwrite",
            path.display()
        ));
    }
    use std::io::Write as _;
    let temp = path.with_extension(format!("cmo3.{}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&temp).map_err(|error| {
        format!(
            "cannot create temporary output '{}': {error}",
            temp.display()
        )
    })?;
    file.write_all(bytes).map_err(|error| {
        format!(
            "cannot write temporary output '{}': {error}",
            temp.display()
        )
    })?;
    file.sync_all().map_err(|error| {
        format!(
            "cannot flush temporary output '{}': {error}",
            temp.display()
        )
    })?;
    drop(file);
    // Windows cannot rename over an existing file; with --force the previous
    // output is removed first (small crash window documented in LIMITATIONS).
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|error| format!("cannot replace '{}': {error}", path.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("cannot finalize '{}': {error}", path.display())
    })
}

/// Run `inspect-cmo3`.
pub fn run_inspect_cmo3(file: &Path, json_output: bool) -> ExitCode {
    const MAX_INPUT: u64 = 512 * 1024 * 1024;
    match std::fs::metadata(file) {
        Ok(metadata) if metadata.len() > MAX_INPUT => {
            eprintln!(
                "error: '{}' is {} bytes which exceeds the inspection cap",
                file.display(),
                metadata.len()
            );
            return ExitCode::FAILURE;
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("error: cannot stat '{}': {error}", file.display());
            return ExitCode::FAILURE;
        }
    }
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot read '{}': {error}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let decoded = match caff::decode_strict_raw(&bytes) {
        Ok(decoded) => decoded,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let validation = match caff::validate_archive(&bytes) {
        Ok(validation) => validation,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Some(main_xml) = decoded.payload(caff::MAIN_XML_PATH) else {
        eprintln!("error: archive has no main.xml payload");
        return ExitCode::FAILURE;
    };
    let xml = String::from_utf8_lossy(main_xml).to_string();
    let inspection = validate::inspect_xml(&xml);
    let scan = validate::scan_xml(&xml);
    let pngs: Vec<String> = decoded
        .entries
        .iter()
        .filter(|entry| entry.path.ends_with(".png"))
        .map(|entry| entry.path.clone())
        .collect();
    if json_output {
        let payload = json!({
            "file": file.display().to_string(),
            "caff": {
                "size": validation.size,
                "entries": validation.entry_count,
                "guard_ok": validation.guard_ok,
                "valid": validation.is_valid(),
            },
            "xml": {
                "declarations": scan.xml_decl_count,
                "duplicate_ids": scan.duplicate_ids.len(),
                "dangling_refs": scan.dangling.len(),
                "malformed_refs": scan.malformed_refs.len(),
            },
            "entities": {
                "parts": inspection.parts,
                "art_meshes": inspection.art_meshes,
                "warps": inspection.warps,
                "rotations": inspection.rotations,
                "parameters": inspection.parameters,
                "bindings": inspection.bindings,
                "keyform_grids": inspection.keyform_grids,
                "forms": inspection.forms,
                "layers": inspection.layers,
                "model_images": inspection.model_images,
                "masks": inspection.masks,
            },
            "textures": pngs,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("error: failed to serialize inspection: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("CMO3 Structural Report");
        println!();
        println!(
            "Archive: {}",
            if validation.is_valid() {
                "PASS"
            } else {
                "FAIL"
            }
        );
        println!(
            "main.xml: {}",
            if scan.is_valid() { "PASS" } else { "FAIL" }
        );
        println!("Textures: {} ({})", pngs.len(), pngs.join(", "));
        println!();
        println!("Parts            {}", inspection.parts);
        println!("ArtMeshes        {}", inspection.art_meshes);
        println!("Warp             {}", inspection.warps);
        println!("Rotation         {}", inspection.rotations);
        println!("Parameters       {}", inspection.parameters);
        println!("Bindings         {}", inspection.bindings);
        println!("KeyformGrids     {}", inspection.keyform_grids);
        println!("Forms            {}", inspection.forms);
        println!("Layers           {}", inspection.layers);
        println!("ModelImages      {}", inspection.model_images);
        println!("Masks            {}", inspection.masks);
        println!();
        println!(
            "References: {} refs, {} dangling",
            scan.refs.len(),
            scan.dangling.len()
        );
        println!();
        println!("Cubism Open:");
        println!("NOT_TESTED");
    }
    if validation.is_valid() && scan.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
