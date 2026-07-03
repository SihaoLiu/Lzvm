use std::fs;
use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::guest_image::read_guest_image_file;
use lzvm_artifacts::guest_input_segment::validate_framed_guest_input_segment;
use lzvm_prover::guest_pc_trace_backend::{summarize_guest_pc_trace_run, GuestPcTraceRunStatus};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "prove guest-run failed: {message}");
            return 1;
        }
    };
    let guest_image_info = match read_guest_image_file(&parsed.guest_image) {
        Ok(info) => info,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "prove guest-run failed: read guest image failed: {}: {error}",
                parsed.guest_image.display()
            );
            return 1;
        }
    };
    let input = match read_input_data(&parsed.input_data) {
        Ok(input) => input,
        Err(message) => {
            let _ = writeln!(stderr, "prove guest-run failed: {message}");
            return 1;
        }
    };
    let summary = match summarize_guest_pc_trace_run(
        &parsed.guest_image,
        &guest_image_info,
        &input,
        parsed.instruction_limit,
    ) {
        Ok(summary) => summary,
        Err(error) => {
            let _ = writeln!(stderr, "prove guest-run failed: {error}");
            return 1;
        }
    };
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "guest_image={}", parsed.guest_image.display());
    let _ = writeln!(stdout, "guest_image_bytes={}", guest_image_info.byte_len);
    match &parsed.input_data {
        Some(path) => {
            let _ = writeln!(stdout, "input_data={}", path.display());
        }
        None => {
            let _ = writeln!(stdout, "input_data=none");
        }
    }
    let _ = writeln!(stdout, "input_bytes={}", input.len());
    let _ = writeln!(
        stdout,
        "guest_run_instruction_limit={}",
        parsed.instruction_limit
    );
    let _ = writeln!(
        stdout,
        "guest_run_status={}",
        match summary.status {
            GuestPcTraceRunStatus::Halted => "halted",
            GuestPcTraceRunStatus::InstructionLimitExceeded => "instruction_limit_exceeded",
        }
    );
    let _ = writeln!(
        stdout,
        "guest_run_executed_instructions={}",
        summary.executed_instructions
    );
    let _ = writeln!(stdout, "guest_run_terminal_pc={}", summary.terminal_pc);
    let _ = writeln!(
        stdout,
        "guest_run_input_data_was_mapped={}",
        summary.input_data_was_mapped
    );
    0
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArgs {
    input_data: Option<PathBuf>,
    instruction_limit: u64,
    guest_image: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseError {
    Usage,
    Invalid(String),
}

fn parse_args(args: &[&str]) -> Result<ParsedArgs, ParseError> {
    let mut input_data = None;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index] {
            "--input-data" => {
                index += 1;
                let value = required_option_value(args.get(index), "--input-data")?;
                if input_data.replace(PathBuf::from(value)).is_some() {
                    return Err(ParseError::Invalid(
                        "duplicate --input-data option".to_owned(),
                    ));
                }
            }
            value if value.starts_with("--") => {
                return Err(ParseError::Invalid(format!("unknown option {value}")));
            }
            value => positionals.push(value),
        }
        index += 1;
    }
    if positionals.len() != 2 {
        return Err(ParseError::Usage);
    }
    let instruction_limit = positionals[0].parse().map_err(|_| {
        ParseError::Invalid(format!("invalid instruction limit: {}", positionals[0]))
    })?;
    Ok(ParsedArgs {
        input_data,
        instruction_limit,
        guest_image: PathBuf::from(positionals[1]),
    })
}

fn required_option_value<'a>(value: Option<&&'a str>, option: &str) -> Result<&'a str, ParseError> {
    let Some(value) = value else {
        return Err(ParseError::Invalid(format!("{option} requires a value")));
    };
    if value.starts_with("--") {
        return Err(ParseError::Invalid(format!("{option} requires a value")));
    }
    Ok(value)
}

fn read_input_data(path: &Option<PathBuf>) -> Result<Vec<u8>, String> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read framed guest input failed: {}: {error}",
            path.display()
        )
    })?;
    validate_framed_guest_input_segment(&bytes)
        .map_err(|error| format!("framed guest input is invalid: {error}"))?;
    Ok(bytes)
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove guest-run [--input-data <framed-input>] <instruction-limit> <guest-image>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_input_data_value_during_parse() {
        for args in [&["--input-data"][..], &["--input-data", "--input-data"][..]] {
            let result = parse_args(args);

            assert!(matches!(
                result,
                Err(ParseError::Invalid(message)) if message == "--input-data requires a value"
            ));
        }
    }

    #[test]
    fn rejects_duplicate_input_data_during_parse() {
        let result = parse_args(&[
            "--input-data",
            "input-a.bin",
            "--input-data",
            "input-b.bin",
            "8",
            "guest.elf",
        ]);

        assert!(matches!(
            result,
            Err(ParseError::Invalid(message)) if message == "duplicate --input-data option"
        ));
    }
}
