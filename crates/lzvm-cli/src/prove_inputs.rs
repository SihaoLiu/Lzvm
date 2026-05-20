use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block_public_values::{
    public_values_from_eth_block_input, validate_eth_block_public_values,
};
use lzvm_artifacts::key_directory::{key_directory_catalog_digest, KeyDirectoryCatalog};
use lzvm_artifacts::public_values::{
    encode_public_values, public_values_digest, read_public_values_file, PublicValues,
};
use lzvm_artifacts::trace_bundle::{read_trace_bundle_file, TraceBundle};
use lzvm_prover::{
    derive_prove_execution_plan_with_program_image_cache, ProveExecutionInputArtifacts,
};

use crate::eth_block_prove_input::{validate_eth_block_input, write_eth_block_input_summary};
use crate::program_image_cache::write_program_image_cache_summary;
use crate::prove_plan::{
    format_hash, parse_run_args, prepare_requested_gpu_setup, read_checked_setup_catalog,
    required_option_value, validate_all_unit_stored_witness_limit, write_run_plan_summary,
    ParseError, ParsedRunArgs,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_inputs_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    let catalog = match read_checked_setup_catalog(&parsed.run_args.positionals[0]) {
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
    let trace_bytes_len = match validate_trace_bytes(&parsed.trace_bytes) {
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
    let eth_block_input = match validate_eth_block_input(&parsed.eth_block_input) {
        Ok(value) => value,
        Err(message) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    let public_inputs = match prepare_public_inputs(&parsed, &catalog, eth_block_input.as_ref()) {
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
        let _ = writeln!(stdout, "trace_bytes={}", path.display());
        let _ = writeln!(
            stdout,
            "trace_bytes_file_bytes={}",
            trace_bytes_len.expect("trace bytes length should be available")
        );
    }
    if let Some((path, bundle, bundle_len)) = &trace_bundle {
        let _ = writeln!(stdout, "trace_bundle={}", path.display());
        let _ = writeln!(stdout, "trace_bundle_units={}", bundle.unit_count());
        let _ = writeln!(stdout, "trace_bundle_bytes={}", bundle_len);
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
    if let Some(summary) = &eth_block_input {
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
    eth_block_input: Option<PathBuf>,
}

struct PreparedPublicInputs {
    inputs: ProveExecutionInputArtifacts,
    generated: bool,
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
    let mut eth_block_input = None;
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
            "--eth-block-input" => {
                index += 1;
                let value = required_option_value(args.get(index), "--eth-block-input")?;
                if eth_block_input.replace(value.into()).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --eth-block-input option".to_owned(),
                    ));
                }
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
    let trace_mode = trace_bytes.is_some() || trace_bundle.is_some();
    let min_positionals = if trace_mode { 3 } else { 4 };
    let max_positionals = if trace_mode { 4 } else { 5 };
    let run_args = parse_run_args(&filtered, min_positionals, max_positionals)?;
    if trace_bytes.is_some() && run_args.request.options.aggregate {
        return Err(ParseError::Invalid(
            "--trace-bytes requires a single-unit witness run".to_owned(),
        ));
    }
    Ok(ParsedInputsArgs {
        run_args,
        trace_bytes,
        trace_bundle,
        eth_block_input,
    })
}

fn parsed_inputs(parsed: &ParsedInputsArgs) -> ProveExecutionInputArtifacts {
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
        validate_eth_block_public_values(&summary.input, &public_values)
            .map_err(|error| error.to_string())?;
        return Ok(PreparedPublicInputs {
            inputs,
            generated: false,
        });
    }

    let output_dir = &parsed.run_args.positionals[1];
    let public_inputs = output_dir.join("eth-block-public-values.bin");
    let public_values = public_values_from_eth_block_input(setup_hash, &summary.input);
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

fn validate_trace_bytes(path: &Option<PathBuf>) -> Result<Option<u64>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let metadata = fs::metadata(path)
        .map_err(|error| format!("trace bytes are missing: {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("trace bytes are not a file: {}", path.display()));
    }
    Ok(Some(metadata.len()))
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
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --eth-block-input <block-input>\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::eth_block_input::{build_eth_block_input, encode_eth_block_input};

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
    fn validates_eth_block_input_artifacts() {
        let dir = std::env::temp_dir().join(format!(
            "lzvm-prove-inputs-eth-block-{}",
            std::process::id()
        ));
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
