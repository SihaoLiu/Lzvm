use std::process::Command;

#[test]
fn default_dependency_tree_excludes_json_parser() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "lzvm-artifacts", "-e", "normal"])
        .output()
        .expect("cargo tree should run");
    assert!(
        output.status.success(),
        "cargo tree failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("serde_json"),
        "default dependency tree should not include serde_json\n{stdout}"
    );
}
