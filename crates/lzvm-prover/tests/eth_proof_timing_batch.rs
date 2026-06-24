use std::process::Command;

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
}

fn script_path() -> std::path::PathBuf {
    workspace_root().join("scripts/run-eth-proof-timing-batch.py")
}

fn test_dir(name: &str) -> std::path::PathBuf {
    workspace_root().join(format!("temp/{name}-{}", std::process::id()))
}

#[test]
fn eth_proof_timing_batch_self_test_runs() {
    let output = Command::new(script_path())
        .arg("--self-test")
        .output()
        .expect("ETH proof timing batch self-test should run");

    assert!(
        output.status.success(),
        "ETH proof timing batch self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("small_runs=3"),
        "self-test should run the small command: {stdout}"
    );
    assert!(
        stdout.contains("large_runs=3"),
        "self-test should run the large command: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_builds_small_command_from_env() {
    let dir = test_dir("eth-proof-timing-batch-dry-run");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let fake_bin = dir.join("lzvm");
    std::fs::write(&fake_bin, b"fixture").expect("fake binary should write");
    let setup = dir.join("setup");
    std::fs::create_dir_all(&setup).expect("setup dir should be created");
    let block_input = write_fixture(&dir, "block.input");
    let cache = write_fixture(&dir, "program-image.cache");
    let input_data = write_fixture(&dir, "input-data.bin");
    let guest = write_fixture(&dir, "guest.elf");

    let output = Command::new(script_path())
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--summary")
        .arg("dry run")
        .env("LZVM_REAL_SMALL_PARITY_BIN", &fake_bin)
        .env("LZVM_REAL_SMALL_PARITY_SETUP", &setup)
        .env("LZVM_REAL_SMALL_PARITY_BLOCK_INPUT", &block_input)
        .env("LZVM_REAL_SMALL_PARITY_PROGRAM_IMAGE_CACHE", &cache)
        .env("LZVM_REAL_SMALL_PARITY_INPUT_DATA", &input_data)
        .env("LZVM_REAL_SMALL_PARITY_GUEST_IMAGE", &guest)
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "dry-run should build a small command: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("--require-proof-output"),
        "runner command should require proof markers: {stdout}"
    );
    assert!(
        stdout.contains("small_command=env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "small command should clear pipeline environment: {stdout}"
    );
    assert!(
        stdout.contains("{batch_dir}/small-{run_padded}.proof"),
        "small command should use a unique per-run output directory: {stdout}"
    );
    assert!(
        stdout.contains("--eth-block-input"),
        "small command should pass the block input: {stdout}"
    );
}

fn write_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"fixture").expect("fixture should write");
    path
}
