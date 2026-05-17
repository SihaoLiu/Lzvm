use std::fs;
use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::trace_bundle::{read_trace_bundle_file, TraceBundle};
use lzvm_prover::{
    derive_prove_execution_plan_with_program_image_cache, ProveExecutionInputArtifacts,
};

use crate::program_image_cache::write_program_image_cache_summary;
use crate::prove_plan::{
    format_hash, parse_run_args, read_checked_setup_catalog, write_run_plan_summary, ParseError,
    ParsedRunArgs,
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

    let inputs = parsed_inputs(&parsed);
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
}

fn parse_inputs_args(args: &[&str]) -> Result<ParsedInputsArgs, ParseError> {
    let mut trace_bytes = None;
    let mut trace_bundle = None;
    let mut filtered = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        match args[index] {
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
    Ok(ParsedInputsArgs {
        run_args: parse_run_args(&filtered, min_positionals, max_positionals)?,
        trace_bytes,
        trace_bundle,
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
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove inputs --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>"
    );
    2
}
