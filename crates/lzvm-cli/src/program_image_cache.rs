use std::io::Write;
use std::path::Path;

use lzvm_artifacts::program_image::{
    parse_program_image_commitment_cache, ProgramImageCommitmentCache, ProgramImageGpuMode,
};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, program_image_cache_segment_digest,
};
use lzvm_prover::ProveProgramImageCache;

use crate::prove_plan::format_hash;

pub(crate) fn run_summary(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        [cache_path] => summarize_program_image_cache(cache_path, stdout, stderr),
        _ => write_summary_usage(stderr),
    }
}

fn summarize_program_image_cache(
    cache_path: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let path = Path::new(cache_path);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "program-image cache summary failed: read input failed: {cache_path}: {error}"
            );
            return 1;
        }
    };
    let cache = match parse_program_image_commitment_cache(&bytes) {
        Ok(cache) => cache,
        Err(error) => {
            let _ = writeln!(stderr, "program-image cache summary failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "program_image_cache={}", path.display());
    let _ = writeln!(stdout, "bytes={}", bytes.len());
    write_program_image_cache_fields(stdout, &cache);
    0
}

fn write_summary_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup program-image-cache-summary <cache-bin>"
    );
    2
}

pub(crate) fn write_program_image_cache_summary(
    stdout: &mut dyn Write,
    summary: &ProveProgramImageCache,
) {
    let _ = writeln!(stdout, "program_image_cache={}", summary.path.display());
    write_program_image_cache_fields(stdout, &summary.cache);
}

pub(crate) fn write_program_image_cache_fields(
    stdout: &mut dyn Write,
    cache: &ProgramImageCommitmentCache,
) {
    if let Ok(segment) = encode_program_image_cache_segment(cache) {
        write_program_image_cache_fields_with_segment_hash(
            stdout,
            cache,
            &program_image_cache_segment_digest(&segment),
        );
        return;
    }
    write_program_image_cache_fields_without_segment_hash(stdout, cache);
}

pub(crate) fn write_program_image_cache_fields_with_segment_hash(
    stdout: &mut dyn Write,
    cache: &ProgramImageCommitmentCache,
    segment_hash: &[u8; 32],
) {
    let _ = writeln!(
        stdout,
        "program_image_cache_segment_hash={}",
        format_hash(segment_hash)
    );
    write_program_image_cache_fields_without_segment_hash(stdout, cache);
}

fn write_program_image_cache_fields_without_segment_hash(
    stdout: &mut dyn Write,
    cache: &ProgramImageCommitmentCache,
) {
    let _ = writeln!(
        stdout,
        "program_image_cache_program_digest={}",
        format_hash(&cache.program_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_source_image_digest={}",
        format_hash(&cache.source_image_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_constraint_system_digest={}",
        format_hash(&cache.constraint_system_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_tree_root={}",
        format_program_image_tree_root(cache.tree_root)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_rows={}",
        cache.trace_row_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_columns={}",
        cache.trace_column_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_blowup_factor={}",
        cache.blowup_factor
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_arity={}",
        cache.merkle_tree_arity
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_gpu_mode={}",
        format_program_image_gpu_mode(cache.gpu_mode)
    );
}

fn format_program_image_gpu_mode(mode: ProgramImageGpuMode) -> &'static str {
    match mode {
        ProgramImageGpuMode::Cpu => "cpu",
        ProgramImageGpuMode::Cuda => "cuda",
    }
}

fn format_program_image_tree_root(root: [u64; 4]) -> String {
    root.iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
