use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use lzvm_artifacts::constant_tree::ConstantTreeFileSummary;
use lzvm_artifacts::eth_block_input::EthBlockInput;
use lzvm_artifacts::eth_block_public_values::validate_eth_block_public_values_with_program_image_cache;
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{public_values_digest, read_public_values_file};
use lzvm_artifacts::trace_bundle::{parse_trace_bundle_ref, read_trace_bundle_file_bytes};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::guest_pc_trace_backend::{
    is_guest_pc_trace_layout_supported, is_guest_pc_trace_segmented_layout_supported,
    GuestPcTraceBackend,
};
use lzvm_prover::unit_values::ProveUnitValues;
use lzvm_prover::witness_layout::derive_witness_trace_layout;
use lzvm_prover::witness_loader::load_witness_library;
use lzvm_prover::{
    build_witness_commitment_segment_for_schedule,
    derive_prove_execution_plan_with_program_image_cache,
    run_prove_witness_commitments_for_all_units,
    run_prove_witness_commitments_for_all_units_with_trace_bundle,
    run_prove_witness_commitments_with_trace_backend,
    run_prove_witness_commitments_with_trace_bytes, ProveExecutionPlan,
    ProveExecutionUnitArtifacts, ProvePassKind, ProvePassRequest, ProveRunRequest, ProveSchedule,
    ProveWitnessAuxiliaryInputs, ProveWitnessCommitments, ProveWitnessTraceCommitments,
};

use crate::eth_block_prove_input::{write_eth_block_input_summary, EthBlockInputSummary};
use crate::program_image_cache::write_program_image_cache_summary;
use crate::prove_plan::{
    prepare_requested_gpu_setup, read_prove_setup_catalog, set_default_input_data,
    validate_all_unit_stored_witness_limit, write_run_plan_summary, write_source_companion_summary,
    ParseError,
};
use crate::trace_input_shape::validate_trace_input_shapes;

mod args;
mod eth_inputs;
mod guest_pc_trace;
mod output_file;
mod proof_timing;
mod timing;
mod usage;
mod value_inputs;

use args::{parse_witness_args, ParsedWitnessArgs};
use eth_inputs::{
    prepare_eth_block_input, prepare_eth_block_public_inputs, public_values_field_count,
    summarize_public_inputs,
};
use guest_pc_trace::{record_guest_pc_trace_timing, run_guest_pc_trace_witness};
use output_file::{write_output_file, write_proof_output};
use proof_timing::record_proof_artifact_timing;
use timing::{write_timing_summary_with_allocator, TimingRecorder};
use usage::write_usage;
use value_inputs::{
    load_batch_unit_values_inputs, read_challenge_values_proof_segment_input,
    read_challenge_values_segment_input, read_evaluation_values_segment_input,
    read_group_values_segment_input, read_packed_extension_values,
    read_packed_proof_values_segment, read_packed_unit_values_segment_for_unit, read_packed_values,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut parsed = match parse_witness_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let mut timings = TimingRecorder::new(parsed.timings);

    let catalog = match read_prove_setup_catalog(&parsed.run_args.positionals[0]) {
        Ok(catalog) => catalog,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    timings.mark("catalog");

    if let Err(message) = validate_guest_pc_trace_eth_input_binding(&parsed) {
        let _ = writeln!(stderr, "prove witness failed: {message}");
        return 1;
    }

    let prepared_eth_block_input = match prepare_eth_block_input(&parsed) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    timings.mark("eth_input");
    if let Some(summary) = &prepared_eth_block_input.summary {
        set_default_input_data(&mut parsed.run_args.request, &summary.path);
    }
    let prepared_public_inputs = match prepare_eth_block_public_inputs(
        &parsed,
        &catalog,
        prepared_eth_block_input.summary.as_ref(),
    ) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let generated_public_inputs = prepared_public_inputs.generated;
    timings.mark("public_inputs");
    let plan = match derive_prove_execution_plan_with_program_image_cache(
        &catalog,
        parsed.run_args.request.clone(),
        prepared_public_inputs.inputs,
        parsed.run_args.program_image_cache.clone(),
    ) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    timings.mark("plan");
    if let Err(error) = prepare_requested_gpu_setup(&plan) {
        let _ = writeln!(stderr, "prove witness failed: {error}");
        return 1;
    }
    if parsed.all_units || plan.run_plan.options.aggregate {
        if let Err(error) = validate_all_unit_stored_witness_limit(
            plan.run_plan.gpu.max_stored_witnesses,
            plan.units.len(),
        ) {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    }
    timings.mark("gpu_setup");
    if let Err(message) = validate_large_guest_pc_gpu(parsed.guest_pc_trace_instruction_limit) {
        let _ = writeln!(stderr, "prove witness failed: {message}");
        return 1;
    }
    let single_unit_index = match selected_single_unit_index(&plan, &parsed) {
        Ok(unit_index) => unit_index,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let public_inputs_summary = match summarize_public_inputs(plan.inputs.public_inputs.as_deref())
    {
        Ok(summary) => summary,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    let contribution_only = contribution_artifact_requested(&plan);
    let mut constant_tree_material_validation = start_constant_tree_material_validation(
        &catalog,
        &plan.run_plan.schedule,
        !contribution_only && plan.inputs.public_inputs.is_some(),
    );
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
    if parsed.guest_pc_trace_instruction_limit.is_some()
        && (parsed.all_units || plan.run_plan.options.aggregate)
    {
        let _ = writeln!(
            stderr,
            "prove witness failed: --guest-pc-trace requires a single-unit witness run"
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
    timings.mark("auxiliary_inputs");
    let trace_bundle_bytes = match &parsed.trace_bundle {
        Some(path) => match read_trace_bundle_file_bytes(path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        },
        None => None,
    };
    let trace_bundle = match trace_bundle_bytes.as_deref() {
        Some(bytes) => match parse_trace_bundle_ref(bytes) {
            Ok(bundle) => Some(bundle),
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        },
        None => None,
    };
    if let Some(bundle) = &trace_bundle {
        if let Err(message) = validate_trace_input_shapes(
            None,
            Some(bundle),
            parsed.all_units || plan.run_plan.options.aggregate,
            single_unit_index,
            &plan.run_plan.schedule,
        ) {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    }
    let challenge_values_segment = match parsed.challenge_values_segment.as_deref() {
        Some(path) => match read_challenge_values_proof_segment_input(path) {
            Ok(segment) => Some(segment),
            Err(message) => {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
        },
        None => None,
    };
    timings.mark("trace_inputs");
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
            timings.mark("witness");
            let constant_tree_material_summaries = match join_constant_tree_material_validation(
                &mut constant_tree_material_validation,
                &mut timings,
            ) {
                Ok(summaries) => summaries,
                Err(message) => {
                    let _ = writeln!(stderr, "prove witness failed: {message}");
                    return 1;
                }
            };
            timings.mark("constant_material_wait");
            if let Err(message) = finish_all_units_witness_run(
                FinishAllUnitsWitnessRunRequest {
                    catalog: &catalog,
                    plan: &plan,
                    constant_tree_material_summaries: constant_tree_material_summaries.as_deref(),
                    parsed: &parsed,
                    auxiliary_inputs: &auxiliary_inputs,
                    outputs: &outputs,
                    eth_block_input: FinishEthBlockInput {
                        summary: prepared_eth_block_input.summary.as_ref(),
                        generated_public_inputs,
                        generated_from_public_input: prepared_eth_block_input
                            .generated_from_public_input,
                        public_input: parsed.eth_public_input.as_deref(),
                    },
                    challenge_values_segment: challenge_values_segment.as_ref(),
                },
                stdout,
                &mut timings,
            ) {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
            timings.mark("finish");
            write_timing_summary_with_allocator(stdout, &mut timings);
            return 0;
        }
    }
    let output = if let Some(path) = &plan.inputs.witness_library {
        let witness_backend = match load_witness_library(path) {
            Ok(backend) => backend,
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        };
        if parsed.all_units || plan.run_plan.options.aggregate {
            let outputs = match run_prove_witness_commitments_for_all_units(
                &plan,
                &auxiliary_inputs,
                &witness_backend,
            ) {
                Ok(outputs) => outputs,
                Err(error) => {
                    let _ = writeln!(stderr, "prove witness failed: {error}");
                    return 1;
                }
            };
            timings.mark("witness");
            let constant_tree_material_summaries = match join_constant_tree_material_validation(
                &mut constant_tree_material_validation,
                &mut timings,
            ) {
                Ok(summaries) => summaries,
                Err(message) => {
                    let _ = writeln!(stderr, "prove witness failed: {message}");
                    return 1;
                }
            };
            timings.mark("constant_material_wait");
            if let Err(message) = finish_all_units_witness_run(
                FinishAllUnitsWitnessRunRequest {
                    catalog: &catalog,
                    plan: &plan,
                    constant_tree_material_summaries: constant_tree_material_summaries.as_deref(),
                    parsed: &parsed,
                    auxiliary_inputs: &auxiliary_inputs,
                    outputs: &outputs,
                    eth_block_input: FinishEthBlockInput {
                        summary: prepared_eth_block_input.summary.as_ref(),
                        generated_public_inputs,
                        generated_from_public_input: prepared_eth_block_input
                            .generated_from_public_input,
                        public_input: parsed.eth_public_input.as_deref(),
                    },
                    challenge_values_segment: challenge_values_segment.as_ref(),
                },
                stdout,
                &mut timings,
            ) {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
            timings.mark("finish");
            write_timing_summary_with_allocator(stdout, &mut timings);
            return 0;
        }
        match run_prove_witness_commitments_with_trace_backend(
            &plan,
            single_unit_index,
            auxiliary_inputs,
            &witness_backend,
        ) {
            Ok(output) => output,
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        }
    } else if let Some(path) = &parsed.trace_bytes {
        let trace_bytes = match fs::read(path) {
            Ok(trace_bytes) => trace_bytes,
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: read trace bytes failed: {}: {error}",
                    path.display()
                );
                return 1;
            }
        };
        match run_prove_witness_commitments_with_trace_bytes(
            &plan,
            single_unit_index,
            auxiliary_inputs,
            &trace_bytes,
        ) {
            Ok(output) => output,
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        }
    } else if let Some(instruction_limit) = parsed.guest_pc_trace_instruction_limit {
        let unit = match plan.run_plan.schedule.units.get(single_unit_index) {
            Some(unit) => unit,
            None => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: unit index out of range: {single_unit_index}"
                );
                return 1;
            }
        };
        let layout = match derive_witness_trace_layout(unit) {
            Ok(layout) => layout,
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: guest PC trace unit layout failed for unit {single_unit_index}: {error}"
                );
                return 1;
            }
        };
        if is_guest_pc_trace_segmented_layout_supported(&layout) {
            let finish_auxiliary_inputs = auxiliary_inputs.clone();
            let trace_can_be_dropped = parsed.evaluation_values_segment.is_none()
                && auxiliary_inputs.evaluations.is_empty()
                && plan
                    .units
                    .get(single_unit_index)
                    .is_some_and(|unit| unit.fri_expression_id.is_none());
            let guest_pc_trace_run = match run_guest_pc_trace_witness(
                &plan,
                single_unit_index,
                auxiliary_inputs,
                instruction_limit,
                trace_can_be_dropped,
                parsed.timings,
            ) {
                Ok(run) => run,
                Err(error) => {
                    let _ = writeln!(stderr, "prove witness failed: {error}");
                    return 1;
                }
            };
            let mut outputs = guest_pc_trace_run.outputs;
            if let Some(timing) = guest_pc_trace_run.timing {
                record_guest_pc_trace_timing(&mut timings, timing);
            }
            if outputs.len() > 1 {
                timings.mark("witness");
                let constant_tree_material_summaries = match join_constant_tree_material_validation(
                    &mut constant_tree_material_validation,
                    &mut timings,
                ) {
                    Ok(summaries) => summaries,
                    Err(message) => {
                        let _ = writeln!(stderr, "prove witness failed: {message}");
                        return 1;
                    }
                };
                timings.mark("constant_material_wait");
                if let Err(message) = finish_all_units_witness_run(
                    FinishAllUnitsWitnessRunRequest {
                        catalog: &catalog,
                        plan: &plan,
                        constant_tree_material_summaries: constant_tree_material_summaries
                            .as_deref(),
                        parsed: &parsed,
                        auxiliary_inputs: &finish_auxiliary_inputs,
                        outputs: &outputs,
                        eth_block_input: FinishEthBlockInput {
                            summary: prepared_eth_block_input.summary.as_ref(),
                            generated_public_inputs,
                            generated_from_public_input: prepared_eth_block_input
                                .generated_from_public_input,
                            public_input: parsed.eth_public_input.as_deref(),
                        },
                        challenge_values_segment: challenge_values_segment.as_ref(),
                    },
                    stdout,
                    &mut timings,
                ) {
                    let _ = writeln!(stderr, "prove witness failed: {message}");
                    return 1;
                }
                timings.mark("finish");
                write_timing_summary_with_allocator(stdout, &mut timings);
                return 0;
            }
            match outputs.pop() {
                Some(output) => output,
                None => {
                    let _ = writeln!(
                        stderr,
                        "prove witness failed: segmented guest PC trace produced no outputs"
                    );
                    return 1;
                }
            }
        } else {
            let backend = GuestPcTraceBackend::new(instruction_limit);
            match run_prove_witness_commitments_with_trace_backend(
                &plan,
                single_unit_index,
                auxiliary_inputs,
                &backend,
            ) {
                Ok(output) => output,
                Err(error) => {
                    let _ = writeln!(stderr, "prove witness failed: {error}");
                    return 1;
                }
            }
        }
    } else if let Some(bundle) = &trace_bundle {
        let selected_unit_u32 = match u32::try_from(single_unit_index) {
            Ok(value) => value,
            Err(_) => {
                let _ = writeln!(
                    stderr,
                    "prove witness failed: trace bundle unit index is too large: {single_unit_index}"
                );
                return 1;
            }
        };
        let Some(trace_bytes) = bundle.trace_bytes_for_unit(selected_unit_u32) else {
            let _ = writeln!(
                stderr,
                "prove witness failed: trace bundle is missing unit {single_unit_index}"
            );
            return 1;
        };
        match run_prove_witness_commitments_with_trace_bytes(
            &plan,
            single_unit_index,
            auxiliary_inputs,
            trace_bytes,
        ) {
            Ok(output) => output,
            Err(error) => {
                let _ = writeln!(stderr, "prove witness failed: {error}");
                return 1;
            }
        }
    } else {
        let _ = writeln!(
            stderr,
            "prove witness failed: witness library, trace bytes, or trace bundle are required"
        );
        return 1;
    };
    timings.mark("witness");
    let constant_tree_material_summaries = match join_constant_tree_material_validation(
        &mut constant_tree_material_validation,
        &mut timings,
    ) {
        Ok(summaries) => summaries,
        Err(message) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };
    timings.mark("constant_material_wait");
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
        constant_tree_material_summaries: constant_tree_material_summaries.as_deref(),
        execution_unit,
        gpu_streams: plan.run_plan.gpu.max_streams,
        public_inputs: plan.inputs.public_inputs.as_deref(),
        unit_values_segment_input: parsed.unit_values_segment.as_deref(),
        program_image_cache: plan
            .program_image_cache
            .as_ref()
            .map(|summary| &summary.cache),
        eth_block_input: prepared_eth_block_input
            .summary
            .as_ref()
            .map(|summary| &summary.input),
        challenge_values_segment: challenge_values_segment.as_ref(),
        output: &output,
        contribution_only,
        include_contribution_segment: false,
    };
    let proof_bytes =
        match build_proof_bytes(&request, plan.run_plan.options.verify_outputs, &mut timings) {
            Ok(proof_bytes) => proof_bytes,
            Err(message) => {
                let _ = writeln!(stderr, "prove witness failed: {message}");
                return 1;
            }
        };
    timings.mark("proof");
    if plan.run_plan.options.save_outputs {
        let segment = match build_witness_commitment_segment_for_schedule(
            plan.run_plan.schedule.units.len(),
            commitments,
        ) {
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
    timings.mark("output_write");

    write_run_plan_summary(stdout, &plan.run_plan);
    write_source_companion_summary(stdout, &catalog);
    if let Some(path) = &plan.inputs.public_inputs {
        let _ = writeln!(stdout, "public_inputs={}", path.display());
    }
    if let Some(summary) = &public_inputs_summary {
        let _ = writeln!(
            stdout,
            "public_inputs_hash={}",
            crate::prove_plan::format_hash(&summary.digest)
        );
        let _ = writeln!(stdout, "public_input_values={}", summary.value_count);
        let _ = writeln!(stdout, "public_input_fields={}", summary.field_count);
    }
    if generated_public_inputs {
        let _ = writeln!(stdout, "public_inputs_generated=eth_block_input");
    }
    if let Some(path) = &parsed.eth_public_input {
        let _ = writeln!(stdout, "eth_public_input={}", path.display());
    }
    if prepared_eth_block_input.generated_from_public_input {
        let _ = writeln!(stdout, "eth_block_input_generated=eth_public_input");
    }
    if let Some(summary) = &plan.program_image_cache {
        write_program_image_cache_summary(stdout, summary);
    }
    if let Some(summary) = &prepared_eth_block_input.summary {
        write_eth_block_input_summary(stdout, summary);
    }
    write_witness_output_summary(stdout, commitments);
    timings.mark("summary");
    write_timing_summary_with_allocator(stdout, &mut timings);
    0
}

struct ConstantTreeMaterialValidationJob {
    handle: JoinHandle<Result<Vec<Option<ConstantTreeFileSummary>>, String>>,
    started: Instant,
}

fn start_constant_tree_material_validation(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    enabled: bool,
) -> Option<ConstantTreeMaterialValidationJob> {
    if !enabled {
        return None;
    }
    let catalog = catalog.clone();
    let schedule = schedule.clone();
    Some(ConstantTreeMaterialValidationJob {
        started: Instant::now(),
        handle: thread::Builder::new()
            .name("lzvm-ct-val".to_owned())
            .spawn(move || {
                lzvm_prover::validate_constant_opening_materials(&catalog, &schedule)
                    .map_err(|error| error.to_string())
            })
            .expect("constant-tree material validation thread should spawn"),
    })
}

fn join_constant_tree_material_validation(
    job: &mut Option<ConstantTreeMaterialValidationJob>,
    timings: &mut TimingRecorder,
) -> Result<Option<Vec<Option<ConstantTreeFileSummary>>>, String> {
    let Some(job) = job.take() else {
        return Ok(None);
    };
    let started = job.started;
    let join_started = Instant::now();
    let summaries = job
        .handle
        .join()
        .map_err(|_| "constant-tree material validation thread panicked".to_owned())?;
    let summaries = summaries?;
    let join_wait = join_started.elapsed();
    record_constant_material_validation_timing(timings, started.elapsed(), join_wait, &summaries);
    Ok(Some(summaries))
}

fn record_constant_material_validation_timing(
    timings: &mut TimingRecorder,
    elapsed: Duration,
    join_wait: Duration,
    summaries: &[Option<ConstantTreeFileSummary>],
) {
    let mut byte_count = 0u64;
    let mut unit_count = 0usize;
    for summary in summaries.iter().flatten() {
        unit_count += 1;
        byte_count = byte_count.saturating_add(summary.byte_count);
    }
    timings.record("constant_material_validation_elapsed", elapsed);
    timings.record("constant_material_validation_join_wait", join_wait);
    timings.record_count("constant_material_validation_units", unit_count);
    timings.record_count(
        "constant_material_validation_bytes",
        saturating_usize_from_u64(byte_count),
    );
}

fn saturating_usize_from_u64(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn contribution_artifact_requested(plan: &ProveExecutionPlan) -> bool {
    plan.run_plan.pass.kind() == ProvePassKind::Contributions
        || plan.run_plan.options.remote_aggregation
}

fn partition_input_data(request: &ProveRunRequest) -> Option<&Path> {
    match &request.pass {
        ProvePassRequest::Contributions(partitions) | ProvePassRequest::Full(partitions) => {
            partitions.input_data.as_deref()
        }
        ProvePassRequest::Internal { .. } => None,
    }
}

fn validate_guest_pc_trace_eth_input_binding(parsed: &ParsedWitnessArgs) -> Result<(), String> {
    if parsed.guest_pc_trace_instruction_limit.is_some()
        && (parsed.eth_block_input.is_some() || parsed.eth_public_input.is_some())
        && partition_input_data(&parsed.run_args.request).is_none()
    {
        return Err("--eth-block-input/--eth-public-input with --guest-pc-trace requires --input-data with framed guest stdin".to_owned());
    }
    Ok(())
}

fn validate_large_guest_pc_gpu(instruction_limit: Option<u64>) -> Result<(), &'static str> {
    match (
        instruction_limit.unwrap_or(0) >= 1_000_000,
        lzvm_prover::gpu_setup_available(),
    ) {
        (true, false) => Err("large --guest-pc-trace runs require a CUDA-enabled lzvm-cli build"),
        _ => Ok(()),
    }
}

fn selected_single_unit_index(
    plan: &ProveExecutionPlan,
    parsed: &ParsedWitnessArgs,
) -> Result<usize, String> {
    if let Some(unit_index) = parsed.unit_index {
        return Ok(unit_index);
    }
    if parsed.guest_pc_trace_instruction_limit.is_some() {
        return selected_guest_pc_trace_unit_index(plan);
    }
    Ok(0)
}

fn selected_guest_pc_trace_unit_index(plan: &ProveExecutionPlan) -> Result<usize, String> {
    let mut fallback = None;
    for (unit_index, unit) in plan.run_plan.schedule.units.iter().enumerate() {
        let layout = derive_witness_trace_layout(unit).map_err(|error| {
            format!("guest PC trace unit layout failed for unit {unit_index}: {error}")
        })?;
        if is_guest_pc_trace_layout_supported(&layout) {
            if unit.unit_name.as_deref() == Some("Main") {
                return Ok(unit_index);
            }
            fallback.get_or_insert(unit_index);
        }
    }
    fallback.ok_or_else(|| {
        "no prove witness unit exposes guest PC trace columns; use a setup with a compatible guest trace layout"
            .to_owned()
    })
}

#[derive(Clone, Copy)]
struct FinishEthBlockInput<'a> {
    summary: Option<&'a EthBlockInputSummary>,
    generated_public_inputs: bool,
    generated_from_public_input: bool,
    public_input: Option<&'a Path>,
}

struct FinishAllUnitsWitnessRunRequest<'a> {
    catalog: &'a KeyDirectoryCatalog,
    plan: &'a ProveExecutionPlan,
    constant_tree_material_summaries: Option<&'a [Option<ConstantTreeFileSummary>]>,
    parsed: &'a ParsedWitnessArgs,
    auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    outputs: &'a [ProveWitnessTraceCommitments],
    eth_block_input: FinishEthBlockInput<'a>,
    challenge_values_segment: Option<&'a ProofSegment>,
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
    constant_tree_material_summaries: Option<&'a [Option<ConstantTreeFileSummary>]>,
    execution_unit: &'a ProveExecutionUnitArtifacts,
    gpu_streams: usize,
    public_inputs: Option<&'a Path>,
    unit_values_segment_input: Option<&'a Path>,
    program_image_cache: Option<&'a ProgramImageCommitmentCache>,
    eth_block_input: Option<&'a EthBlockInput>,
    challenge_values_segment: Option<&'a ProofSegment>,
    output: &'a ProveWitnessTraceCommitments,
    contribution_only: bool,
    include_contribution_segment: bool,
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
        let root_path = witness_stage_output_path(
            request.output_dir,
            commitments,
            commitment.stage_index(),
            "witness-root",
        );
        let tree_path = witness_stage_output_path(
            request.output_dir,
            commitments,
            commitment.stage_index(),
            "witness-tree",
        );
        let mut root_bytes = Vec::with_capacity(32);
        for value in commitment.root() {
            root_bytes.extend_from_slice(&value.to_le_bytes());
        }
        write_output_file(&root_path, &root_bytes)?;
        write_output_file(&tree_path, commitment.tree_bytes())?;
    }
    let segment_path = witness_segment_output_path(request.output_dir, commitments);
    write_output_file(&segment_path, &segment.data)?;
    Ok(())
}

fn witness_stage_output_path(
    output_dir: &Path,
    commitments: &ProveWitnessCommitments,
    stage_index: usize,
    suffix: &str,
) -> std::path::PathBuf {
    if commitments.trace_instance_index() == 0 {
        output_dir.join(format!(
            "unit-{}-stage-{}.{}",
            commitments.unit_index(),
            stage_index,
            suffix
        ))
    } else {
        output_dir.join(format!(
            "unit-{}-trace-{}-stage-{}.{}",
            commitments.unit_index(),
            commitments.trace_instance_index(),
            stage_index,
            suffix
        ))
    }
}

fn witness_segment_output_path(
    output_dir: &Path,
    commitments: &ProveWitnessCommitments,
) -> std::path::PathBuf {
    if commitments.trace_instance_index() == 0 {
        output_dir.join(format!("unit-{}.witness-segment", commitments.unit_index()))
    } else {
        output_dir.join(format!(
            "unit-{}-trace-{}.witness-segment",
            commitments.unit_index(),
            commitments.trace_instance_index()
        ))
    }
}

fn write_witness_output_summary(stdout: &mut dyn Write, commitments: &ProveWitnessCommitments) {
    write_witness_output_summary_with_trace(stdout, commitments, false);
}

fn write_witness_output_summary_with_trace(
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

fn finish_all_units_witness_run(
    request: FinishAllUnitsWitnessRunRequest<'_>,
    stdout: &mut dyn Write,
    timings: &mut TimingRecorder,
) -> Result<(), String> {
    let FinishAllUnitsWitnessRunRequest {
        catalog,
        plan,
        constant_tree_material_summaries,
        parsed,
        auxiliary_inputs,
        outputs,
        eth_block_input,
        challenge_values_segment,
    } = request;
    let unit_values = load_batch_unit_values_inputs(
        &plan.run_plan.schedule,
        outputs,
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
    if let (Some(summary), Some(public_values)) = (eth_block_input.summary, public_values.as_ref())
    {
        validate_eth_block_public_values_with_program_image_cache(
            &summary.input,
            public_values,
            plan.program_image_cache
                .as_ref()
                .map(|cache_summary| &cache_summary.cache),
        )
        .map_err(|error| error.to_string())?;
    }
    let evaluation_values_segment = match parsed.evaluation_values_segment.as_deref() {
        Some(path) => Some(read_evaluation_values_segment_input(path)?),
        None => None,
    };
    let proof_request = lzvm_prover::WitnessAllUnitsProofRequest {
        catalog,
        schedule: &plan.run_plan.schedule,
        constant_tree_material_summaries,
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
        eth_block_input: eth_block_input.summary.map(|summary| &summary.input),
        challenge_values_segment,
        include_contribution_segment: !contribution_artifact_requested(plan)
            && plan.run_plan.options.aggregate
            && challenge_values_segment.is_some(),
    };
    let mut proof_artifact_timing = lzvm_prover::WitnessProofArtifactTiming::default();
    let proof = if contribution_artifact_requested(plan) {
        lzvm_prover::build_witness_contribution_proof_artifact_for_all_units(&proof_request)?
    } else {
        lzvm_prover::build_witness_proof_artifact_for_all_units_with_timing(
            &proof_request,
            &mut proof_artifact_timing,
        )?
    };
    record_proof_artifact_timing(timings, &proof_artifact_timing);
    let proof_bytes = match proof {
        Some(proof) => {
            let started = Instant::now();
            let proof_bytes = encode_proof_artifact(&proof)
                .map_err(|error| format!("encode witness proof artifact failed: {error}"))?;
            timings.record("finish_proof_encode", started.elapsed());
            Some(proof_bytes)
        }
        None => None,
    };
    if plan.run_plan.options.save_outputs {
        for output in outputs {
            let commitments = output.commitments();
            let segment = build_witness_commitment_segment_for_schedule(
                plan.run_plan.schedule.units.len(),
                commitments,
            )
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
                constant_tree_material_summaries,
                execution_unit,
                gpu_streams: plan.run_plan.gpu.max_streams,
                public_inputs: plan.inputs.public_inputs.as_deref(),
                unit_values_segment_input: parsed.unit_values_segment.as_deref(),
                program_image_cache: plan
                    .program_image_cache
                    .as_ref()
                    .map(|summary| &summary.cache),
                eth_block_input: eth_block_input.summary.map(|summary| &summary.input),
                challenge_values_segment,
                output,
                contribution_only: contribution_artifact_requested(plan),
                include_contribution_segment: !contribution_artifact_requested(plan)
                    && plan.run_plan.options.aggregate
                    && challenge_values_segment.is_some(),
            };
            save_witness_outputs(&request, &segment)?;
        }
    }
    if let Some(proof_bytes) = proof_bytes {
        write_proof_output(&plan.run_plan.options.output_dir, &proof_bytes)?;
    }

    write_run_plan_summary(stdout, &plan.run_plan);
    write_source_companion_summary(stdout, catalog);
    if let Some(path) = &plan.inputs.public_inputs {
        let _ = writeln!(stdout, "public_inputs={}", path.display());
    }
    if let Some(public_values) = public_values.as_ref() {
        let digest = public_values_digest(public_values)
            .map_err(|error| format!("digest public inputs failed: {error}"))?;
        let _ = writeln!(
            stdout,
            "public_inputs_hash={}",
            crate::prove_plan::format_hash(&digest)
        );
        let _ = writeln!(stdout, "public_input_values={}", public_values.values.len());
        let _ = writeln!(
            stdout,
            "public_input_fields={}",
            public_values_field_count(public_values)
        );
    }
    if eth_block_input.generated_public_inputs {
        let _ = writeln!(stdout, "public_inputs_generated=eth_block_input");
    }
    if let Some(path) = eth_block_input.public_input {
        let _ = writeln!(stdout, "eth_public_input={}", path.display());
    }
    if eth_block_input.generated_from_public_input {
        let _ = writeln!(stdout, "eth_block_input_generated=eth_public_input");
    }
    if let Some(summary) = &plan.program_image_cache {
        write_program_image_cache_summary(stdout, summary);
    }
    if let Some(summary) = eth_block_input.summary {
        write_eth_block_input_summary(stdout, summary);
    }
    let include_trace_instance = outputs
        .iter()
        .any(|output| output.commitments().trace_instance_index() != 0);
    for output in outputs {
        write_witness_output_summary_with_trace(
            stdout,
            output.commitments(),
            include_trace_instance,
        );
    }
    Ok(())
}

pub fn build_witness_proof_core_artifact(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    public_values_hash: [u8; 32],
    witness_outputs: &[&ProveWitnessTraceCommitments],
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
    witness_outputs: &[&ProveWitnessTraceCommitments],
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

fn build_proof_bytes(
    request: &WitnessOutputSaveRequest<'_>,
    verify_outputs: bool,
    timings: &mut TimingRecorder,
) -> Result<Option<Vec<u8>>, String> {
    let Some(public_inputs) = request.public_inputs else {
        return Ok(None);
    };
    let public_values = read_public_values_file(public_inputs)
        .map_err(|error| format!("read public inputs failed: {error}"))?;
    if let Some(input) = request.eth_block_input {
        validate_eth_block_public_values_with_program_image_cache(
            input,
            &public_values,
            request.program_image_cache,
        )
        .map_err(|error| error.to_string())?;
    }
    let unit_index = request.output.commitments().unit_index();
    let trace_instance_index = request.output.commitments().trace_instance_index();
    let unit_values = match request.unit_values_segment_input {
        Some(path) => Some(read_packed_unit_values_segment_for_unit(
            request.schedule,
            unit_index,
            trace_instance_index,
            path,
        )?),
        None => None,
    };
    let mut proof_artifact_timing = lzvm_prover::WitnessProofArtifactTiming::default();
    let proof = if request.contribution_only {
        lzvm_prover::build_witness_contribution_proof_artifact_for_unit(
            &lzvm_prover::WitnessProofRequest {
                catalog: request.catalog,
                schedule: request.schedule,
                constant_tree_material_summaries: request.constant_tree_material_summaries,
                execution_unit: request.execution_unit,
                gpu_streams: request.gpu_streams,
                public_values: Some(&public_values),
                unit_values: unit_values.as_deref(),
                output: request.output,
                verify_outputs,
                program_image_cache: request.program_image_cache,
                eth_block_input: request.eth_block_input,
                challenge_values_segment: request.challenge_values_segment,
                include_contribution_segment: false,
            },
        )?
    } else {
        lzvm_prover::build_witness_proof_artifact_for_unit_with_timing(
            &lzvm_prover::WitnessProofRequest {
                catalog: request.catalog,
                schedule: request.schedule,
                constant_tree_material_summaries: request.constant_tree_material_summaries,
                execution_unit: request.execution_unit,
                gpu_streams: request.gpu_streams,
                public_values: Some(&public_values),
                unit_values: unit_values.as_deref(),
                output: request.output,
                verify_outputs,
                program_image_cache: request.program_image_cache,
                eth_block_input: request.eth_block_input,
                challenge_values_segment: request.challenge_values_segment,
                include_contribution_segment: request.include_contribution_segment,
            },
            &mut proof_artifact_timing,
        )?
    };
    record_proof_artifact_timing(timings, &proof_artifact_timing);
    match proof {
        Some(proof) => {
            let started = Instant::now();
            let proof_bytes = encode_proof_artifact(&proof)
                .map_err(|error| format!("encode witness proof artifact failed: {error}"))?;
            timings.record("finish_proof_encode", started.elapsed());
            Ok(Some(proof_bytes))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests;
