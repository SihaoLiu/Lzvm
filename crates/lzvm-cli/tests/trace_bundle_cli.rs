use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::trace_bundle::{
    encode_trace_bundle, parse_trace_bundle, TraceBundle, TraceBundleUnit,
};
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-trace-bundle-cli-{}-{name}",
            std::process::id()
        ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should be created");
    }
    fs::write(path, bytes).expect("fixture bytes should be written");
}

#[test]
fn writes_trace_bundle_from_unit_trace_files() {
    let dir = temp_dir("write");
    let _ = fs::remove_dir_all(&dir);
    let out = dir.join("trace-bundle.bin");
    let unit0 = dir.join("unit-0.trace");
    let unit2 = dir.join("unit-2.trace");
    write_bytes(&unit0, [1_u8, 2, 3, 4]);
    write_bytes(&unit2, [9_u8, 8]);

    let expected_bundle = TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 0,
                trace_bytes: vec![1, 2, 3, 4],
            },
            TraceBundleUnit {
                unit_index: 2,
                trace_bytes: vec![9, 8],
            },
        ],
    };
    let expected_bytes =
        encode_trace_bundle(&expected_bundle).expect("expected bundle should encode");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "write-trace-bundle",
            out.to_str().expect("output path should be utf-8"),
            "0",
            unit0.to_str().expect("unit path should be utf-8"),
            "2",
            unit2.to_str().expect("unit path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits=2\nbytes_written={}\noutput={}\n",
            expected_bytes.len(),
            out.display()
        )
    );
    let parsed = parse_trace_bundle(&fs::read(&out).expect("bundle output should read"))
        .expect("bundle should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(parsed, expected_bundle);
}

#[test]
fn rejects_trace_bundle_write_without_units() {
    let dir = temp_dir("without-units");
    let out = dir.join("unused-trace-bundle.bin");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "write-trace-bundle",
            out.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let _ = fs::remove_dir_all(&dir);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm prove write-trace-bundle <out-bundle> <unit-index> <trace-bin>...\n"
    );
}

#[test]
fn rejects_trace_bundle_write_duplicate_units() {
    let dir = temp_dir("duplicate");
    let _ = fs::remove_dir_all(&dir);
    let out = dir.join("trace-bundle.bin");
    let unit0 = dir.join("unit-0.trace");
    let unit0_again = dir.join("unit-0-again.trace");
    write_bytes(&unit0, [1_u8]);
    write_bytes(&unit0_again, [2_u8]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "prove",
            "write-trace-bundle",
            out.to_str().expect("output path should be utf-8"),
            "0",
            unit0.to_str().expect("unit path should be utf-8"),
            "0",
            unit0_again.to_str().expect("unit path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "prove trace bundle write failed: duplicate trace bundle unit index: 0\n"
    );
}
