//! Write all synthetic fixtures to a directory (default `fixtures/synthetic`).

#![forbid(unsafe_code)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "fixtures/synthetic".to_string());
    let path = std::path::Path::new(&out_dir);
    if let Err(error) = std::fs::create_dir_all(path) {
        eprintln!("error: could not create '{}': {error}", path.display());
        std::process::exit(1);
    }
    let fixtures = fixture_gen::all_fixtures();
    for fixture in &fixtures {
        let target = path.join(fixture.name);
        if let Err(error) = std::fs::write(&target, &fixture.bytes) {
            eprintln!("error: could not write '{}': {error}", target.display());
            std::process::exit(1);
        }
        println!("wrote {} ({} bytes)", target.display(), fixture.bytes.len());
    }
}
