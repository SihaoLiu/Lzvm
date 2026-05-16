use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn reserved_terms() -> [Vec<u8>; 3] {
    [
        [b"ve".as_slice(), b"nus".as_slice()].concat(),
        [b"zi".as_slice(), b"sk".as_slice()].concat(),
        [b"cir".as_slice(), b"com".as_slice()].concat(),
    ]
}

fn tracked_paths() -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files should run");
    assert!(
        output.status.success(),
        "git ls-files failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(String::from_utf8_lossy(path).into_owned()))
        .collect()
}

fn should_scan(path: &Path, bytes: &[u8]) -> bool {
    let path = path.to_string_lossy();
    !path.starts_with("temp/") && !path.starts_with("target/") && !bytes.contains(&0)
}

fn ascii_lowercase(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(u8::to_ascii_lowercase).collect()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[test]
fn tracked_text_files_avoid_reserved_project_names() {
    let terms = reserved_terms();
    let mut violations = Vec::new();
    for path in tracked_paths() {
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            panic!("tracked file should read: {}: {error}", path.display())
        });
        if !should_scan(&path, &bytes) {
            continue;
        }
        let lower = ascii_lowercase(&bytes);
        for term in &terms {
            if let Some(offset) = find_bytes(&lower, term) {
                violations.push(format!("{} at byte {offset}", path.display()));
                break;
            }
        }
    }

    assert!(
        violations.is_empty(),
        "reserved project names found:\n{}",
        violations.join("\n")
    );
}
