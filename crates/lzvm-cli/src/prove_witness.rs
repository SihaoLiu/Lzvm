use std::fs;
use std::io::Write;
use std::path::Path;

use lzvm_artifacts::challenge_values_segment::parse_challenge_values_segment;
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_nonce_segment::parse_pcs_query_nonce_segment;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::{
    encode_program_image_cache_segment, PROGRAM_IMAGE_CACHE_SEGMENT_ID,
};
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_artifacts::trace_bundle::{read_trace_bundle_file, TraceBundle};
use lzvm_artifacts::unit_values_segment::{parse_unit_values_segment, UNIT_VALUES_SEGMENT_ID};
use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;
use lzvm_field::{Ext3, Felt};
use lzvm_prover::group_values::{build_group_values_segment, load_group_values_from_segments};
use lzvm_prover::proof_values::{
    build_pcs_proof_values_segment_from_packed_values, flatten_pcs_proof_values,
    load_pcs_proof_values_from_segments,
};
use lzvm_prover::setup_preflight::validate_setup_preflight;
use lzvm_prover::unit_values::{
    build_unit_values_segment_from_packed_values_batch, load_unit_values_from_segments,
    ProveUnitValues,
};
use lzvm_prover::witness_loader::{load_witness_library, TraceBytesBackend, WitnessBackend};
use lzvm_prover::{
    build_constant_opening_segment, build_pcs_evaluation_segment,
    build_pcs_fri_opening_segment_from_transcript_values,
    build_pcs_fri_transcript_values_from_trace_segments, build_pcs_material_manifest_segment,
    build_pcs_query_nonce_segment_with_streams, build_pcs_query_plan_segment,
    build_pcs_query_plan_segment_from_challenge, build_pcs_query_plan_segment_with_bindings,
    build_witness_commitment_segment, build_witness_opening_segment,
    build_witness_opening_segment_batch, derive_prove_execution_plan_with_program_image_cache,
    run_prove_witness_commitments_with_trace_backend,
    unit_values::build_unit_values_segment_from_packed_values, ProveExecutionInputArtifacts,
    ProveExecutionPlan, ProveExecutionUnitArtifacts, ProvePcsEvaluationValues,
    ProvePcsFriTranscriptTraceSegmentValues, ProveSchedule, ProveWitnessAuxiliaryInputs,
    ProveWitnessCommitments, ProveWitnessTraceCommitments,
};

use crate::program_image_cache::write_program_image_cache_summary;
use crate::prove_plan::{
    parse_run_args, read_checked_setup_catalog, write_run_plan_summary, ParseError, ParsedRunArgs,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_witness_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };

    let catalog = match read_checked_setup_catalog(&parsed.run_args.positionals[0]) {
        Ok(catalog) => catalog,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed);
    let plan = match derive_prove_execution_plan_with_program_image_cache(
        &catalog,
        parsed.run_args.request.clone(),
        inputs,
        parsed.run_args.program_image_cache.clone(),
    ) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    if plan.run_plan.options.remote_aggregation {
        let _ = writeln!(
            stderr,
            "prove witness failed: remote aggregation is unsupported by prove witness"
        );
        return 1;
    }
    if plan.run_plan.options.final_wrap {
        let _ = writeln!(
            stderr,
            "prove witness failed: final wrap is unsupported by prove witness"
        );
        return 1;
    }
    if parsed.evaluation_values_segment.is_some()
        && !(parsed.all_units || plan.run_plan.options.aggregate)
    {
        let _ = writeln!(
            stderr,
            "prove witness failed: --evaluation-values-segment requires all-units mode"
        );
        return 1;
    }
    if parsed.trace_bytes.is_some() && (parsed.all_units || plan.run_plan.options.aggregate) {
        let _ = writeln!(
            stderr,
            "prove witness failed: --trace-bytes requires a single-unit witness run"
        );
        return 1;
    }
    let auxiliary_request = WitnessAuxiliaryInputRequest {
        global_info: &catalog.layout.global_info,
        unit_values_input: parsed.unit_values.as_deref(),
        proof_values_input: parsed.proof_values.as_deref(),
        proof_values_segment_input: parsed.proof_values_segment.as_deref(),
        group_values_input: parsed.group_values.as_deref(),
        group_values_segment_input: parsed.group_values_segment.as_deref(),
        challenge_values_input: parsed.challenge_values.as_deref(),
        challenge_values_segment_input: parsed.challenge_values_segment.as_deref(),
        evaluation_values_input: parsed.evaluation_values.as_deref(),
    };
    let auxiliary_inputs = match load_witness_auxiliary_inputs(&auxiliary_request) {
        Ok(inputs) => inputs,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let trace_bundle = match &parsed.trace_bundle {
        Some(path) => match read_trace_bundle_file(path) {
            Ok(bundle) => Some(bundle),
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        },
        None => None,
    };
    if let Some(bundle) = &trace_bundle {
        if parsed.all_units || plan.run_plan.options.aggregate {
            let outputs = match run_prove_witness_commitments_for_all_units_with_trace_bundle(
                &plan,
                &auxiliary_inputs,
                bundle,
            ) {
                Ok(outputs) => outputs,
                Err(error) => {
                    let _ = writeln!(stderr, "prove witness failed: {error}");
                    return 1;
                }
            };
            if let Err(message) = finish_all_units_witness_run(
                &catalog,
                &plan,
                &parsed,
                &auxiliary_inputs,
                &outputs,
                stdout,
            ) {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
            return 0;
        }
    }
    let witness_backend: Box<dyn WitnessBackend> = match (
        &plan.inputs.witness_library,
        &parsed.trace_bytes,
        &trace_bundle,
    ) {
        (Some(path), _, _) => match load_witness_library(path) {
            Ok(backend) => Box::new(backend),
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        },
        (None, Some(path), _) => match fs::read(path) {
            Ok(trace_bytes) => Box::new(TraceBytesBackend::new(trace_bytes)),
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: read trace bytes failed: {}: {error}",
                    path.display()
                );
                return 1;
            }
        },
        (None, None, Some(bundle)) => match bundle.trace_bytes_for_unit(0) {
            Some(trace_bytes) => Box::new(TraceBytesBackend::new(trace_bytes.to_vec())),
            None => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: trace bundle is missing unit 0"
                );
                return 1;
            }
        },
        (None, None, None) => {
            let _ = writeln!(
                stderr,
                "prove witness failed: witness library, trace bytes, or trace bundle are required"
            );
            return 1;
        }
    };
    if parsed.all_units || plan.run_plan.options.aggregate {
        let outputs = match run_prove_witness_commitments_for_all_units(
            &plan,
            &auxiliary_inputs,
            witness_backend.as_ref(),
        ) {
            Ok(outputs) => outputs,
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        };
        if let Err(message) = finish_all_units_witness_run(
            &catalog,
            &plan,
            &parsed,
            &auxiliary_inputs,
            &outputs,
            stdout,
        ) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
        return 0;
    }
    let output = match run_prove_witness_commitments_with_trace_backend(
        &plan,
        0,
        auxiliary_inputs,
        witness_backend.as_ref(),
    ) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    let commitments = output.commitments();
    let segment = match build_witness_commitment_segment(commitments) {
        Ok(segment) => segment,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "prove witness failed: build witness segment failed: {error}"
            );
            return 1;
        }
    };
    let output_unit_index = commitments.unit_index();
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
        gpu_streams: plan.run_plan.gpu.max_streams,
        public_inputs: plan.inputs.public_inputs.as_deref(),
        unit_values_input: parsed.unit_values.as_deref(),
        unit_values_segment_input: parsed.unit_values_segment.as_deref(),
        proof_values_input: parsed.proof_values.as_deref(),
        program_image_cache: plan
            .program_image_cache
            .as_ref()
            .map(|summary| &summary.cache),
        output: &output,
    };
    let proof_bytes =
        match build_proof_bytes(&request, &segment, plan.run_plan.options.verify_outputs) {
            Ok(proof_bytes) => proof_bytes,
            Err(message) => {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
        };
    if plan.run_plan.options.save_outputs {
        if let Err(message) = save_witness_outputs(&request, &segment) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    }
    if let Some(proof_bytes) = proof_bytes {
        if let Err(message) = write_proof_output(request.output_dir, &proof_bytes) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    }

    write_run_plan_summary(stdout, &plan.run_plan);
    if let Some(summary) = &plan.program_image_cache {
        write_program_image_cache_summary(stdout, summary);
    }
    write_witness_output_summary(stdout, commitments);
    0
}

struct ParsedWitnessArgs {
    run_args: ParsedRunArgs,
    all_units: bool,
    trace_bytes: Option<std::path::PathBuf>,
    trace_bundle: Option<std::path::PathBuf>,
    unit_values: Option<std::path::PathBuf>,
    unit_values_segment: Option<std::path::PathBuf>,
    proof_values: Option<std::path::PathBuf>,
    proof_values_segment: Option<std::path::PathBuf>,
    group_values: Option<std::path::PathBuf>,
    group_values_segment: Option<std::path::PathBuf>,
    challenge_values: Option<std::path::PathBuf>,
    challenge_values_segment: Option<std::path::PathBuf>,
    evaluation_values: Option<std::path::PathBuf>,
    evaluation_values_segment: Option<std::path::PathBuf>,
}

fn parse_witness_args(args: &[&str]) -> Result<ParsedWitnessArgs, ParseError> {
    let mut all_units = false;
    let mut trace_bytes = None;
    let mut trace_bundle = None;
    let mut unit_values = None;
    let mut unit_values_segment = None;
    let mut proof_values = None;
    let mut proof_values_segment = None;
    let mut group_values = None;
    let mut group_values_segment = None;
    let mut challenge_values = None;
    let mut challenge_values_segment = None;
    let mut evaluation_values = None;
    let mut evaluation_values_segment = None;
    let mut filtered = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--all-units" => all_units = true,
            "--trace-bytes" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| ParseError::Invalid("missing --trace-bytes value".to_owned()))?;
                if trace_bytes.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --trace-bytes option".to_owned(),
                    ));
                }
            }
            "--trace-bundle" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --trace-bundle value".to_owned())
                })?;
                if trace_bundle.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --trace-bundle option".to_owned(),
                    ));
                }
            }
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
            "--unit-values-segment" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --unit-values-segment value".to_owned())
                })?;
                if unit_values_segment.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --unit-values-segment option".to_owned(),
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
            "--proof-values-segment" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --proof-values-segment value".to_owned())
                })?;
                if proof_values_segment.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --proof-values-segment option".to_owned(),
                    ));
                }
            }
            "--group-values" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --group-values value".to_owned())
                })?;
                if group_values.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --group-values option".to_owned(),
                    ));
                }
            }
            "--group-values-segment" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --group-values-segment value".to_owned())
                })?;
                if group_values_segment.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --group-values-segment option".to_owned(),
                    ));
                }
            }
            "--challenge-values" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --challenge-values value".to_owned())
                })?;
                if challenge_values.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --challenge-values option".to_owned(),
                    ));
                }
            }
            "--challenge-values-segment" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --challenge-values-segment value".to_owned())
                })?;
                if challenge_values_segment.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --challenge-values-segment option".to_owned(),
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
            "--evaluation-values-segment" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --evaluation-values-segment value".to_owned())
                })?;
                if evaluation_values_segment.replace((*value).into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --evaluation-values-segment option".to_owned(),
                    ));
                }
            }
            _ => filtered.push(args[index]),
        }
        index += 1;
    }
    if evaluation_values.is_some() && evaluation_values_segment.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --evaluation-values and --evaluation-values-segment".to_owned(),
        ));
    }
    if unit_values.is_some() && unit_values_segment.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --unit-values and --unit-values-segment".to_owned(),
        ));
    }
    if proof_values.is_some() && proof_values_segment.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --proof-values and --proof-values-segment".to_owned(),
        ));
    }
    if group_values.is_some() && group_values_segment.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --group-values and --group-values-segment".to_owned(),
        ));
    }
    if challenge_values.is_some() && challenge_values_segment.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --challenge-values and --challenge-values-segment".to_owned(),
        ));
    }
    if trace_bytes.is_some() && trace_bundle.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --trace-bytes and --trace-bundle".to_owned(),
        ));
    }
    let trace_mode = trace_bytes.is_some() || trace_bundle.is_some();
    let min_positionals = if trace_mode { 3 } else { 4 };
    let max_positionals = if trace_mode { 4 } else { 5 };
    Ok(ParsedWitnessArgs {
        run_args: parse_run_args(&filtered, min_positionals, max_positionals)?,
        all_units,
        trace_bytes,
        trace_bundle,
        unit_values,
        unit_values_segment,
        proof_values,
        proof_values_segment,
        group_values,
        group_values_segment,
        challenge_values,
        challenge_values_segment,
        evaluation_values,
        evaluation_values_segment,
    })
}

fn parsed_inputs(parsed: &ParsedWitnessArgs) -> ProveExecutionInputArtifacts {
    let trace_mode = parsed.trace_bytes.is_some() || parsed.trace_bundle.is_some();
    let witness_library = if trace_mode {
        None
    } else {
        Some(parsed.run_args.positionals[2].clone())
    };
    let guest_image_index = if trace_mode { 2 } else { 3 };
    let public_inputs_index = if trace_mode { 3 } else { 4 };
    ProveExecutionInputArtifacts {
        witness_library,
        guest_image: parsed.run_args.positionals[guest_image_index].clone(),
        public_inputs: parsed
            .run_args
            .positionals
            .get(public_inputs_index)
            .cloned(),
    }
}

struct WitnessAuxiliaryInputRequest<'a> {
    global_info: &'a GlobalInfo,
    unit_values_input: Option<&'a Path>,
    proof_values_input: Option<&'a Path>,
    proof_values_segment_input: Option<&'a Path>,
    group_values_input: Option<&'a Path>,
    group_values_segment_input: Option<&'a Path>,
    challenge_values_input: Option<&'a Path>,
    challenge_values_segment_input: Option<&'a Path>,
    evaluation_values_input: Option<&'a Path>,
}

fn load_witness_auxiliary_inputs(
    request: &WitnessAuxiliaryInputRequest<'_>,
) -> Result<ProveWitnessAuxiliaryInputs, String> {
    Ok(ProveWitnessAuxiliaryInputs {
        unit_values: match request.unit_values_input {
            Some(path) => read_packed_values(path, "unit values")?,
            None => Vec::new(),
        },
        proof_values: match request.proof_values_input {
            Some(path) => read_packed_values(path, "proof values")?,
            None => match request.proof_values_segment_input {
                Some(path) => read_packed_proof_values_segment(request.global_info, path)?,
                None => Vec::new(),
            },
        },
        group_values: match request.group_values_input {
            Some(path) => read_packed_extension_values(path, "group values")?,
            None => match request.group_values_segment_input {
                Some(path) => read_group_values_segment_input(request.global_info, path)?,
                None => Vec::new(),
            },
        },
        challenges: match request.challenge_values_input {
            Some(path) => read_packed_extension_values(path, "challenge values")?,
            None => match request.challenge_values_segment_input {
                Some(path) => read_challenge_values_segment_input(path)?,
                None => Vec::new(),
            },
        },
        evaluations: match request.evaluation_values_input {
            Some(path) => read_packed_extension_values(path, "evaluation values")?,
            None => Vec::new(),
        },
    })
}

struct WitnessOutputSaveRequest<'a> {
    output_dir: &'a Path,
    catalog: &'a KeyDirectoryCatalog,
    schedule: &'a ProveSchedule,
    execution_unit: &'a ProveExecutionUnitArtifacts,
    gpu_streams: usize,
    public_inputs: Option<&'a Path>,
    unit_values_input: Option<&'a Path>,
    unit_values_segment_input: Option<&'a Path>,
    proof_values_input: Option<&'a Path>,
    program_image_cache: Option<&'a ProgramImageCommitmentCache>,
    output: &'a ProveWitnessTraceCommitments,
}

fn save_witness_outputs(
    request: &WitnessOutputSaveRequest<'_>,
    segment: &ProofSegment,
) -> Result<(), String> {
    fs::create_dir_all(request.output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            request.output_dir.display()
        )
    })?;

    let commitments = request.output.commitments();

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
    Ok(())
}

fn write_witness_output_summary(stdout: &mut dyn Write, commitments: &ProveWitnessCommitments) {
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
}

fn finish_all_units_witness_run(
    catalog: &KeyDirectoryCatalog,
    plan: &ProveExecutionPlan,
    parsed: &ParsedWitnessArgs,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    outputs: &[ProveWitnessTraceCommitments],
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let unit_values = load_batch_unit_values_inputs(
        &plan.run_plan.schedule,
        parsed.unit_values_segment.as_deref(),
        &auxiliary_inputs.unit_values,
    )?;
    let proof_request = WitnessAllUnitsProofRequest {
        catalog,
        schedule: &plan.run_plan.schedule,
        execution_units: &plan.units,
        gpu_streams: plan.run_plan.gpu.max_streams,
        public_inputs: plan.inputs.public_inputs.as_deref(),
        outputs,
        auxiliary_inputs,
        unit_values: &unit_values,
        evaluation_values_segment_input: parsed.evaluation_values_segment.as_deref(),
        verify_outputs: plan.run_plan.options.verify_outputs,
        program_image_cache: plan
            .program_image_cache
            .as_ref()
            .map(|summary| &summary.cache),
    };
    let proof_bytes = build_witness_proof_artifact_for_all_units(&proof_request)?;
    if plan.run_plan.options.save_outputs {
        for output in outputs {
            let commitments = output.commitments();
            let segment = build_witness_commitment_segment(commitments)
                .map_err(|error| format!("build witness segment failed: {error}"))?;
            let output_unit_index = commitments.unit_index();
            let execution_unit = plan
                .units
                .get(output_unit_index)
                .ok_or_else(|| format!("output unit index out of range: {output_unit_index}"))?;
            let request = WitnessOutputSaveRequest {
                output_dir: &plan.run_plan.options.output_dir,
                catalog,
                schedule: &plan.run_plan.schedule,
                execution_unit,
                gpu_streams: plan.run_plan.gpu.max_streams,
                public_inputs: plan.inputs.public_inputs.as_deref(),
                unit_values_input: parsed.unit_values.as_deref(),
                unit_values_segment_input: parsed.unit_values_segment.as_deref(),
                proof_values_input: parsed.proof_values.as_deref(),
                program_image_cache: plan
                    .program_image_cache
                    .as_ref()
                    .map(|summary| &summary.cache),
                output,
            };
            save_witness_outputs(&request, &segment)?;
        }
    }
    if let Some(proof_bytes) = proof_bytes {
        write_proof_output(&plan.run_plan.options.output_dir, &proof_bytes)?;
    }

    write_run_plan_summary(stdout, &plan.run_plan);
    if let Some(summary) = &plan.program_image_cache {
        write_program_image_cache_summary(stdout, summary);
    }
    for output in outputs {
        write_witness_output_summary(stdout, output.commitments());
    }
    Ok(())
}

pub fn build_witness_proof_core_artifact(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
) -> Result<ProofArtifact, String> {
    build_witness_proof_core_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        &[],
    )
}

fn build_witness_proof_core_artifact_with_bindings(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    binding_segments: &[ProofSegment],
) -> Result<ProofArtifact, String> {
    let material_segment = build_pcs_material_manifest_segment(schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let mut witness_segments = Vec::with_capacity(witness_outputs.len());
    for output in witness_outputs {
        witness_segments.push(
            build_witness_commitment_segment(output)
                .map_err(|error| format!("build witness segment failed: {error}"))?,
        );
    }
    witness_segments.sort_by_key(|segment| segment.id);

    let query_segment = build_pcs_query_plan_segment_with_bindings(
        schedule,
        public_values_hash,
        &material_segment,
        &witness_segments,
        binding_segments,
    )
    .map_err(|error| format!("build query plan segment failed: {error}"))?;
    let constant_opening_segment =
        build_constant_opening_segment(catalog, schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment_batch(schedule, &query_segment, witness_outputs)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;

    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
    ];
    segments.extend(witness_segments);

    Ok(ProofArtifact {
        setup_hash: schedule.setup_hash,
        public_values_hash,
        segments,
    })
}

pub fn build_witness_proof_artifact(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    proof_values: &[Felt],
    group_values: &[Ext3],
    unit_values: &[ProveUnitValues],
) -> Result<ProofArtifact, String> {
    build_witness_proof_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        ProofArtifactAuxInputs {
            proof_values,
            group_values,
            unit_values,
            binding_segments: &[],
        },
    )
}

fn build_witness_proof_artifact_with_bindings(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessCommitments],
    contents: ProofArtifactAuxInputs<'_>,
) -> Result<ProofArtifact, String> {
    let mut proof = build_witness_proof_core_artifact_with_bindings(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        contents.binding_segments,
    )?;
    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &catalog.layout.global_info,
        contents.proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    if let Some(segment) = proof_values_segment {
        proof.segments.push(segment);
    }
    let group_values_segment =
        build_group_values_segment(&catalog.layout.global_info, contents.group_values)
            .map_err(|error| format!("build group values segment failed: {error}"))?;
    if let Some(segment) = group_values_segment {
        proof.segments.push(segment);
    }
    let unit_values_segment =
        build_unit_values_segment_from_packed_values_batch(contents.unit_values)
            .map_err(|error| format!("build unit values segment failed: {error}"))?;
    if let Some(segment) = unit_values_segment {
        proof.segments.push(segment);
    }
    Ok(proof)
}

struct ProofArtifactAuxInputs<'a> {
    proof_values: &'a [Felt],
    group_values: &'a [Ext3],
    unit_values: &'a [ProveUnitValues],
    binding_segments: &'a [ProofSegment],
}

fn build_program_image_cache_proof_segment(
    cache: Option<&ProgramImageCommitmentCache>,
) -> Result<Option<ProofSegment>, String> {
    let Some(cache) = cache else {
        return Ok(None);
    };
    let data = encode_program_image_cache_segment(cache)
        .map_err(|error| format!("build program image cache segment failed: {error}"))?;
    Ok(Some(ProofSegment {
        id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
        data,
    }))
}

fn append_program_image_cache_segment(
    segments: &mut Vec<ProofSegment>,
    cache_segment: Option<ProofSegment>,
) {
    if let Some(segment) = cache_segment {
        segments.push(segment);
    }
}

struct WitnessAllUnitsProofRequest<'a> {
    catalog: &'a KeyDirectoryCatalog,
    schedule: &'a ProveSchedule,
    execution_units: &'a [ProveExecutionUnitArtifacts],
    gpu_streams: usize,
    public_inputs: Option<&'a Path>,
    outputs: &'a [ProveWitnessTraceCommitments],
    auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    unit_values: &'a [ProveUnitValues],
    evaluation_values_segment_input: Option<&'a Path>,
    verify_outputs: bool,
    program_image_cache: Option<&'a ProgramImageCommitmentCache>,
}

fn build_witness_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
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
    let cache_segment = build_program_image_cache_proof_segment(request.program_image_cache)?;
    let witness_outputs = request
        .outputs
        .iter()
        .map(|output| output.commitments())
        .collect::<Vec<_>>();
    let binding_segments: &[ProofSegment] = cache_segment.as_slice();
    let proof = if all_units_transcript_required(
        request.execution_units,
        request.outputs,
        request.auxiliary_inputs,
        request.evaluation_values_segment_input.is_some(),
    )? {
        build_witness_transcript_proof_artifact_for_all_units(
            request,
            public_values_hash,
            binding_segments,
        )?
    } else if binding_segments.is_empty() {
        build_witness_proof_artifact(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            &request.auxiliary_inputs.proof_values,
            &request.auxiliary_inputs.group_values,
            request.unit_values,
        )?
    } else {
        build_witness_proof_artifact_with_bindings(
            request.catalog,
            request.schedule,
            public_values_hash,
            &witness_outputs,
            ProofArtifactAuxInputs {
                proof_values: &request.auxiliary_inputs.proof_values,
                group_values: &request.auxiliary_inputs.group_values,
                unit_values: request.unit_values,
                binding_segments,
            },
        )?
    };
    let mut proof = proof;
    append_program_image_cache_segment(&mut proof.segments, cache_segment);
    if request.verify_outputs {
        validate_setup_preflight(request.catalog, &proof, &public_values)
            .map_err(|error| format!("verify proof output failed: {error}"))?;
    }
    encode_proof_artifact(&proof)
        .map(Some)
        .map_err(|error| format!("encode witness proof artifact failed: {error}"))
}

fn all_units_transcript_required(
    execution_units: &[ProveExecutionUnitArtifacts],
    outputs: &[ProveWitnessTraceCommitments],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    has_evaluation_segment: bool,
) -> Result<bool, String> {
    let mut has_fri_unit = false;
    for output in outputs {
        let unit_index = output.commitments().unit_index();
        let execution_unit = execution_units
            .get(unit_index)
            .ok_or_else(|| format!("output unit index out of range: {unit_index}"))?;
        if execution_unit.fri_expression_id.is_some() {
            has_fri_unit = true;
            if auxiliary_inputs.evaluations.is_empty() && !has_evaluation_segment {
                return Err(format!(
                    "missing evaluation values for unit {unit_index}: expected {}",
                    execution_unit.expected_evaluation_value_count()
                ));
            }
        }
    }
    Ok(has_fri_unit || !auxiliary_inputs.evaluations.is_empty() || has_evaluation_segment)
}

fn build_witness_transcript_proof_artifact_for_all_units(
    request: &WitnessAllUnitsProofRequest<'_>,
    public_values_hash: [u8; 32],
    binding_segments: &[ProofSegment],
) -> Result<ProofArtifact, String> {
    let material_segment = build_pcs_material_manifest_segment(request.schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let witness_outputs = request
        .outputs
        .iter()
        .map(|output| output.commitments())
        .collect::<Vec<_>>();
    let mut witness_segments = Vec::with_capacity(witness_outputs.len());
    for output in &witness_outputs {
        witness_segments.push(
            build_witness_commitment_segment(output)
                .map_err(|error| format!("build witness segment failed: {error}"))?,
        );
    }
    witness_segments.sort_by_key(|segment| segment.id);

    let evaluation_segment = match request.evaluation_values_segment_input {
        Some(path) => read_evaluation_values_segment_input(path)?,
        None => {
            let evaluation_values = request
                .outputs
                .iter()
                .map(|output| ProvePcsEvaluationValues {
                    unit_index: output.commitments().unit_index(),
                    values: request.auxiliary_inputs.evaluations.clone(),
                })
                .collect::<Vec<_>>();
            build_pcs_evaluation_segment(request.schedule, &evaluation_values)
                .map_err(|error| format!("build evaluation segment failed: {error}"))?
        }
    };
    let transcript_auxiliary_inputs = request
        .outputs
        .iter()
        .map(|output| {
            let unit_index = output.commitments().unit_index();
            let mut inputs = output.auxiliary_inputs().clone();
            if let Some(values) = request
                .unit_values
                .iter()
                .find(|values| values.unit_index == unit_index)
            {
                inputs.unit_values = values.packed_values.clone();
            }
            inputs
        })
        .collect::<Vec<_>>();
    let transcript_inputs = request
        .outputs
        .iter()
        .zip(transcript_auxiliary_inputs.iter())
        .map(|(output, auxiliary_inputs)| {
            let commitments = output.commitments();
            let unit_index = commitments.unit_index();
            let execution_unit = request
                .execution_units
                .get(unit_index)
                .ok_or_else(|| format!("output unit index out of range: {unit_index}"))?;
            let unit_index_u32 = u32::try_from(unit_index).map_err(|_| {
                format!("witness segment unit index does not fit u32: {unit_index}")
            })?;
            let expected_segment_id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
                .checked_add(unit_index_u32)
                .ok_or_else(|| format!("witness segment unit index overflow: {unit_index}"))?;
            let witness_segment = witness_segments
                .iter()
                .find(|segment| segment.id == expected_segment_id)
                .ok_or_else(|| format!("missing witness segment for unit {unit_index}"))?;
            Ok(ProvePcsFriTranscriptTraceSegmentValues {
                unit_index,
                execution_unit,
                trace: output.trace(),
                publics: output.publics(),
                auxiliary_inputs,
                material_segment: &material_segment,
                witness_segment,
                evaluation_segment: &evaluation_segment,
                binding_segments,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let transcript_values =
        build_pcs_fri_transcript_values_from_trace_segments(request.schedule, &transcript_inputs)
            .map_err(|error| format!("build FRI transcript values failed: {error}"))?;
    let final_query_challenge = transcript_values
        .first()
        .ok_or_else(|| "build FRI transcript values failed: no units".to_owned())?
        .commitments
        .final_query_challenge;
    let nonce_segment = build_pcs_query_nonce_segment_with_streams(
        request.schedule,
        final_query_challenge,
        request.gpu_streams,
    )
    .map_err(|error| format!("build query nonce segment failed: {error}"))?;
    let nonce = Felt::from_u64(
        parse_pcs_query_nonce_segment(&nonce_segment.data)
            .map_err(|error| format!("parse query nonce segment failed: {error}"))?
            .nonce,
    );
    let query_segment = build_pcs_query_plan_segment_from_challenge(
        request.schedule,
        &witness_segments,
        final_query_challenge,
        nonce,
    )
    .map_err(|error| format!("build query plan segment failed: {error}"))?;
    let constant_opening_segment =
        build_constant_opening_segment(request.catalog, request.schedule, &query_segment)
            .map_err(|error| format!("build constant opening segment failed: {error}"))?;
    let opening_segment =
        build_witness_opening_segment_batch(request.schedule, &query_segment, &witness_outputs)
            .map_err(|error| format!("build witness opening segment failed: {error}"))?;
    let fri_segment = build_pcs_fri_opening_segment_from_transcript_values(
        request.schedule,
        &query_segment,
        &transcript_values,
    )
    .map_err(|error| format!("build FRI opening segment failed: {error}"))?;

    let proof_values_segment = build_pcs_proof_values_segment_from_packed_values(
        &request.catalog.layout.global_info,
        &request.auxiliary_inputs.proof_values,
    )
    .map_err(|error| format!("build proof values segment failed: {error}"))?;
    let group_values_segment = build_group_values_segment(
        &request.catalog.layout.global_info,
        &request.auxiliary_inputs.group_values,
    )
    .map_err(|error| format!("build group values segment failed: {error}"))?;
    let unit_values_segment =
        build_unit_values_segment_from_packed_values_batch(request.unit_values)
            .map_err(|error| format!("build unit values segment failed: {error}"))?;

    let mut segments = vec![
        material_segment,
        query_segment,
        constant_opening_segment,
        opening_segment,
    ];
    segments.extend(witness_segments);
    segments.push(evaluation_segment);
    segments.push(fri_segment);
    segments.push(nonce_segment);
    if let Some(segment) = proof_values_segment {
        segments.push(segment);
    }
    if let Some(segment) = group_values_segment {
        segments.push(segment);
    }
    if let Some(segment) = unit_values_segment {
        segments.push(segment);
    }

    Ok(ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    })
}

fn read_evaluation_values_segment_input(path: &Path) -> Result<ProofSegment, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read evaluation values segment failed: {}: {error}",
            path.display()
        )
    })?;
    parse_pcs_evaluation_segment(&bytes)
        .map_err(|error| format!("parse evaluation values segment failed: {error}"))?;
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: bytes,
    })
}

fn read_challenge_values_segment_input(path: &Path) -> Result<Vec<Ext3>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read challenge values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = parse_challenge_values_segment(&bytes)
        .map_err(|error| format!("parse challenge values segment failed: {error}"))?;
    segment
        .values
        .into_iter()
        .enumerate()
        .map(|(index, words)| {
            Ok(Ext3::new(
                Felt::from_canonical(words[0]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 0 is invalid: {error}",
                        path.display()
                    )
                })?,
                Felt::from_canonical(words[1]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 1 is invalid: {error}",
                        path.display()
                    )
                })?,
                Felt::from_canonical(words[2]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 2 is invalid: {error}",
                        path.display()
                    )
                })?,
            ))
        })
        .collect()
}

fn read_packed_proof_values_segment(
    global_info: &GlobalInfo,
    path: &Path,
) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read proof values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: bytes,
    };
    let values = load_pcs_proof_values_from_segments(global_info, std::slice::from_ref(&segment))
        .map_err(|error| format!("load proof values segment failed: {error}"))?;
    flatten_pcs_proof_values(global_info, &values)
        .map_err(|error| format!("flatten proof values segment failed: {error}"))
}

fn read_group_values_segment_input(
    global_info: &GlobalInfo,
    path: &Path,
) -> Result<Vec<Ext3>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read group values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = ProofSegment {
        id: GROUP_VALUES_SEGMENT_ID,
        data: bytes,
    };
    load_group_values_from_segments(global_info, std::slice::from_ref(&segment))
        .map_err(|error| format!("load group values segment failed: {error}"))
}

fn load_batch_unit_values_inputs(
    schedule: &ProveSchedule,
    unit_values_segment_input: Option<&Path>,
    shared_unit_values: &[Felt],
) -> Result<Vec<ProveUnitValues>, String> {
    if let Some(path) = unit_values_segment_input {
        let bytes = fs::read(path).map_err(|error| {
            format!(
                "read unit values segment failed: {}: {error}",
                path.display()
            )
        })?;
        let parsed = parse_unit_values_segment(&bytes)
            .map_err(|error| format!("parse unit values segment failed: {error}"))?;
        let mut values = Vec::with_capacity(parsed.units.len());
        for unit in parsed.units {
            let unit_index = usize::try_from(unit.unit_index).map_err(|_| {
                format!(
                    "unit values segment unit index does not fit usize: {}",
                    unit.unit_index
                )
            })?;
            let schedule_unit = schedule.units.get(unit_index).ok_or_else(|| {
                format!("unit values segment unit index out of range: {unit_index}")
            })?;
            let packed_values = unit
                .values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    Felt::from_canonical(value).map_err(|error| {
                        format!(
                            "unit values segment unit {unit_index} field word {index} is invalid: {error}"
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            values.push(ProveUnitValues {
                unit_index,
                unit_value_map: schedule_unit.unit_value_map.clone(),
                packed_values,
            });
        }
        return Ok(values);
    }

    Ok(schedule
        .units
        .iter()
        .enumerate()
        .map(|(unit_index, unit)| ProveUnitValues {
            unit_index,
            unit_value_map: unit.unit_value_map.clone(),
            packed_values: shared_unit_values.to_vec(),
        })
        .collect())
}

fn run_prove_witness_commitments_for_all_units(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    backend: &(impl WitnessBackend + ?Sized),
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    let mut outputs = Vec::with_capacity(plan.units.len());
    for unit_index in 0..plan.units.len() {
        let output = run_prove_witness_commitments_with_trace_backend(
            plan,
            unit_index,
            auxiliary_inputs.clone(),
            backend,
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    Ok(outputs)
}

fn run_prove_witness_commitments_for_all_units_with_trace_bundle(
    plan: &ProveExecutionPlan,
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    bundle: &TraceBundle,
) -> Result<Vec<ProveWitnessTraceCommitments>, String> {
    let mut outputs = Vec::with_capacity(plan.units.len());
    for unit_index in 0..plan.units.len() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| format!("trace bundle unit index is too large: {unit_index}"))?;
        let trace_bytes = bundle
            .trace_bytes_for_unit(unit_index_u32)
            .ok_or_else(|| format!("trace bundle is missing unit {unit_index}"))?;
        let backend = TraceBytesBackend::new(trace_bytes.to_vec());
        let output = run_prove_witness_commitments_with_trace_backend(
            plan,
            unit_index,
            auxiliary_inputs.clone(),
            &backend,
        )
        .map_err(|error| {
            format!("run witness commitments failed for unit {unit_index}: {error}")
        })?;
        outputs.push(output);
    }
    Ok(outputs)
}

fn build_proof_bytes(
    request: &WitnessOutputSaveRequest<'_>,
    segment: &ProofSegment,
    verify_outputs: bool,
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
    let cache_segment = build_program_image_cache_proof_segment(request.program_image_cache)?;
    let binding_segments: &[ProofSegment] = cache_segment.as_slice();
    let material_segment = build_pcs_material_manifest_segment(request.schedule)
        .map_err(|error| format!("build material manifest segment failed: {error}"))?;
    let commitments = request.output.commitments();
    let transcript_values = if request.output.auxiliary_inputs().evaluations.is_empty() {
        if request.execution_unit.fri_expression_id.is_some() {
            return Err(format!(
                "missing evaluation values for unit {}: expected {}",
                commitments.unit_index(),
                request.execution_unit.expected_evaluation_value_count()
            ));
        }
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
                binding_segments,
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
            let nonce_segment = build_pcs_query_nonce_segment_with_streams(
                request.schedule,
                final_query_challenge,
                request.gpu_streams,
            )
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
            let query_segment = match cache_segment.as_ref() {
                Some(cache_segment) => build_pcs_query_plan_segment_with_bindings(
                    request.schedule,
                    public_values_hash,
                    &material_segment,
                    std::slice::from_ref(segment),
                    std::slice::from_ref(cache_segment),
                )
                .map_err(|error| format!("build query plan segment failed: {error}"))?,
                None => build_pcs_query_plan_segment(
                    request.schedule,
                    public_values_hash,
                    &material_segment,
                    std::slice::from_ref(segment),
                )
                .map_err(|error| format!("build query plan segment failed: {error}"))?,
            };
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
    let unit_index = commitments.unit_index();
    let unit = request
        .schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let packed_unit_values = match request.unit_values_segment_input {
        Some(path) => read_packed_unit_values_segment_for_unit(request.schedule, unit_index, path)?,
        None => match request.unit_values_input {
            Some(path) => read_packed_values(path, "unit values")?,
            None => Vec::new(),
        },
    };
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
    let group_values_segment = build_group_values_segment(
        &request.catalog.layout.global_info,
        &request.output.auxiliary_inputs().group_values,
    )
    .map_err(|error| format!("build group values segment failed: {error}"))?;
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
    if let Some(group_values_segment) = group_values_segment {
        segments.push(group_values_segment);
    }
    if let Some(unit_values_segment) = unit_values_segment {
        segments.push(unit_values_segment);
    }
    append_program_image_cache_segment(&mut segments, cache_segment);
    let proof = ProofArtifact {
        setup_hash: request.schedule.setup_hash,
        public_values_hash,
        segments,
    };
    if verify_outputs {
        validate_setup_preflight(request.catalog, &proof, &public_values)
            .map_err(|error| format!("verify proof output failed: {error}"))?;
    }
    encode_proof_artifact(&proof)
        .map(Some)
        .map_err(|error| format!("encode witness proof artifact failed: {error}"))
}

fn read_packed_unit_values_segment_for_unit(
    schedule: &ProveSchedule,
    unit_index: usize,
    path: &Path,
) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read unit values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let unit = schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let segment = ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: bytes,
    };
    load_unit_values_from_segments(
        unit_index,
        &unit.unit_value_map,
        std::slice::from_ref(&segment),
    )
    .map_err(|error| format!("load unit values segment failed: {error}"))
}

fn write_proof_output(output_dir: &Path, proof_bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            output_dir.display()
        )
    })?;
    write_output_file(&output_dir.join("proof.bin"), proof_bytes)
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
        "usage: lzvm prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>"
    );
    2
}
