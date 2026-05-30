use std::path::Path;

const MAX_NATIVE_SOURCE_LINES: usize = 1_300;

#[test]
fn native_sources_stay_under_size_guideline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_root = crate_root.join("native");
    let mut oversized = Vec::new();

    collect_oversized_sources(&native_root, &native_root, &mut oversized);

    assert!(
        oversized.is_empty(),
        "oversized native source files: {}",
        oversized.join(", ")
    );
}

#[test]
fn row_major_poseidon_helpers_do_not_allocate_packed_states() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_poseidon2_row_major.cuh");
    let source =
        std::fs::read_to_string(&source_path).expect("row-major native source should read");

    assert!(
        !source.contains("DeviceBuffer<uint64_t>"),
        "row-major Poseidon helpers should not allocate a packed device buffer"
    );
    assert!(
        !source.contains("cudaMemset"),
        "row-major Poseidon helpers should not clear packed state buffers"
    );
}

#[test]
fn merkle_parent_poseidon_helpers_do_not_allocate_packed_states() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(crate_root.join("native/cuda_poseidon2_merkle_parent.cuh"))
            .expect("Merkle parent native source should read");

    for body in [
        function_body(
            &source,
            "int run_poseidon2_width8_merkle_parent_on_device",
            "int run_poseidon2_width16_merkle_parent_on_device",
        ),
        function_body(
            &source,
            "int run_poseidon2_width16_merkle_parent_on_device",
            "*** end of Merkle parent source ***",
        ),
    ] {
        assert!(
            !body.contains("DeviceBuffer<uint64_t>"),
            "Merkle parent helpers should not allocate a packed device buffer"
        );
        assert!(
            !body.contains("cudaMemset"),
            "Merkle parent helpers should not clear packed state buffers"
        );
    }
}

fn collect_oversized_sources(root: &Path, path: &Path, oversized: &mut Vec<String>) {
    let entries = std::fs::read_dir(path).expect("native source directory should read");
    for entry in entries {
        let entry = entry.expect("native source entry should read");
        let path = entry.path();
        if path.is_dir() {
            collect_oversized_sources(root, &path, oversized);
            continue;
        }
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("cu" | "cuh" | "cpp" | "hpp")
        ) {
            continue;
        }

        let contents = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "native source file should read: {}: {error}",
                path.display()
            )
        });
        let line_count = contents.lines().count();
        if line_count > MAX_NATIVE_SOURCE_LINES {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            oversized.push(format!("{} has {line_count} lines", relative.display()));
        }
    }
}

fn function_body<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let body = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing function start: {start}"))
        .1;
    body.split_once(end).map_or(body, |(body, _)| body)
}
