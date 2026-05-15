use std::fs;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_material_manifest_segment,
    build_pcs_query_plan_segment, build_witness_commitment_segment, build_witness_opening_segment,
    derive_prove_execution_plan, run_prove_witness_commitments, ProveExecutionInputArtifacts,
    ProveSchedule, ProveWitnessCommitments,
};

use crate::prove_plan::{parse_run_args, write_run_plan_summary, ParseError, ParsedRunArgs};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_run_args(args, 4, 5) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };

    let catalog = match read_key_directory_catalog(&parsed.positionals[0]) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed);
    let plan = match derive_prove_execution_plan(&catalog, parsed.request, inputs) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    let output = match run_prove_witness_commitments(&plan, 0) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    if plan.run_plan.options.save_outputs {
        if let Err(message) = save_witness_outputs(
            &plan.run_plan.options.output_dir,
            &catalog,
            &plan.run_plan.schedule,
            plan.inputs.public_inputs.as_deref(),
            &output,
        ) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    }

    write_run_plan_summary(stdout, &plan.run_plan);
    let _ = writeln!(stdout, "unit_index={}", output.unit_index());
    let _ = writeln!(stdout, "input_bytes={}", output.input_byte_count());
    let _ = writeln!(stdout, "trace_rows={}", output.trace_row_count());
    let _ = writeln!(stdout, "trace_columns={}", output.trace_column_count());
    let _ = writeln!(
        stdout,
        "stage_count={}",
        output.stage_commitments().stage_count()
    );
    for commitment in output.stage_commitments().commitments() {
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
            commitment.tree_bytes().len()
        );
    }
    0
}

fn parsed_inputs(parsed: &ParsedRunArgs) -> ProveExecutionInputArtifacts {
    ProveExecutionInputArtifacts {
        witness_library: parsed.positionals[2].clone(),
        guest_image: parsed.positionals[3].clone(),
        public_inputs: parsed.positionals.get(4).cloned(),
    }
}

fn save_witness_outputs(
    output_dir: &Path,
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_inputs: Option<&Path>,
    output: &ProveWitnessCommitments,
) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            output_dir.display()
        )
    })?;

    let segment = build_witness_commitment_segment(output)
        .map_err(|error| format!("build witness segment failed: {error}"))?;
    let proof_bytes = build_proof_bytes(catalog, schedule, public_inputs, output, &segment)?;

    for commitment in output.stage_commitments().commitments() {
        let root_path = output_dir.join(format!(
            "unit-{}-stage-{}.witness-root",
            output.unit_index(),
            commitment.stage_index()
        ));
        let tree_path = output_dir.join(format!(
            "unit-{}-stage-{}.witness-tree",
            output.unit_index(),
            commitment.stage_index()
        ));
        let mut root_bytes = Vec::with_capacity(32);
        for value in commitment.root() {
            root_bytes.extend_from_slice(&value.to_le_bytes());
        }
        write_output_file(&root_path, &root_bytes)?;
        write_output_file(&tree_path, commitment.tree_bytes())?;
    }
    let segment_path = output_dir.join(format!("unit-{}.witness-segment", output.unit_index()));
    write_output_file(&segment_path, &segment.data)?;
    if let Some(proof_bytes) = proof_bytes {
        write_output_file(&output_dir.join("proof.bin"), &proof_bytes)?;
    }
    Ok(())
}

fn build_proof_bytes(
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_inputs: Option<&Path>,
    output: &ProveWitnessCommitments,
    segment: &ProofSegment,
) -> Result<Option<Vec<u8>>, String> {
    let Some(public_inputs) = public_inputs else {
        return Ok(None);
    };
    let public_values = read_public_values_file(public_inputs)
        .map_err(|error| format!("read public inputs failed: {error}"))?;
    if public_values.setup_hash != schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(&public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    let material_segment = build_pcs_material_manifest_segment(schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let query_segment = build_pcs_query_plan_segment(
        schedule,
        public_values_hash,
        &material_segment,
        std::slice::from_ref(segment),
    )
    .map_err(|error| format!("build query plan segment failed: {error}"))?;
    let constant_opening_segment =
        build_constant_opening_segment(catalog, schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment = build_witness_opening_segment(schedule, &query_segment, output)
        .map_err(|error| format!("build witness opening segment failed: {error}"))?;
    let proof = ProofArtifact {
        setup_hash: schedule.setup_hash,
        public_values_hash,
        segments: vec![
            material_segment,
            query_segment,
            constant_opening_segment,
            opening_segment,
            segment.clone(),
        ],
    };
    encode_proof_artifact(&proof)
        .map(Some)
        .map_err(|error| format!("encode witness proof artifact failed: {error}"))
}

fn write_output_file(path: &Path, value: &[u8]) -> Result<(), String> {
    fs::write(path, value)
        .map_err(|error| format!("write output file failed: {}: {error}", path.display()))
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]"
    );
    2
}
