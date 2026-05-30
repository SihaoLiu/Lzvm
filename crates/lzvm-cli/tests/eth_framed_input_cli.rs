use std::fs;
use std::path::{Path, PathBuf};

use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-eth-framed-input-cli-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should be created");
    }
    fs::write(path, bytes).expect("fixture bytes should be written");
}

fn framed_section(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(data.len() as u64).to_le_bytes());
    encoded.extend_from_slice(data);
    let padding = (8 - ((8 + data.len()) % 8)) % 8;
    encoded.extend(std::iter::repeat_n(0, padding));
    encoded
}

#[test]
fn summarizes_framed_input_chunks() {
    let dir = temp_dir("summary");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("input.bin");
    let mut input = framed_section(b"public");
    input.extend_from_slice(&framed_section(b"witness-data"));
    write_bytes(&input_path, &input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "framed-input-summary",
            input_path.to_str().expect("input path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\ninput={}\nbytes=40\nchunks=2\nchunk_0_offset=0\nchunk_0_payload_offset=8\nchunk_0_bytes=6\nchunk_0_padding=2\nchunk_1_offset=16\nchunk_1_payload_offset=24\nchunk_1_bytes=12\nchunk_1_padding=4\n",
            input_path.display()
        )
    );
}

#[test]
fn reports_truncated_framed_input_chunks() {
    let dir = temp_dir("truncated");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("input.bin");
    write_bytes(&input_path, [4_u8, 0, 0, 0, 0, 0, 0, 0, b'a']);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "framed-input-summary",
            input_path.to_str().expect("input path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth framed input summary failed: truncated chunk 0: expected 16 bytes, found 9\n"
    );
}
