use std::io::Write;

use lzvm_artifacts::program_image::{ProgramImageCommitmentCache, ProgramImageGpuMode};
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
