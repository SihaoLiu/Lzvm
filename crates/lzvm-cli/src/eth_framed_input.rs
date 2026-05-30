use std::io::Write;
use std::path::Path;

use lzvm_artifacts::framed_stdin::parse_framed_stdin_chunks;

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [input_path] => summarize_input(input_path, stdout, stderr),
        _ => write_usage(stderr),
    }
}

fn summarize_input(input_path: &str, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth framed input summary failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let chunks = match parse_framed_stdin_chunks(&bytes) {
        Ok(chunks) => chunks,
        Err(error) => {
            let _ = writeln!(stderr, "eth framed input summary failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    let _ = writeln!(stdout, "chunks={}", chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        let _ = writeln!(stdout, "chunk_{index}_offset={}", chunk.offset);
        let _ = writeln!(
            stdout,
            "chunk_{index}_payload_offset={}",
            chunk.payload_offset
        );
        let _ = writeln!(stdout, "chunk_{index}_bytes={}", chunk.payload_len);
        let _ = writeln!(stdout, "chunk_{index}_padding={}", chunk.padding_len);
    }
    0
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm eth framed-input-summary <input>");
    2
}
