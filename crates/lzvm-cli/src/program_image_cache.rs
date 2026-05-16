use std::io::Write;

use lzvm_prover::ProveProgramImageCache;

use crate::prove_plan::format_hash;

pub(crate) fn write_program_image_cache_summary(
    stdout: &mut dyn Write,
    summary: &ProveProgramImageCache,
) {
    let _ = writeln!(stdout, "program_image_cache={}", summary.path.display());
    let _ = writeln!(
        stdout,
        "program_image_cache_program_digest={}",
        format_hash(&summary.cache.program_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_source_image_digest={}",
        format_hash(&summary.cache.source_image_digest)
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_rows={}",
        summary.cache.trace_row_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_trace_columns={}",
        summary.cache.trace_column_count
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_blowup_factor={}",
        summary.cache.blowup_factor
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_arity={}",
        summary.cache.merkle_tree_arity
    );
    let _ = writeln!(
        stdout,
        "program_image_cache_gpu_mode={}",
        format_program_image_gpu_mode(summary.cache.gpu_mode)
    );
}

fn format_program_image_gpu_mode(
    mode: lzvm_artifacts::program_image::ProgramImageGpuMode,
) -> &'static str {
    match mode {
        lzvm_artifacts::program_image::ProgramImageGpuMode::Cpu => "cpu",
        lzvm_artifacts::program_image::ProgramImageGpuMode::Cuda => "cuda",
    }
}
