use std::fs;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::key_directory::{read_key_directory_catalog, KeyDirectoryCatalog};
use lzvm_artifacts::pcs_nonce_segment::parse_pcs_query_nonce_segment;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_evaluation_segment,
    build_pcs_fri_opening_segment_from_transcript_values,
    build_pcs_fri_transcript_values_from_trace_segments, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge, build_witness_commitment_segment,
    build_witness_opening_segment, derive_prove_execution_plan,
    proof_values::build_pcs_proof_values_segment_from_packed_values,
    run_prove_witness_commitments_with_trace,
    unit_values::build_unit_values_segment_from_packed_values, ProveExecutionInputArtifacts,
    ProveExecutionUnitArtifacts, ProvePcsEvaluationValues, ProvePcsFriTranscriptTraceSegmentValues,
    ProveSchedule, ProveWitnessAuxiliaryInputs, ProveWitnessTraceCommitments,
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
        parsed.evaluation_values.as_deref(),
    ) {
        Ok(inputs) => inputs,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let output = match run_prove_witness_commitments_with_trace(&plan, 0, auxiliary_inputs) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    if plan.run_plan.options.save_outputs {
        let output_unit_index = output.commitments().unit_index();
        let execution_unit = match plan.units.get(output_unit_index) {
            Some(unit) => unit,
            None => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: output unit index out of range: {output_unit_index}"
                );
                return 1;
            }
        };
        let request = WitnessOutputSaveRequest {
            output_dir: &plan.run_plan.options.output_dir,
            catalog: &catalog,
            schedule: &plan.run_plan.schedule,
            execution_unit,
            public_inputs: plan.inputs.public_inputs.as_deref(),
            unit_values_input: parsed.unit_values.as_deref(),
            proof_values_input: parsed.proof_values.as_deref(),
            output: &output,
        };
        if let Err(message) = save_witness_outputs(request) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    }

    write_run_plan_summary(stdout, &plan.run_plan);
    let commitments = output.commitments();
    let _ = writeln!(stdout, "unit_index={}", commitments.unit_index());
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
            commitment.tree_bytes().len()
        );
    }
    0
}

struct ParsedWitnessArgs {
    run_args: ParsedRunArgs,
    unit_values: Option<std::path::PathBuf>,
    proof_values: Option<std::path::PathBuf>,
    evaluation_values: Option<std::path::PathBuf>,
}

fn parse_witness_args(args: &[&str]) -> Result<ParsedWitnessArgs, ParseError> {
    let mut unit_values = None;
    let mut proof_values = None;
    let mut evaluation_values = None;
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
            "--evaluation-values" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --evaluation-values value".to_owned())
                })?;
                if evaluation_values.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --evaluation-values option".to_owned(),
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
        evaluation_values,
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
    evaluation_values_input: Option<&Path>,
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
        evaluations: match evaluation_values_input {
            Some(path) => read_packed_extension_values(path, "evaluation values")?,
            None => Vec::new(),
        },
        ..ProveWitnessAuxiliaryInputs::default()
    })
}

struct WitnessOutputSaveRequest<'a> {
    output_dir: &'a Path,
    catalog: &'a KeyDirectoryCatalog,
    schedule: &'a ProveSchedule,
    execution_unit: &'a ProveExecutionUnitArtifacts,
    public_inputs: Option<&'a Path>,
    unit_values_input: Option<&'a Path>,
    proof_values_input: Option<&'a Path>,
    output: &'a ProveWitnessTraceCommitments,
}

fn save_witness_outputs(request: WitnessOutputSaveRequest<'_>) -> Result<(), String> {
    fs::create_dir_all(request.output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            request.output_dir.display()
        )
    })?;

    let commitments = request.output.commitments();
    let segment = build_witness_commitment_segment(commitments)
        .map_err(|error| format!("build witness segment failed: {error}"))?;
    let proof_bytes = build_proof_bytes(&request, &segment)?;

    for commitment in commitments.stage_commitments().commitments() {
        let root_path = request.output_dir.join(format!(
            "unit-{}-stage-{}.witness-root",
            commitments.unit_index(),
            commitment.stage_index()
        ));
        let tree_path = request.output_dir.join(format!(
            "unit-{}-stage-{}.witness-tree",
            commitments.unit_index(),
            commitment.stage_index()
        ));
        let mut root_bytes = Vec::with_capacity(32);
        for value in commitment.root() {
            root_bytes.extend_from_slice(&value.to_le_bytes());
        }
        write_output_file(&root_path, &root_bytes)?;
        write_output_file(&tree_path, commitment.tree_bytes())?;
    }
    let segment_path = request
        .output_dir
        .join(format!("unit-{}.witness-segment", commitments.unit_index()));
    write_output_file(&segment_path, &segment.data)?;
    if let Some(proof_bytes) = proof_bytes {
        write_output_file(&request.output_dir.join("proof.bin"), &proof_bytes)?;
    }
    Ok(())
}

fn build_proof_bytes(
    request: &WitnessOutputSaveRequest<'_>,
    segment: &ProofSegment,
) -> Result<Option<Vec<u8>>, String> {
    let Some(public_inputs) = request.public_inputs else {
        return Ok(None);
    };
    let public_values = read_public_values_file(public_inputs)
        .map_err(|error| format!("read public inputs failed: {error}"))?;
    if public_values.setup_hash != request.schedule.setup_hash {
        return Err("public inputs setup hash mismatch".to_owned());
    }
    let public_values_hash = public_values_digest(&public_values)
        .map_err(|error| format!("hash public inputs failed: {error}"))?;
    let material_segment = build_pcs_material_manifest_segment(request.schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let commitments = request.output.commitments();
    let transcript_values = if request.output.auxiliary_inputs().evaluations.is_empty() {
        None
    } else {
        let evaluation_segment = build_pcs_evaluation_segment(
            request.schedule,
            &[ProvePcsEvaluationValues {
                unit_index: commitments.unit_index(),
                values: request.output.auxiliary_inputs().evaluations.clone(),
            }],
        )
        .map_err(|error| format!("build evaluation segment failed: {error}"))?;
        let values = build_pcs_fri_transcript_values_from_trace_segments(
            request.schedule,
            &[ProvePcsFriTranscriptTraceSegmentValues {
                unit_index: commitments.unit_index(),
                execution_unit: request.execution_unit,
                trace: request.output.trace(),
                publics: request.output.publics(),
                auxiliary_inputs: request.output.auxiliary_inputs(),
                material_segment: &material_segment,
                witness_segment: segment,
                evaluation_segment: &evaluation_segment,
            }],
        )
        .map_err(|error| format!("build FRI transcript values failed: {error}"))?;
        Some((evaluation_segment, values))
    };
    let query_segment = match &transcript_values {
        Some((_, values)) => {
            let final_query_challenge = values
                .first()
                .ok_or_else(|| "build FRI transcript values failed: no units".to_owned())?
                .commitments
                .final_query_challenge;
            let nonce_segment =
                build_pcs_query_nonce_segment(request.schedule, final_query_challenge)
                    .map_err(|error| format!("build query nonce segment failed: {error}"))?;
            let nonce = Felt::from_u64(
                parse_pcs_query_nonce_segment(&nonce_segment.data)
                    .map_err(|error| format!("parse query nonce segment failed: {error}"))?
                    .nonce,
            );
            let query_segment = build_pcs_query_plan_segment_from_challenge(
                request.schedule,
                std::slice::from_ref(segment),
                final_query_challenge,
                nonce,
            )
            .map_err(|error| format!("build query plan segment failed: {error}"))?;
            (query_segment, Some(nonce_segment))
        }
        None => {
            let query_segment = build_pcs_query_plan_segment(
                request.schedule,
                public_values_hash,
                &material_segment,
                std::slice::from_ref(segment),
            )
            .map_err(|error| format!("build query plan segment failed: {error}"))?;
            (query_segment, None)
        }
    };
    let (query_segment, nonce_segment) = query_segment;
    let constant_opening_segment =
        build_constant_opening_segment(request.catalog, request.schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment(request.schedule, &query_segment, commitments)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;
    let packed_unit_values = match request.unit_values_input {
        Some(path) => read_packed_values(path, "unit values")?,
        None => Vec::new(),
    };
    let unit_index = commitments.unit_index();
    let unit = request
        .schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let unit_values_segment = build_unit_values_segment_from_packed_values(
        unit_index,
        &unit.unit_value_map,
        &packed_unit_values,
    )
    .map_err(|error| format!("build unit values segment failed: {error}"))?;
    let packed_proof_values = match request.proof_values_input {
        Some(path) => read_packed_values(path, "proof values")?,
        None => Vec::new(),
    };
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
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
    if let Some((evaluation_segment, transcript_values)) = transcript_values {
        let fri_segment = build_pcs_fri_opening_segment_from_transcript_values(
            request.schedule,
            &segments[1],
            &transcript_values,
        )
        .map_err(|error| format!("build FRI opening segment failed: {error}"))?;
        segments.push(evaluation_segment);
        segments.push(fri_segment);
    }
    if let Some(nonce_segment) = nonce_segment {
        segments.push(nonce_segment);
    }
    if let Some(proof_values_segment) = proof_values_segment {
        segments.push(proof_values_segment);
    }
    if let Some(unit_values_segment) = unit_values_segment {
        segments.push(unit_values_segment);
    }
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
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

fn read_packed_extension_values(path: &Path, label: &str) -> Result<Vec<Ext3>, String> {
    let values = read_packed_values(path, label)?;
    if values.len() % 3 != 0 {
        return Err(format!(
            "read {label} input failed: {}: field word count is not a multiple of 3",
            path.display()
        ));
    }
    Ok(values
        .chunks_exact(3)
        .map(|chunk| Ext3::new(chunk[0], chunk[1], chunk[2]))
        .collect())
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
