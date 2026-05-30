use std::io::Write;
use std::path::Path;

use lzvm_artifacts::framed_stdin::parse_framed_stdin_chunks;

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [input_path] => summarize_input(input_path, stdout, stderr),
        _ => write_usage(stderr),
    }
}

pub(crate) fn run_write_chunk(
    args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match args {
        [input_path, chunk_index, output_path] => {
            write_chunk(input_path, chunk_index, output_path, stdout, stderr)
        }
        _ => write_chunk_usage(stderr),
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

fn write_chunk(
    input_path: &str,
    chunk_index: &str,
    output_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let chunk_index = match parse_chunk_index(chunk_index) {
        Ok(index) => index,
        Err(message) => {
            let _ = writeln!(stderr, "eth framed input chunk write failed: {message}");
            return 1;
        }
    };
    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "eth framed input chunk write failed: read input failed: {input_path}: {error}"
            );
            return 1;
        }
    };
    let chunks = match parse_framed_stdin_chunks(&bytes) {
        Ok(chunks) => chunks,
        Err(error) => {
            let _ = writeln!(stderr, "eth framed input chunk write failed: {error}");
            return 1;
        }
    };
    let Some(chunk) = chunks.get(chunk_index) else {
        let _ = writeln!(
            stderr,
            "eth framed input chunk write failed: chunk index {chunk_index} out of range: chunks={}",
            chunks.len()
        );
        return 1;
    };

    let output = Path::new(output_path);
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "eth framed input chunk write failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output, &chunk.data) {
        let _ = writeln!(
            stderr,
            "eth framed input chunk write failed: write output failed: {}: {error}",
            output.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "input={}", Path::new(input_path).display());
    let _ = writeln!(stdout, "chunk={chunk_index}");
    let _ = writeln!(stdout, "chunk_offset={}", chunk.offset);
    let _ = writeln!(stdout, "chunk_payload_offset={}", chunk.payload_offset);
    let _ = writeln!(stdout, "bytes={}", chunk.payload_len);
    let _ = writeln!(stdout, "output={}", output.display());
    0
}

fn parse_chunk_index(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid chunk index: {value}"))
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm eth framed-input-summary <input>");
    2
}

fn write_chunk_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm eth write-framed-input-chunk <input> <chunk-index> <out>"
    );
    2
}
