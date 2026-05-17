use std::io::Write;

use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, program_image_cache_segment_digest,
};
use lzvm_prover::ProveProgramImageCache;

use crate::prove_plan::format_hash;

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
        let _ = writeln!(
            stdout,
            "program_image_cache_segment_hash={}",
            format_hash(&program_image_cache_segment_digest(&segment))
        );
    }
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
