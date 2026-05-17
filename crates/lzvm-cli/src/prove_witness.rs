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
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::read_public_values_file;
use lzvm_artifacts::trace_bundle::read_trace_bundle_file;
use lzvm_artifacts::unit_values_segment::{parse_unit_values_segment, UNIT_VALUES_SEGMENT_ID};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::group_values::load_group_values_from_segments;
use lzvm_prover::proof_values::{flatten_pcs_proof_values, load_pcs_proof_values_from_segments};
use lzvm_prover::unit_values::{load_unit_values_from_segments, ProveUnitValues};
use lzvm_prover::witness_loader::{load_witness_library, TraceBytesBackend, WitnessBackend};
use lzvm_prover::{
    build_witness_commitment_segment, derive_prove_execution_plan_with_program_image_cache,
    run_prove_witness_commitments_for_all_units,
    run_prove_witness_commitments_for_all_units_with_trace_bundle,
    run_prove_witness_commitments_with_trace_backend, ProveExecutionInputArtifacts,
    ProveExecutionPlan, ProveExecutionUnitArtifacts, ProveSchedule, ProveWitnessAuxiliaryInputs,
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
        unit_values_segment_input: parsed.unit_values_segment.as_deref(),
        program_image_cache: plan
            .program_image_cache
            .as_ref()
            .map(|summary| &summary.cache),
        output: &output,
    };
    let proof_bytes = match build_proof_bytes(&request, plan.run_plan.options.verify_outputs) {
        Ok(proof_bytes) => proof_bytes,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    if plan.run_plan.options.save_outputs {
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
    unit_values_segment_input: Option<&'a Path>,
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
    let public_values =
        match plan.inputs.public_inputs.as_deref() {
            Some(path) => Some(read_public_values_file(path).map_err(|error| {
                format!("read public inputs failed: {}: {error}", path.display())
            })?),
            None => None,
        };
    let evaluation_values_segment = match parsed.evaluation_values_segment.as_deref() {
        Some(path) => Some(read_evaluation_values_segment_input(path)?),
        None => None,
    };
    let proof = lzvm_prover::build_witness_proof_artifact_for_all_units(
        &lzvm_prover::WitnessAllUnitsProofRequest {
            catalog,
            schedule: &plan.run_plan.schedule,
            execution_units: &plan.units,
            gpu_streams: plan.run_plan.gpu.max_streams,
            public_values: public_values.as_ref(),
            outputs,
            auxiliary_inputs,
            unit_values: &unit_values,
            evaluation_values_segment: evaluation_values_segment.as_ref(),
            verify_outputs: plan.run_plan.options.verify_outputs,
            program_image_cache: plan
                .program_image_cache
                .as_ref()
                .map(|summary| &summary.cache),
        },
    )?;
    let proof_bytes = match proof {
        Some(proof) => Some(
            encode_proof_artifact(&proof)
                .map_err(|error| format!("encode witness proof artifact failed: {error}"))?,
        ),
        None => None,
    };
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
                unit_values_segment_input: parsed.unit_values_segment.as_deref(),
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
    lzvm_prover::build_witness_proof_core_artifact(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
    )
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
    lzvm_prover::build_witness_proof_artifact(
        catalog,
        schedule,
        public_values_hash,
        witness_outputs,
        proof_values,
        group_values,
        unit_values,
    )
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

fn build_proof_bytes(
    request: &WitnessOutputSaveRequest<'_>,
    verify_outputs: bool,
) -> Result<Option<Vec<u8>>, String> {
    let Some(public_inputs) = request.public_inputs else {
        return Ok(None);
    };
    let public_values = read_public_values_file(public_inputs)
        .map_err(|error| format!("read public inputs failed: {error}"))?;
    let unit_index = request.output.commitments().unit_index();
    let unit_values = match request.unit_values_segment_input {
        Some(path) => Some(read_packed_unit_values_segment_for_unit(
            request.schedule,
            unit_index,
            path,
        )?),
        None => None,
    };
    let proof =
        lzvm_prover::build_witness_proof_artifact_for_unit(&lzvm_prover::WitnessProofRequest {
            catalog: request.catalog,
            schedule: request.schedule,
            execution_unit: request.execution_unit,
            gpu_streams: request.gpu_streams,
            public_values: Some(&public_values),
            unit_values: unit_values.as_deref(),
            output: request.output,
            verify_outputs,
            program_image_cache: request.program_image_cache,
        })?;
    match proof {
        Some(proof) => encode_proof_artifact(&proof)
            .map(Some)
            .map_err(|error| format!("encode witness proof artifact failed: {error}")),
        None => Ok(None),
    }
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
