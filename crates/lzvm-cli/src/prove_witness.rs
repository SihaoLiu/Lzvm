use std::fs;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_field::Felt;
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_material_manifest_segment,
    build_pcs_query_plan_segment, build_witness_commitment_segment, build_witness_opening_segment,
    derive_prove_execution_plan, proof_values::build_pcs_proof_values_segment_from_packed_values,
    run_prove_witness_commitments_with_auxiliary_inputs,
    unit_values::build_unit_values_segment_from_packed_values, ProveExecutionInputArtifacts,
    ProveSchedule, ProveWitnessAuxiliaryInputs, ProveWitnessCommitments,
};

use crate::prove_plan::{parse_run_args, write_run_plan_summary, ParseError, ParsedRunArgs};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_witness_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };

    let catalog = match read_key_directory_catalog(&parsed.run_args.positionals[0]) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed.run_args);
    let plan = match derive_prove_execution_plan(&catalog, parsed.run_args.request, inputs) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    let auxiliary_inputs = match load_witness_auxiliary_inputs(
        parsed.unit_values.as_deref(),
        parsed.proof_values.as_deref(),
    ) {
        Ok(inputs) => inputs,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let output =
        match run_prove_witness_commitments_with_auxiliary_inputs(&plan, 0, auxiliary_inputs) {
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
            parsed.unit_values.as_deref(),
            parsed.proof_values.as_deref(),
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

struct ParsedWitnessArgs {
    run_args: ParsedRunArgs,
    unit_values: Option<std::path::PathBuf>,
    proof_values: Option<std::path::PathBuf>,
}

fn parse_witness_args(args: &[&str]) -> Result<ParsedWitnessArgs, ParseError> {
    let mut unit_values = None;
    let mut proof_values = None;
    let mut filtered = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--unit-values" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| ParseError::Invalid("missing --unit-values value".to_owned()))?;
                if unit_values.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --unit-values option".to_owned(),
                    ));
                }
            }
            "--proof-values" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --proof-values value".to_owned())
                })?;
                if proof_values.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --proof-values option".to_owned(),
                    ));
                }
            }
            _ => filtered.push(args[index]),
        }
        index += 1;
    }
    Ok(ParsedWitnessArgs {
        run_args: parse_run_args(&filtered, 4, 5)?,
        unit_values,
        proof_values,
    })
}

fn parsed_inputs(parsed: &ParsedRunArgs) -> ProveExecutionInputArtifacts {
    ProveExecutionInputArtifacts {
        witness_library: parsed.positionals[2].clone(),
        guest_image: parsed.positionals[3].clone(),
        public_inputs: parsed.positionals.get(4).cloned(),
    }
}

fn load_witness_auxiliary_inputs(
    unit_values_input: Option<&Path>,
    proof_values_input: Option<&Path>,
) -> Result<ProveWitnessAuxiliaryInputs, String> {
    Ok(ProveWitnessAuxiliaryInputs {
        unit_values: match unit_values_input {
            Some(path) => read_packed_values(path, "unit values")?,
            None => Vec::new(),
        },
        proof_values: match proof_values_input {
            Some(path) => read_packed_values(path, "proof values")?,
            None => Vec::new(),
        },
        ..ProveWitnessAuxiliaryInputs::default()
    })
}

fn save_witness_outputs(
    output_dir: &Path,
    catalog: &lzvm_artifacts::key_directory::KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_inputs: Option<&Path>,
    unit_values_input: Option<&Path>,
    proof_values_input: Option<&Path>,
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
    let proof_bytes = build_proof_bytes(
        catalog,
        schedule,
        public_inputs,
        unit_values_input,
        proof_values_input,
        output,
        &segment,
    )?;

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
    unit_values_input: Option<&Path>,
    proof_values_input: Option<&Path>,
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
    let packed_unit_values = match unit_values_input {
        Some(path) => read_packed_values(path, "unit values")?,
        None => Vec::new(),
    };
    let unit_index = output.unit_index();
    let unit = schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let unit_values_segment = build_unit_values_segment_from_packed_values(
        unit_index,
        &unit.unit_value_map,
        &packed_unit_values,
    )
    .map_err(|error| format!("build unit values segment failed: {error}"))?;
    let packed_proof_values = match proof_values_input {
        Some(path) => read_packed_values(path, "proof values")?,
        None => Vec::new(),
    };
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &catalog.layout.global_info,
        &packed_proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
        segment.clone(),
    ];
    if let Some(proof_values_segment) = proof_values_segment {
        segments.push(proof_values_segment);
    }
    if let Some(unit_values_segment) = unit_values_segment {
        segments.push(unit_values_segment);
    }
    let proof = ProofArtifact {
        setup_hash: schedule.setup_hash,
        public_values_hash,
        segments,
    };
    encode_proof_artifact(&proof)
        .map(Some)
        .map_err(|error| format!("encode witness proof artifact failed: {error}"))
}

fn read_packed_values(path: &Path, label: &str) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read {label} input failed: {}: {error}", path.display()))?;
    if bytes.len() % 8 != 0 {
        return Err(format!(
            "read {label} input failed: {}: byte length is not aligned to field words",
            path.display()
        ));
    }
    bytes
        .chunks_exact(8)
        .enumerate()
        .map(|(index, chunk)| {
            let value = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
            Felt::from_canonical(value).map_err(|error| {
                format!(
                    "read {label} input failed: {}: field word {index} is invalid: {error}",
                    path.display()
                )
            })
        })
        .collect()
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
