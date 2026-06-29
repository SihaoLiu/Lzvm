use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block_public_values::{
    public_values_from_eth_block_input_for_metadata,
    validate_eth_block_public_values_with_program_image_cache,
};
use lzvm_artifacts::key_directory::{key_directory_catalog_digest, KeyDirectoryCatalog};
use lzvm_artifacts::program_image::read_program_image_commitment_cache_file;
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, read_public_values_file, PublicValues,
};
use lzvm_artifacts::trace_bundle::{read_trace_bundle_file, TraceBundle};
use lzvm_prover::{
    derive_prove_execution_plan_with_program_image_cache, derive_prove_run_plan,
    ProveExecutionInputArtifacts,
};

use crate::eth_block_prove_input::{
    validate_eth_block_input, write_eth_block_input_from_public_input,
    write_eth_block_input_summary, EthBlockInputSummary, EthPublicInputMode,
};
use crate::program_image_cache::write_program_image_cache_summary;
use crate::prove_plan::{
    format_hash, parse_run_args, prepare_requested_gpu_setup, read_prove_setup_catalog,
    required_option_value, selected_guest_pc_trace_unit_index, set_default_input_data,
    validate_all_unit_stored_witness_limit, write_guest_pc_trace_capacity_summary,
    write_run_plan_summary, ParseError, ParsedRunArgs, GUEST_PC_TRACE_WITNESS_THREAD_POOLS,
};
use crate::trace_input_shape::validate_trace_input_shapes;

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut parsed = match parse_inputs_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    let catalog = match read_prove_setup_catalog(&parsed.run_args.positionals[0]) {
        Ok(catalog) => catalog,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    if parsed.trace_bytes.is_some() && parsed.run_args.request.options.aggregate {
        let _ = writeln!(
            stderr,
            "prove inputs failed: --trace-bytes requires a single-unit witness run"
        );
        return 1;
    }
    if parsed.guest_pc_trace_instruction_limit.is_some()
        && parsed.run_args.request.options.aggregate
    {
        let _ = writeln!(
            stderr,
            "prove inputs failed: --guest-pc-trace requires a single-unit witness run"
        );
        return 1;
    }
    let trace_bytes = match validate_trace_bytes(&parsed.trace_bytes) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };
    let trace_bundle = match validate_trace_bundle(&parsed.trace_bundle) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };
    let preflight_run_plan = match derive_prove_run_plan(&catalog, parsed.run_args.request.clone())
    {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };
    if let Err(message) = validate_trace_input_shapes(
        trace_bytes.map(|metadata| metadata.file_bytes),
        trace_bundle.as_ref().map(|(_, bundle, _)| bundle),
        preflight_run_plan.options.aggregate,
        0,
        &preflight_run_plan.schedule,
    ) {
        let _ = writeln!(stderr, "prove inputs failed: {message}");
        return 1;
    }
    let prepared_eth_block_input = match prepare_eth_block_input(&parsed) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };
    if let Some(summary) = &prepared_eth_block_input.summary {
        set_default_input_data(&mut parsed.run_args.request, &summary.path);
    }

    let public_inputs =
        match prepare_public_inputs(&parsed, &catalog, prepared_eth_block_input.summary.as_ref()) {
            Ok(public_inputs) => public_inputs,
            Err(message) => {
                let _ = writeln!(stderr, "prove inputs failed: {message}");
                return 1;
            }
        };
    let generated_public_inputs = public_inputs.generated;
    let inputs = public_inputs.inputs;
    let plan = match derive_prove_execution_plan_with_program_image_cache(
        &catalog,
        parsed.run_args.request,
        inputs,
        parsed.run_args.program_image_cache.clone(),
    ) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };
    if let Err(error) = prepare_requested_gpu_setup(&plan) {
        let _ = writeln!(stderr, "prove inputs failed: {error}");
        return 1;
    }
    if plan.run_plan.options.aggregate {
        if let Err(error) = validate_all_unit_stored_witness_limit(
            plan.run_plan.gpu.max_stored_witnesses,
            plan.units.len(),
        ) {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    }
    let public_inputs_summary = match summarize_public_inputs(plan.inputs.public_inputs.as_deref())
    {
        Ok(summary) => summary,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    write_run_plan_summary(stdout, &plan.run_plan);
    match (&plan.inputs.witness_library, &plan.witness_library_info) {
        (Some(path), Some(info)) => {
            let _ = writeln!(stdout, "witness_library={}", path.display());
            let _ = writeln!(stdout, "witness_library_bytes={}", info.byte_len);
            let _ = writeln!(stdout, "witness_library_machine={}", info.machine);
            let _ = writeln!(
                stdout,
                "witness_library_digest={}",
                format_hash(&info.digest)
            );
        }
        _ => {
            let _ = writeln!(stdout, "witness_library=none");
        }
    }
    if let Some(path) = &parsed.trace_bytes {
        let Some(trace_bytes) = trace_bytes else {
            let _ = writeln!(
                stderr,
                "prove inputs failed: trace bytes metadata is missing"
            );
            return 1;
        };
        let _ = writeln!(stdout, "trace_bytes={}", path.display());
        let _ = writeln!(stdout, "trace_bytes_file_bytes={}", trace_bytes.file_bytes);
        let _ = writeln!(
            stdout,
            "trace_bytes_storage_bytes={}",
            trace_bytes.storage_bytes_text()
        );
        let _ = writeln!(stdout, "trace_bytes_sparse={}", trace_bytes.sparse_text());
    }
    if let Some((path, bundle, bundle_len)) = &trace_bundle {
        let _ = writeln!(stdout, "trace_bundle={}", path.display());
        let _ = writeln!(stdout, "trace_bundle_units={}", bundle.unit_count());
        let _ = writeln!(stdout, "trace_bundle_bytes={}", bundle_len);
    }
    if let Some(instruction_limit) = parsed.guest_pc_trace_instruction_limit {
        let unit_index = match selected_guest_pc_trace_unit_index(&plan) {
            Ok(unit_index) => unit_index,
            Err(message) => {
                let _ = writeln!(stderr, "prove inputs failed: {message}");
                return 1;
            }
        };
        if let Err(message) =
            write_guest_pc_trace_capacity_summary(stdout, &plan, unit_index, instruction_limit)
        {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    }
    let _ = writeln!(stdout, "guest_image={}", plan.inputs.guest_image.display());
    let _ = writeln!(
        stdout,
        "guest_image_bytes={}",
        plan.guest_image_info.byte_len
    );
    let _ = writeln!(
        stdout,
        "guest_image_machine={}",
        plan.guest_image_info.machine
    );
    let _ = writeln!(stdout, "guest_image_entry={}", plan.guest_image_info.entry);
    let _ = writeln!(
        stdout,
        "guest_image_digest={}",
        format_hash(&plan.guest_image_info.digest)
    );
    let _ = writeln!(
        stdout,
        "public_inputs={}",
        plan.inputs
            .public_inputs
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    if let Some(summary) = &public_inputs_summary {
        let _ = writeln!(
            stdout,
            "public_inputs_hash={}",
            format_hash(&summary.digest)
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
    if let Some(summary) = &prepared_eth_block_input.summary {
        write_eth_block_input_summary(stdout, summary);
    }
    if let Some(summary) = &plan.program_image_cache {
        write_program_image_cache_summary(stdout, summary);
    }
    0
}

#[derive(Debug)]
struct ParsedInputsArgs {
    run_args: ParsedRunArgs,
    trace_bytes: Option<PathBuf>,
    trace_bundle: Option<PathBuf>,
    guest_pc_trace_instruction_limit: Option<u64>,
    eth_block_input: Option<PathBuf>,
    eth_public_input: Option<PathBuf>,
    eth_public_input_allow_trailing: bool,
}

struct PreparedPublicInputs {
    inputs: ProveExecutionInputArtifacts,
    generated: bool,
}

struct PreparedEthBlockInput {
    summary: Option<EthBlockInputSummary>,
    generated_from_public_input: bool,
}

struct PublicInputSummary {
    digest: [u8; 32],
    value_count: usize,
    field_count: usize,
}

fn summarize_public_inputs(path: Option<&Path>) -> Result<Option<PublicInputSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let public_values = read_public_values_file(path)
        .map_err(|error| format!("read public inputs failed: {}: {error}", path.display()))?;
    let digest = public_values_digest(&public_values)
        .map_err(|error| format!("digest public inputs failed: {}: {error}", path.display()))?;
    Ok(Some(PublicInputSummary {
        digest,
        value_count: public_values.values.len(),
        field_count: public_values_field_count(&public_values),
    }))
}

fn public_values_field_count(public_values: &PublicValues) -> usize {
    public_values
        .values
        .iter()
        .map(|entry| entry.elements.len())
        .sum()
}

fn parse_inputs_args(args: &[&str]) -> Result<ParsedInputsArgs, ParseError> {
    let mut trace_bytes = None;
    let mut trace_bundle = None;
    let mut guest_pc_trace_instruction_limit = None;
    let mut eth_block_input = None;
    let mut eth_public_input = None;
    let mut eth_public_input_allow_trailing = false;
    let mut filtered = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
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
                let value = parse_positive_u64(args.get(index), "--guest-pc-trace")?;
                if guest_pc_trace_instruction_limit.replace(value).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --guest-pc-trace option".to_owned(),
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
            value => filtered.push(value),
        }
        index += 1;
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
    let mut run_args = parse_run_args(&filtered, min_positionals, max_positionals)?;
    if guest_pc_trace_instruction_limit.is_some() && !run_args.witness_thread_pools_used {
        run_args.request.gpu.witness_thread_pools = GUEST_PC_TRACE_WITNESS_THREAD_POOLS;
    }
    if trace_bytes.is_some() && run_args.request.options.aggregate {
        return Err(ParseError::Invalid(
            "--trace-bytes requires a single-unit witness run".to_owned(),
        ));
    }
    if guest_pc_trace_instruction_limit.is_some() && run_args.request.options.aggregate {
        return Err(ParseError::Invalid(
            "--guest-pc-trace requires a single-unit witness run".to_owned(),
        ));
    }
    Ok(ParsedInputsArgs {
        run_args,
        trace_bytes,
        trace_bundle,
        guest_pc_trace_instruction_limit,
        eth_block_input,
        eth_public_input,
        eth_public_input_allow_trailing,
    })
}

fn prepare_eth_block_input(parsed: &ParsedInputsArgs) -> Result<PreparedEthBlockInput, String> {
    if let Some(path) = &parsed.eth_block_input {
        return Ok(PreparedEthBlockInput {
            summary: validate_eth_block_input(&Some(path.clone()))?,
            generated_from_public_input: false,
        });
    }
    let Some(path) = &parsed.eth_public_input else {
        return Ok(PreparedEthBlockInput {
            summary: None,
            generated_from_public_input: false,
        });
    };

    let output_path = parsed.run_args.positionals[1].join("eth-block.input");
    let mode = if parsed.eth_public_input_allow_trailing {
        EthPublicInputMode::AllowTrailing
    } else {
        EthPublicInputMode::Strict
    };
    Ok(PreparedEthBlockInput {
        summary: Some(write_eth_block_input_from_public_input(
            path,
            &output_path,
            mode,
        )?),
        generated_from_public_input: true,
    })
}

fn parsed_inputs(parsed: &ParsedInputsArgs) -> ProveExecutionInputArtifacts {
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

fn parse_positive_u64(value: Option<&&str>, option: &str) -> Result<u64, ParseError> {
    let value = required_option_value(value, option)?
        .parse::<u64>()
        .map_err(|_| ParseError::Invalid(format!("{option} value must be a positive integer")))?;
    if value == 0 {
        return Err(ParseError::Invalid(format!(
            "{option} value must be a positive integer"
        )));
    }
    Ok(value)
}

fn prepare_public_inputs(
    parsed: &ParsedInputsArgs,
    catalog: &KeyDirectoryCatalog,
    eth_block_input: Option<&crate::eth_block_prove_input::EthBlockInputSummary>,
) -> Result<PreparedPublicInputs, String> {
    let mut inputs = parsed_inputs(parsed);
    let Some(summary) = eth_block_input else {
        return Ok(PreparedPublicInputs {
            inputs,
            generated: false,
        });
    };

    let setup_hash = key_directory_catalog_digest(catalog)
        .map_err(|error| format!("derive setup hash failed: {error}"))?;
    if let Some(public_inputs) = &inputs.public_inputs {
        let public_values = read_public_values_file(public_inputs).map_err(|error| {
            format!(
                "read public inputs failed: {}: {error}",
                public_inputs.display()
            )
        })?;
        if public_values.setup_hash != setup_hash {
            return Err("public inputs setup hash mismatch".to_owned());
        }
        let program_image_cache =
            read_optional_program_image_cache(parsed.run_args.program_image_cache.as_deref())?;
        validate_eth_block_public_values_with_program_image_cache(
            &summary.input,
            &public_values,
            program_image_cache.as_ref(),
        )
        .map_err(|error| error.to_string())?;
        return Ok(PreparedPublicInputs {
            inputs,
            generated: false,
        });
    }

    let output_dir = &parsed.run_args.positionals[1];
    let public_inputs = output_dir.join("eth-block-public-values.bin");
    let program_image_cache =
        read_optional_program_image_cache(parsed.run_args.program_image_cache.as_deref())?;
    let public_values = public_values_from_eth_block_input_for_metadata(
        setup_hash,
        &summary.input,
        &catalog.layout.global_info,
        program_image_cache.as_ref(),
    )
    .map_err(|error| format!("encode ETH block public values failed: {error}"))?;
    let encoded = encode_public_values(&public_values)
        .map_err(|error| format!("encode ETH block public values failed: {error}"))?;
    if let Some(parent) = public_inputs.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "create public inputs directory failed: {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    fs::write(&public_inputs, encoded).map_err(|error| {
        format!(
            "write ETH block public values failed: {}: {error}",
            public_inputs.display()
        )
    })?;
    inputs.public_inputs = Some(public_inputs);
    Ok(PreparedPublicInputs {
        inputs,
        generated: true,
    })
}

fn read_optional_program_image_cache(
    path: Option<&Path>,
) -> Result<Option<lzvm_artifacts::program_image::ProgramImageCommitmentCache>, String> {
    path.map(|path| {
        read_program_image_commitment_cache_file(path).map_err(|error| {
            format!(
                "read program-image cache failed: {}: {error}",
                path.display()
            )
        })
    })
    .transpose()
}

#[derive(Debug, Clone, Copy)]
struct TraceBytesMetadata {
    file_bytes: u64,
    storage_bytes: Option<u64>,
}

impl TraceBytesMetadata {
    fn storage_bytes_text(self) -> String {
        self.storage_bytes
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    }

    fn sparse_text(self) -> &'static str {
        match self.storage_bytes {
            Some(storage_bytes) if storage_bytes < self.file_bytes => "true",
            Some(_) => "false",
            None => "unknown",
        }
    }
}

fn validate_trace_bytes(path: &Option<PathBuf>) -> Result<Option<TraceBytesMetadata>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let metadata = fs::metadata(path)
        .map_err(|error| format!("trace bytes are missing: {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("trace bytes are not a file: {}", path.display()));
    }
    Ok(Some(TraceBytesMetadata {
        file_bytes: metadata.len(),
        storage_bytes: trace_storage_bytes(&metadata),
    }))
}

#[cfg(unix)]
fn trace_storage_bytes(metadata: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;

    Some(metadata.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
fn trace_storage_bytes(_metadata: &fs::Metadata) -> Option<u64> {
    None
}

fn validate_trace_bundle(
    path: &Option<PathBuf>,
) -> Result<Option<(PathBuf, TraceBundle, u64)>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let metadata = fs::metadata(path)
        .map_err(|error| format!("trace bundle is missing: {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("trace bundle is not a file: {}", path.display()));
    }
    let bundle = read_trace_bundle_file(path)
        .map_err(|error| format!("trace bundle failed: {}: {error}", path.display()))?;
    Ok(Some((path.clone(), bundle, metadata.len())))
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --guest-pc-trace <instruction-limit> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --eth-block-input <block-input>\n  --eth-public-input <public-input>\n  --eth-public-input-allow-trailing\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>\n  --guest-pc-trace <instruction-limit>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::eth_block_input::{
        build_eth_block_input, encode_eth_block_input, parse_eth_block_input,
    };
    use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;

    fn temp_dir(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should resolve")
            .join("temp")
            .join(format!("lzvm-prove-inputs-{name}-{}", std::process::id()))
    }

    #[test]
    fn rejects_trace_bytes_with_aggregate_during_parse() {
        let result = parse_inputs_args(&[
            "--trace-bytes",
            "trace.bin",
            "--aggregate",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "--trace-bytes requires a single-unit witness run"
        ));
    }

    #[test]
    fn parses_guest_pc_trace_option_for_input_args() {
        let result = parse_inputs_args(&[
            "--guest-pc-trace",
            "64",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ])
        .expect("input args should parse");
        let inputs = parsed_inputs(&result);

        assert_eq!(result.guest_pc_trace_instruction_limit, Some(64));
        assert_eq!(inputs.witness_library, None);
        assert_eq!(inputs.guest_image, PathBuf::from("guest.elf"));
    }

    #[test]
    fn rejects_zero_guest_pc_trace_for_input_args() {
        let result =
            parse_inputs_args(&["--guest-pc-trace", "0", "setup-dir", "out-dir", "guest.elf"]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "--guest-pc-trace value must be a positive integer"
        ));
    }

    #[test]
    fn guest_pc_trace_uses_parallel_witness_threads_for_input_args() {
        let result = parse_inputs_args(&[
            "--guest-pc-trace",
            "64",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ])
        .expect("input args should parse");

        assert_eq!(result.run_args.request.gpu.witness_thread_pools, 32);
    }

    #[test]
    fn guest_pc_trace_preserves_explicit_witness_threads_for_input_args() {
        let result = parse_inputs_args(&[
            "--guest-pc-trace",
            "64",
            "--witness-thread-pools",
            "6",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ])
        .expect("input args should parse");

        assert_eq!(result.run_args.request.gpu.witness_thread_pools, 6);
    }

    #[test]
    fn rejects_guest_pc_trace_with_trace_bytes_during_parse() {
        let result = parse_inputs_args(&[
            "--guest-pc-trace",
            "64",
            "--trace-bytes",
            "trace.bin",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "cannot combine --guest-pc-trace with --trace-bytes or --trace-bundle"
        ));
    }

    #[test]
    fn rejects_guest_pc_trace_with_aggregate_during_parse() {
        let result = parse_inputs_args(&[
            "--guest-pc-trace",
            "64",
            "--aggregate",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "--guest-pc-trace requires a single-unit witness run"
        ));
    }

    #[test]
    fn parses_eth_block_input_option_for_input_args() {
        let result = parse_inputs_args(&[
            "--eth-block-input",
            "block.input",
            "setup-dir",
            "out-dir",
            "witness.so",
            "guest.elf",
        ])
        .expect("input args should parse");

        assert_eq!(result.eth_block_input, Some(PathBuf::from("block.input")));
    }

    #[test]
    fn parses_eth_public_input_option_for_input_args() {
        let result = parse_inputs_args(&[
            "--eth-public-input",
            "public.bin",
            "setup-dir",
            "out-dir",
            "witness.so",
            "guest.elf",
        ])
        .expect("input args should parse");

        assert_eq!(result.eth_public_input, Some(PathBuf::from("public.bin")));
    }

    #[test]
    fn rejects_combined_eth_block_and_public_input_options() {
        let result = parse_inputs_args(&[
            "--eth-block-input",
            "block.input",
            "--eth-public-input",
            "public.bin",
            "setup-dir",
            "out-dir",
            "witness.so",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message == "cannot combine --eth-block-input and --eth-public-input"
        ));
    }

    #[test]
    fn rejects_missing_eth_public_input_value_during_parse() {
        let result = parse_inputs_args(&[
            "--eth-public-input",
            "--trace-bytes",
            "trace.bin",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message)) if message == "missing --eth-public-input value"
        ));
    }

    #[test]
    fn writes_eth_public_input_option_as_block_input_artifact() {
        let dir = temp_dir("eth-public");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let input_path = dir.join("public.bin");
        let output_dir = dir.join("proof-out");
        fs::write(&input_path, sample_public_block_bytes_with_matching_roots())
            .expect("public input should be written");
        let parsed = parse_inputs_args(&[
            "--eth-public-input",
            input_path.to_str().expect("input path should be utf-8"),
            "setup-dir",
            output_dir.to_str().expect("output path should be utf-8"),
            "witness.so",
            "guest.elf",
        ])
        .expect("input args should parse");

        let prepared =
            prepare_eth_block_input(&parsed).expect("public input should prepare block input");
        let summary = prepared
            .summary
            .expect("block input summary should be present");
        let output_path = output_dir.join("eth-block.input");
        let encoded = fs::read(&output_path).expect("block input should be written");
        let parsed_input = parse_eth_block_input(&encoded).expect("block input should parse");

        assert!(prepared.generated_from_public_input);
        assert_eq!(summary.path, output_path);
        assert_eq!(summary.byte_len, encoded.len() as u64);
        assert_eq!(summary.input, parsed_input);
        assert_eq!(summary.block_number, 42);
        assert_eq!(summary.transaction_preimage_count, 1);
        assert_eq!(summary.withdrawal_count, Some(1));
        fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn rejects_eth_public_input_with_trailing_bytes() {
        let dir = temp_dir("eth-public-trailing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let input_path = dir.join("public.bin");
        let output_dir = dir.join("proof-out");
        let mut public_input = sample_public_block_bytes_with_matching_roots();
        public_input.extend_from_slice(b"tail");
        fs::write(&input_path, public_input).expect("public input should be written");
        let parsed = parse_inputs_args(&[
            "--eth-public-input",
            input_path.to_str().expect("input path should be utf-8"),
            "setup-dir",
            output_dir.to_str().expect("output path should be utf-8"),
            "witness.so",
            "guest.elf",
        ])
        .expect("input args should parse");

        let result = prepare_eth_block_input(&parsed);
        let output_exists = output_dir.join("eth-block.input").exists();
        fs::remove_dir_all(&dir).expect("temp dir should be removed");

        assert!(matches!(
            result,
            Err(message)
                if message
                    == format!(
                        "ETH public input failed: {}: unexpected trailing bytes in ETH public input: 4",
                        input_path.display()
                    )
        ));
        assert!(!output_exists);
    }

    #[test]
    fn writes_eth_public_input_with_allowed_trailing_bytes_as_block_input_artifact() {
        let dir = temp_dir("eth-public-allow-trailing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let input_path = dir.join("public.bin");
        let output_dir = dir.join("proof-out");
        let mut public_input = sample_public_block_bytes_with_matching_roots();
        public_input.extend_from_slice(b"tail");
        fs::write(&input_path, public_input).expect("public input should be written");
        let parsed = parse_inputs_args(&[
            "--eth-public-input",
            input_path.to_str().expect("input path should be utf-8"),
            "--eth-public-input-allow-trailing",
            "setup-dir",
            output_dir.to_str().expect("output path should be utf-8"),
            "witness.so",
            "guest.elf",
        ])
        .expect("input args should parse");

        let prepared =
            prepare_eth_block_input(&parsed).expect("public input should prepare block input");
        let summary = prepared
            .summary
            .expect("block input summary should be present");
        let output_path = output_dir.join("eth-block.input");
        let encoded = fs::read(&output_path).expect("block input should be written");
        let parsed_input = parse_eth_block_input(&encoded).expect("block input should parse");

        assert!(prepared.generated_from_public_input);
        assert_eq!(summary.path, output_path);
        assert_eq!(summary.byte_len, encoded.len() as u64);
        assert_eq!(summary.input, parsed_input);
        assert_eq!(summary.block_number, 42);
        assert_eq!(summary.transaction_preimage_count, 1);
        assert_eq!(summary.withdrawal_count, Some(1));
        fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn rejects_eth_public_input_allow_trailing_without_eth_public_input() {
        let result = parse_inputs_args(&[
            "--eth-public-input-allow-trailing",
            "--trace-bytes",
            "trace.bin",
            "setup-dir",
            "out-dir",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message))
                if message
                    == "cannot use --eth-public-input-allow-trailing without --eth-public-input"
        ));
    }

    #[test]
    fn validates_eth_block_input_artifacts() {
        let dir = temp_dir("eth-block");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let input_path = dir.join("block.input");
        let block_rlp = sample_block_rlp();
        let input = build_eth_block_input(&block_rlp).expect("block input should build");
        let encoded = encode_eth_block_input(&input).expect("block input should encode");
        fs::write(&input_path, &encoded).expect("block input should be written");

        let summary = validate_eth_block_input(&Some(input_path.clone()))
            .expect("block input should validate")
            .expect("block input summary should exist");

        assert_eq!(summary.path, input_path);
        assert_eq!(summary.byte_len, encoded.len() as u64);
        assert_eq!(summary.block_rlp_len, block_rlp.len());
        assert_eq!(summary.block_number, 2);
        assert_eq!(summary.timestamp, 101);
        assert_eq!(summary.transaction_preimage_count, 1);
        assert_eq!(summary.legacy_transaction_count, 1);
        assert_eq!(summary.typed_transaction_count, 0);
        assert_eq!(summary.legacy_receipt_count, None);
        assert_eq!(summary.typed_receipt_count, None);
        assert_eq!(summary.withdrawal_preimage_count, None);
        fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    fn sample_block_rlp() -> Vec<u8> {
        let header_rlp = rlp_list(&legacy_header_items(
            hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
            None,
        ));
        let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
        let empty_list = rlp_list(&[]);
        rlp_list(&[header_rlp, transactions, empty_list])
    }

    fn legacy_header_items(
        transactions_root: [u8; 32],
        withdrawals_root: Option<[u8; 32]>,
    ) -> Vec<Vec<u8>> {
        let mut items = vec![
            rlp_bytes(&[0x11; 32]),
            rlp_bytes(&hex32(
                "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )),
            rlp_bytes(&[0x33; 20]),
            rlp_bytes(&[0x44; 32]),
            rlp_bytes(&transactions_root),
            rlp_bytes(&[0x66; 32]),
            rlp_bytes(&[0x77; 256]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
            rlp_bytes(&[0x0f, 0x42, 0x40]),
            rlp_bytes(&[0x0d, 0xbb, 0xa0]),
            rlp_bytes(&[0x65]),
            rlp_bytes(b"lzvm"),
            rlp_bytes(&[0xaa; 32]),
            rlp_bytes(&[0xbb; 8]),
        ];
        if let Some(root) = withdrawals_root {
            items.push(rlp_bytes(&[1]));
            items.push(rlp_bytes(&root));
        }
        items
    }

    fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
        if payload.len() == 1 && payload[0] <= 0x7f {
            return vec![payload[0]];
        }
        rlp_with_payload(0x80, 0xb7, payload)
    }

    fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
        let payload = items.iter().flatten().copied().collect::<Vec<_>>();
        rlp_with_payload(0xc0, 0xf7, &payload)
    }

    fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
        let mut input = sample_public_header_bytes();
        input.extend_from_slice(&1_u64.to_le_bytes());
        input.extend_from_slice(&eip1559_transaction_bytes());
        input.extend_from_slice(&0_u64.to_le_bytes());
        input.push(1);
        input.extend_from_slice(&1_u64.to_le_bytes());
        input.extend_from_slice(&withdrawal_bytes());

        let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
        let transaction_root = parsed.transactions_root();
        let ommers_hash = parsed.ommers_hash();
        let withdrawal_root = parsed
            .withdrawals_root()
            .expect("withdrawals root should be present");
        input[48..80].copy_from_slice(&ommers_hash);
        input[156..188].copy_from_slice(&transaction_root);
        input[237..269].copy_from_slice(&withdrawal_root);
        input
    }

    fn sample_public_header_bytes() -> Vec<u8> {
        let mut input = Vec::new();
        push_public_bytes(&mut input, &[1; 32]);
        push_public_bytes(&mut input, &[2; 32]);
        push_public_bytes(&mut input, &[3; 20]);
        push_public_bytes(&mut input, &[4; 32]);
        push_public_bytes(&mut input, &[5; 32]);
        push_public_bytes(&mut input, &[6; 32]);
        push_public_option_bytes(&mut input, Some(&[7; 32]));
        push_public_bytes(&mut input, &[8; 256]);
        push_public_bytes(&mut input, &u256_bytes(9));
        input.extend_from_slice(&42_u64.to_le_bytes());
        input.extend_from_slice(&100_u64.to_le_bytes());
        input.extend_from_slice(&90_u64.to_le_bytes());
        input.extend_from_slice(&77_u64.to_le_bytes());
        push_public_bytes(&mut input, &[10; 32]);
        push_public_bytes(&mut input, &[11; 8]);
        push_public_option_u64(&mut input, Some(123));
        push_public_option_u64(&mut input, Some(456));
        push_public_option_u64(&mut input, Some(789));
        push_public_option_bytes(&mut input, Some(&[12; 32]));
        push_public_option_bytes(&mut input, Some(&[13; 32]));
        push_public_bytes(&mut input, b"abc");
        input
    }

    fn eip1559_transaction_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_public_u256(&mut bytes, 0x11);
        push_public_u256(&mut bytes, 0x22);
        push_public_uint_u64(&mut bytes, 1);
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u64.to_le_bytes());
        bytes.extend_from_slice(&7_u64.to_le_bytes());
        bytes.extend_from_slice(&21_000_u64.to_le_bytes());
        bytes.extend_from_slice(&300_u128.to_le_bytes());
        bytes.extend_from_slice(&20_u128.to_le_bytes());
        push_public_option_bytes(&mut bytes, Some(&[9; 20]));
        push_public_u256(&mut bytes, 123);
        bytes.extend_from_slice(&0_u64.to_le_bytes());
        push_public_bytes(&mut bytes, b"call-data");
        bytes
    }

    fn withdrawal_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_public_uint_u64(&mut bytes, 7);
        push_public_uint_u64(&mut bytes, 8);
        push_public_bytes(&mut bytes, &[6; 20]);
        push_public_uint_u64(&mut bytes, 9);
        bytes
    }

    fn push_public_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(bytes);
    }

    fn push_public_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
        match bytes {
            Some(bytes) => {
                out.push(1);
                push_public_bytes(out, bytes);
            }
            None => out.push(0),
        }
    }

    fn push_public_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
        match value {
            Some(value) => {
                out.push(1);
                out.extend_from_slice(&value.to_le_bytes());
            }
            None => out.push(0),
        }
    }

    fn push_public_u256(out: &mut Vec<u8>, value: u8) {
        let mut bytes = [0; 32];
        bytes[31] = value;
        push_public_bytes(out, &bytes);
    }

    fn push_public_uint_u64(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&8_u64.to_le_bytes());
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn u256_bytes(value: u8) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[31] = value;
        bytes
    }

    fn rlp_with_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
        if payload.len() <= 55 {
            let mut output = vec![short_base + payload.len() as u8];
            output.extend_from_slice(payload);
            return output;
        }

        let length = length_bytes(payload.len());
        let mut output = vec![long_base + length.len() as u8];
        output.extend_from_slice(&length);
        output.extend_from_slice(payload);
        output
    }

    fn length_bytes(mut value: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        while value > 0 {
            bytes.push((value & 0xff) as u8);
            value >>= 8;
        }
        bytes.reverse();
        bytes
    }

    fn hex32(value: &str) -> [u8; 32] {
        hex_bytes(value)
            .try_into()
            .expect("hex value should have 32 bytes")
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        assert!(value.len().is_multiple_of(2));
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| (hex_value(pair[0]) << 4) | hex_value(pair[1]))
            .collect()
    }

    fn hex_value(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            b'A'..=b'F' => value - b'A' + 10,
            _ => panic!("invalid hex digit"),
        }
    }
}
