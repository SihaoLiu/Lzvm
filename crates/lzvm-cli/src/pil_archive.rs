use std::io::Write;
use std::path::PathBuf;

use lzvm_setup::{write_source_program_archive, SourceProgramArchiveWriteRequest};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "pil archive failed: {message}");
            return 1;
        }
    };

    let request = SourceProgramArchiveWriteRequest {
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        include_paths: parsed.include_paths,
        include_path_first: parsed.include_path_first,
        main_file: parsed.main_file,
        output_path: parsed.output_path,
    };
    let report = match write_source_program_archive(&request) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "pil archive failed: {error}");
            return 1;
        }
    };
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
    let _ = writeln!(stdout, "sources={}", report.source_count);
    let _ = writeln!(stdout, "edges={}", report.edge_count);
    let _ = writeln!(stdout, "modules={}", report.module_count);
    let _ = writeln!(
        stdout,
        "fixed_file_pragmas={}",
        report.fixed_file_pragma_count
    );
    let _ = writeln!(
        stdout,
        "air_template_fixed_file_pragmas={}",
        report.air_template_fixed_file_pragma_count
    );
    let _ = writeln!(stdout, "air_units={}", report.air_unit_count);
    let _ = writeln!(stdout, "output={}", report.output_path.display());
    0
}

struct ParsedArgs {
    main_file: PathBuf,
    output_path: PathBuf,
    include_paths: Vec<PathBuf>,
    include_path_first: bool,
}

enum ParseError {
    Usage,
    Invalid(String),
}

fn parse_args(args: &[&str]) -> Result<ParsedArgs, ParseError> {
    let mut include_paths = Vec::new();
    let mut include_path_first = false;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "--include-path" => {
                index += 1;
                let value = required_option_value(args.get(index), "--include-path")?;
                include_paths.push(PathBuf::from(value));
            }
            "--include-path-first" => include_path_first = true,
            value if value.starts_with("--") => {
                return Err(ParseError::Invalid(format!("unknown option {value}")));
            }
            value => positionals.push(PathBuf::from(value)),
        }
        index += 1;
    }

    let [main_file, output_path] = positionals.as_slice() else {
        return Err(ParseError::Usage);
    };

    Ok(ParsedArgs {
        main_file: main_file.clone(),
        output_path: output_path.clone(),
        include_paths,
        include_path_first,
    })
}

fn required_option_value<'a>(value: Option<&&'a str>, option: &str) -> Result<&'a str, ParseError> {
    let Some(value) = value else {
        return Err(ParseError::Invalid(format!("missing {option} value")));
    };
    if value.starts_with("--") {
        return Err(ParseError::Invalid(format!("missing {option} value")));
    }
    Ok(value)
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm pil archive [--include-path <dir>] [--include-path-first] <main-file> <output-file>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_include_path_value_during_parse() {
        for args in [
            &["--include-path"][..],
            &["--include-path", "--include-path-first"][..],
        ] {
            let result = parse_args(args);

            assert!(matches!(
                result,
                Err(ParseError::Invalid(message)) if message == "missing --include-path value"
            ));
        }
    }
}
