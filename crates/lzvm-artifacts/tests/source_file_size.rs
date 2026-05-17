use std::path::Path;

const MAX_PRODUCTION_SOURCE_LINES: usize = 1_300;

#[test]
fn production_rust_sources_stay_under_size_guideline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = crate_root.join("src");
    let mut oversized = Vec::new();

    collect_oversized_sources(&source_root, &source_root, &mut oversized);

    assert!(
        oversized.is_empty(),
        "oversized production Rust source files: {}",
        oversized.join(", ")
    );
}

fn collect_oversized_sources(root: &Path, path: &Path, oversized: &mut Vec<String>) {
    let entries = std::fs::read_dir(path).expect("source directory should read");
    for entry in entries {
        let entry = entry.expect("source entry should read");
        let path = entry.path();
        if path.is_dir() {
            collect_oversized_sources(root, &path, oversized);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }

        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("source file should read: {}: {error}", path.display()));
        let line_count = contents.lines().count();
        if line_count > MAX_PRODUCTION_SOURCE_LINES {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            oversized.push(format!("{} has {line_count} lines", relative.display()));
        }
    }
}
