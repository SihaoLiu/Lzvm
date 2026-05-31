use std::path::PathBuf;

use lzvm_prover::ProveExecutionInputArtifacts;

use crate::prove_plan::{parse_run_args, required_option_value, ParseError, ParsedRunArgs};

pub(super) struct ParsedWitnessArgs {
    pub(super) run_args: ParsedRunArgs,
    pub(super) all_units: bool,
    pub(super) unit_index: Option<usize>,
    pub(super) trace_bytes: Option<PathBuf>,
    pub(super) trace_bundle: Option<PathBuf>,
    pub(super) guest_pc_trace_instruction_limit: Option<u64>,
    pub(super) unit_values: Option<PathBuf>,
    pub(super) unit_values_segment: Option<PathBuf>,
    pub(super) proof_values: Option<PathBuf>,
    pub(super) proof_values_segment: Option<PathBuf>,
    pub(super) group_values: Option<PathBuf>,
    pub(super) group_values_segment: Option<PathBuf>,
    pub(super) challenge_values: Option<PathBuf>,
    pub(super) challenge_values_segment: Option<PathBuf>,
    pub(super) evaluation_values: Option<PathBuf>,
    pub(super) evaluation_values_segment: Option<PathBuf>,
    pub(super) eth_block_input: Option<PathBuf>,
    pub(super) eth_public_input: Option<PathBuf>,
    pub(super) eth_public_input_allow_trailing: bool,
}

pub(super) fn parse_witness_args(args: &[&str]) -> Result<ParsedWitnessArgs, ParseError> {
    let mut all_units = false;
    let mut unit_index = None;
    let mut trace_bytes = None;
    let mut trace_bundle = None;
    let mut guest_pc_trace_instruction_limit = None;
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
    let mut eth_block_input = None;
    let mut eth_public_input = None;
    let mut eth_public_input_allow_trailing = false;
    let mut filtered = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--all-units" => all_units = true,
            "--unit-index" => {
                index += 1;
                let value = parse_usize(args.get(index), "--unit-index")?;
                if unit_index.replace(value).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --unit-index option".to_owned(),
                    ));
                }
            }
            "--trace-bytes" => {
                index += 1;
                let value = required_option_value(args.get(index), "--trace-bytes")?;
                if trace_bytes.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --trace-bytes option".to_owned(),
                    ));
                }
            }
            "--trace-bundle" => {
                index += 1;
                let value = required_option_value(args.get(index), "--trace-bundle")?;
                if trace_bundle.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --trace-bundle option".to_owned(),
                    ));
                }
            }
            "--guest-pc-trace" => {
                index += 1;
                let value = parse_u64(args.get(index), "--guest-pc-trace")?;
                if guest_pc_trace_instruction_limit.replace(value).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --guest-pc-trace option".to_owned(),
                    ));
                }
            }
            "--unit-values" => {
                index += 1;
                let value = required_option_value(args.get(index), "--unit-values")?;
                if unit_values.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --unit-values option".to_owned(),
                    ));
                }
            }
            "--unit-values-segment" => {
                index += 1;
                let value = required_option_value(args.get(index), "--unit-values-segment")?;
                if unit_values_segment.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --unit-values-segment option".to_owned(),
                    ));
                }
            }
            "--proof-values" => {
                index += 1;
                let value = required_option_value(args.get(index), "--proof-values")?;
                if proof_values.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --proof-values option".to_owned(),
                    ));
                }
            }
            "--proof-values-segment" => {
                index += 1;
                let value = required_option_value(args.get(index), "--proof-values-segment")?;
                if proof_values_segment.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --proof-values-segment option".to_owned(),
                    ));
                }
            }
            "--group-values" => {
                index += 1;
                let value = required_option_value(args.get(index), "--group-values")?;
                if group_values.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --group-values option".to_owned(),
                    ));
                }
            }
            "--group-values-segment" => {
                index += 1;
                let value = required_option_value(args.get(index), "--group-values-segment")?;
                if group_values_segment.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --group-values-segment option".to_owned(),
                    ));
                }
            }
            "--challenge-values" => {
                index += 1;
                let value = required_option_value(args.get(index), "--challenge-values")?;
                if challenge_values.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --challenge-values option".to_owned(),
                    ));
                }
            }
            "--challenge-values-segment" => {
                index += 1;
                let value = required_option_value(args.get(index), "--challenge-values-segment")?;
                if challenge_values_segment.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --challenge-values-segment option".to_owned(),
                    ));
                }
            }
            "--evaluation-values" => {
                index += 1;
                let value = required_option_value(args.get(index), "--evaluation-values")?;
                if evaluation_values.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --evaluation-values option".to_owned(),
                    ));
                }
            }
            "--evaluation-values-segment" => {
                index += 1;
                let value = required_option_value(args.get(index), "--evaluation-values-segment")?;
                if evaluation_values_segment.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --evaluation-values-segment option".to_owned(),
                    ));
                }
            }
            "--eth-block-input" => {
                index += 1;
                let value = required_option_value(args.get(index), "--eth-block-input")?;
                if eth_block_input.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --eth-block-input option".to_owned(),
                    ));
                }
            }
            "--eth-public-input" => {
                index += 1;
                let value = required_option_value(args.get(index), "--eth-public-input")?;
                if eth_public_input.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --eth-public-input option".to_owned(),
                    ));
                }
            }
            "--eth-public-input-allow-trailing" => {
                if eth_public_input_allow_trailing {
                    return Err(ParseError::Invalid(
                        "duplicate --eth-public-input-allow-trailing option".to_owned(),
                    ));
                }
                eth_public_input_allow_trailing = true;
            }
            "--program-image-cache" => {
                index += 1;
                let value = required_option_value(args.get(index), "--program-image-cache")?;
                filtered.push("--program-image-cache");
                filtered.push(value);
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
    if guest_pc_trace_instruction_limit.is_some()
        && (trace_bytes.is_some() || trace_bundle.is_some())
    {
        return Err(ParseError::Invalid(
            "cannot combine --guest-pc-trace with --trace-bytes or --trace-bundle".to_owned(),
        ));
    }
    if eth_block_input.is_some() && eth_public_input.is_some() {
        return Err(ParseError::Invalid(
            "cannot combine --eth-block-input and --eth-public-input".to_owned(),
        ));
    }
    if eth_public_input_allow_trailing && eth_public_input.is_none() {
        return Err(ParseError::Invalid(
            "cannot use --eth-public-input-allow-trailing without --eth-public-input".to_owned(),
        ));
    }
    let trace_mode = trace_bytes.is_some()
        || trace_bundle.is_some()
        || guest_pc_trace_instruction_limit.is_some();
    let min_positionals = if trace_mode { 3 } else { 4 };
    let max_positionals = if trace_mode { 4 } else { 5 };
    let run_args = parse_run_args(&filtered, min_positionals, max_positionals)?;
    if trace_bytes.is_some() && (all_units || run_args.request.options.aggregate) {
        return Err(ParseError::Invalid(
            "--trace-bytes requires a single-unit witness run".to_owned(),
        ));
    }
    if unit_index.is_some() && (all_units || run_args.request.options.aggregate) {
        return Err(ParseError::Invalid(
            "--unit-index requires a single-unit witness run".to_owned(),
        ));
    }
    if guest_pc_trace_instruction_limit.is_some()
        && (all_units || run_args.request.options.aggregate)
    {
        return Err(ParseError::Invalid(
            "--guest-pc-trace requires a single-unit witness run".to_owned(),
        ));
    }
    if evaluation_values_segment.is_some() && !(all_units || run_args.request.options.aggregate) {
        return Err(ParseError::Invalid(
            "--evaluation-values-segment requires all-units mode".to_owned(),
        ));
    }
    Ok(ParsedWitnessArgs {
        run_args,
        all_units,
        unit_index,
        trace_bytes,
        trace_bundle,
        guest_pc_trace_instruction_limit,
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
        eth_block_input,
        eth_public_input,
        eth_public_input_allow_trailing,
    })
}

pub(super) fn parsed_inputs(parsed: &ParsedWitnessArgs) -> ProveExecutionInputArtifacts {
    let trace_mode = parsed.trace_bytes.is_some()
        || parsed.trace_bundle.is_some()
        || parsed.guest_pc_trace_instruction_limit.is_some();
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

fn parse_u64(value: Option<&&str>, option: &str) -> Result<u64, ParseError> {
    required_option_value(value, option)?
        .parse::<u64>()
        .map_err(|_| ParseError::Invalid(format!("{option} value must be an unsigned integer")))
}

fn parse_usize(value: Option<&&str>, option: &str) -> Result<usize, ParseError> {
    required_option_value(value, option)?
        .parse::<usize>()
        .map_err(|_| ParseError::Invalid(format!("{option} value must be an unsigned integer")))
}
