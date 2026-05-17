use std::fs;
use std::path::{Path, PathBuf};

const MAX_SOURCE_LINES: usize = 1_300;

#[test]
fn production_rust_sources_stay_under_size_limit() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_sources(&src_dir, &mut files);

    let oversized = files
        .into_iter()
        .filter_map(|path| {
            let contents = fs::read_to_string(&path).expect("source file should be readable");
            let line_count = contents.lines().count();
            (line_count > MAX_SOURCE_LINES).then(|| {
                let relative = path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .expect("source path should be under crate")
                    .display()
                    .to_string();
                format!("{relative} has {line_count} lines")
            })
        })
        .collect::<Vec<_>>();

    assert!(
        oversized.is_empty(),
        "oversized production Rust source files: {}",
        oversized.join(", ")
    );
}

fn collect_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source directory should be readable") {
        let path = entry.expect("source entry should be readable").path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}
