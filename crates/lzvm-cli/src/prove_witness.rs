use std::io::Write;

use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_prover::{
    derive_prove_execution_plan, run_prove_witness_commitments, ProveExecutionInputArtifacts,
};

use crate::prove_plan::{parse_run_args, write_run_plan_summary, ParseError, ParsedRunArgs};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_run_args(args, 4, 5) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove witness failed: {message}");
            return 1;
        }
    };

    let catalog = match read_key_directory_catalog(&parsed.positionals[0]) {
        Ok(catalog) => catalog,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };

    let inputs = parsed_inputs(&parsed);
    let plan = match derive_prove_execution_plan(&catalog, parsed.request, inputs) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };
    let output = match run_prove_witness_commitments(&plan, 0) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(stderr, "prove witness failed: {error}");
            return 1;
        }
    };

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
        "usage: lzvm prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]"
    );
    2
}
