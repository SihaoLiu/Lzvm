use std::io::Write;

use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_prover::{derive_prove_execution_plan, ProveExecutionInputArtifacts};

use crate::prove_plan::{
    format_hash, parse_run_args, write_run_plan_summary, ParseError, ParsedRunArgs,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_run_args(args, 4, 5) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove inputs failed: {message}");
            return 1;
        }
    };

    let catalog = match read_key_directory_catalog(&parsed.positionals[0]) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed);
    let plan = match derive_prove_execution_plan(&catalog, parsed.request, inputs) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove inputs failed: {error}");
            return 1;
        }
    };

    write_run_plan_summary(stdout, &plan.run_plan);
    let _ = writeln!(
        stdout,
        "witness_library={}",
        plan.inputs.witness_library.display()
    );
    let _ = writeln!(
        stdout,
        "witness_library_bytes={}",
        plan.witness_library_info.byte_len
    );
    let _ = writeln!(
        stdout,
        "witness_library_machine={}",
        plan.witness_library_info.machine
    );
    let _ = writeln!(
        stdout,
        "witness_library_digest={}",
        format_hash(&plan.witness_library_info.digest)
    );
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
    0
}

fn parsed_inputs(parsed: &ParsedRunArgs) -> ProveExecutionInputArtifacts {
    ProveExecutionInputArtifacts {
        witness_library: parsed.positionals[2].clone(),
        guest_image: parsed.positionals[3].clone(),
        public_inputs: parsed.positionals.get(4).cloned(),
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove inputs [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]"
    );
    2
}
