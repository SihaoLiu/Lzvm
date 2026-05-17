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

fn should_scan_path(path: &Path) -> bool {
    let path = path.to_string_lossy();
    !path.starts_with("temp/") && !path.starts_with("target/")
}

fn should_scan_contents(bytes: &[u8]) -> bool {
    !bytes.contains(&0)
}

fn ascii_lowercase(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(u8::to_ascii_lowercase).collect()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn reserved_term_offset(haystack: &[u8], terms: &[Vec<u8>]) -> Option<usize> {
    let lower = ascii_lowercase(haystack);
    terms.iter().find_map(|term| find_bytes(&lower, term))
}

fn reserved_term_violation(path: &Path, bytes: &[u8], terms: &[Vec<u8>]) -> Option<String> {
    if !should_scan_path(path) {
        return None;
    }
    let path_text = path.to_string_lossy();
    if let Some(offset) = reserved_term_offset(path_text.as_bytes(), terms) {
        return Some(format!("{} path byte {offset}", path.display()));
    }
    if should_scan_contents(bytes) {
        if let Some(offset) = reserved_term_offset(bytes, terms) {
            return Some(format!("{} at byte {offset}", path.display()));
        }
    }
    None
}

#[test]
fn tracked_file_paths_avoid_reserved_project_names() {
    let terms = reserved_terms();
    let mut path = b"docs/".to_vec();
    path.extend_from_slice(&terms[0]);
    path.extend_from_slice(b"/notes.md");
    let path = PathBuf::from(String::from_utf8(path).expect("path should be utf8"));

    assert!(
        reserved_term_violation(&path, b"plain text", &terms).is_some(),
        "reserved project names in tracked paths should be reported"
    );
}

#[test]
fn tracked_binary_file_paths_avoid_reserved_project_names() {
    let terms = reserved_terms();
    let mut path = b"fixtures/".to_vec();
    path.extend_from_slice(&terms[1]);
    path.extend_from_slice(b".bin");
    let path = PathBuf::from(String::from_utf8(path).expect("path should be utf8"));

    assert!(
        reserved_term_violation(&path, b"\0binary", &terms).is_some(),
        "reserved project names in binary file paths should be reported"
    );
}

#[test]
fn tracked_text_files_avoid_reserved_project_names() {
    let terms = reserved_terms();
    let mut violations = Vec::new();
    for path in tracked_paths() {
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            panic!("tracked file should read: {}: {error}", path.display())
        });
        if let Some(violation) = reserved_term_violation(&path, &bytes, &terms) {
            violations.push(violation);
        }
    }

    assert!(
        violations.is_empty(),
        "reserved project names found:\n{}",
        violations.join("\n")
    );
}
