use std::fs;
use std::io::Write;
use std::path::Path;

use lzvm_prover::ProveWitnessCommitments;

pub(super) fn write_proof_output(output_dir: &Path, proof_bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            output_dir.display()
        )
    })?;
    write_output_file(&output_dir.join("proof.bin"), proof_bytes)
}

pub(super) fn write_output_file(path: &Path, value: &[u8]) -> Result<(), String> {
    fs::write(path, value)
        .map_err(|error| format!("write output file failed: {}: {error}", path.display()))
}

pub(super) fn write_witness_output_summary(
    stdout: &mut dyn Write,
    commitments: &ProveWitnessCommitments,
) {
    write_witness_output_summary_with_trace(stdout, commitments, false);
}

pub(super) fn write_witness_output_summary_with_trace(
    stdout: &mut dyn Write,
    commitments: &ProveWitnessCommitments,
    include_trace_instance: bool,
) {
    let _ = writeln!(stdout, "unit_index={}", commitments.unit_index());
    if include_trace_instance {
        let _ = writeln!(
            stdout,
            "trace_instance_index={}",
            commitments.trace_instance_index()
        );
    }
    let _ = writeln!(stdout, "input_bytes={}", commitments.input_byte_count());
    let _ = writeln!(stdout, "trace_rows={}", commitments.trace_row_count());
    let _ = writeln!(stdout, "trace_columns={}", commitments.trace_column_count());
    let _ = writeln!(
        stdout,
        "stage_count={}",
        commitments.stage_commitments().stage_count()
    );
    for commitment in commitments.stage_commitments().commitments() {
        let root = commitment
            .root()
            .iter()
            .map(|value| value.to_u64().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(stdout, "stage_{}_root={root}", commitment.stage_index());
        let _ = writeln!(
            stdout,
            "stage_{}_tree_bytes={}",
            commitment.stage_index(),
            commitment.tree_byte_count()
        );
    }
}
