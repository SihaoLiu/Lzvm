use std::process::Command;

#[cfg(unix)]
#[test]
fn profiling_helpers_are_directly_executable() {
    use std::os::unix::fs::PermissionsExt;

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");

    for script in [
        "scripts/ncu-cuda-kernel-summary.py",
        "scripts/nsys-cuda-copy-summary.py",
        "scripts/nsys-cuda-kernel-summary.py",
        "scripts/nsys-cuda-sync-summary.py",
        "scripts/run-proof-timing-batch.py",
    ] {
        let script_path = workspace_root.join(script);
        let mode = std::fs::metadata(&script_path)
            .unwrap_or_else(|error| panic!("{script} metadata should read: {error}"))
            .permissions()
            .mode();
        assert_ne!(
            mode & 0o111,
            0,
            "{script} should be executable as a profiling helper"
        );

        let output = Command::new(&script_path)
            .arg("--self-test")
            .output()
            .unwrap_or_else(|error| {
                panic!("{script} should run directly through its shebang: {error}")
            });
        assert!(
            output.status.success(),
            "{script} direct self-test should pass: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
