use std::io::Write;
use std::path::PathBuf;

use lzvm_setup::{write_source_companions, SourceCompanionWriteRequest};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "setup source companions failed: {message}");
            return 1;
        }
    };

    let request = SourceCompanionWriteRequest {
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        include_paths: parsed.include_paths,
        include_path_first: parsed.include_path_first,
        refresh_setup_directory_manifest: parsed.refresh_manifest,
        main_file: parsed.main_file,
        setup_dir: parsed.setup_dir,
    };
    let report = match write_source_companions(&request) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "setup source companions failed: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(
        stdout,
        "source_program_archive_bytes={}",
        report.source_program_archive.bytes_written
    );
    let _ = writeln!(
        stdout,
        "source_program_archive_sources={}",
        report.source_program_archive.source_count
    );
    let _ = writeln!(
        stdout,
        "source_program_archive_edges={}",
        report.source_program_archive.edge_count
    );
    let _ = writeln!(
        stdout,
        "source_modules={}",
        report.source_program_archive.module_count
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_pragmas={}",
        report.source_program_archive.fixed_file_pragma_count
    );
    let _ = writeln!(
        stdout,
        "source_air_template_fixed_file_pragmas={}",
        report
            .source_program_archive
            .air_template_fixed_file_pragma_count
    );
    let _ = writeln!(
        stdout,
        "source_air_units={}",
        report.source_program_archive.air_unit_count
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest_bytes={}",
        report.source_fixed_file_manifest.bytes_written
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest_entries={}",
        report.source_fixed_file_manifest.entry_count
    );
    let _ = writeln!(
        stdout,
        "setup_directory_manifest_refreshed={}",
        report.setup_directory_manifest.is_some()
    );
    if let Some(manifest) = report.setup_directory_manifest.as_ref() {
        let _ = writeln!(
            stdout,
            "setup_directory_manifest_bytes={}",
            manifest.bytes_written
        );
        let _ = writeln!(
            stdout,
            "setup_directory_manifest_fingerprint={}",
            manifest.fingerprint
        );
    }
    let _ = writeln!(stdout, "setup_dir={}", report.setup_dir.display());
    let _ = writeln!(
        stdout,
        "source_program_archive={}",
        report.source_program_archive.output_path.display()
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest={}",
        report.source_fixed_file_manifest.output_path.display()
    );
    if let Some(manifest) = report.setup_directory_manifest.as_ref() {
        let _ = writeln!(
            stdout,
            "setup_directory_manifest={}",
            manifest.path.display()
        );
    }
    0
}

struct ParsedArgs {
    main_file: PathBuf,
    setup_dir: PathBuf,
    include_paths: Vec<PathBuf>,
    include_path_first: bool,
    refresh_manifest: bool,
}

enum ParseError {
    Usage,
    Invalid(String),
}

fn parse_args(args: &[&str]) -> Result<ParsedArgs, ParseError> {
    let mut include_paths = Vec::new();
    let mut include_path_first = false;
    let mut refresh_manifest = false;
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
            "--refresh-manifest" => refresh_manifest = true,
            value if value.starts_with("--") => {
                return Err(ParseError::Invalid(format!("unknown option {value}")));
            }
            value => positionals.push(PathBuf::from(value)),
        }
        index += 1;
    }

    let [main_file, setup_dir] = positionals.as_slice() else {
        return Err(ParseError::Usage);
    };

    Ok(ParsedArgs {
        main_file: main_file.clone(),
        setup_dir: setup_dir.clone(),
        include_paths,
        include_path_first,
        refresh_manifest,
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
        "usage: lzvm setup write-source-companions [--include-path <dir>] [--include-path-first] [--refresh-manifest] <main-file> <setup-dir>"
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
            &["--include-path", "--refresh-manifest"][..],
        ] {
            let result = parse_args(args);

            assert!(matches!(
                result,
                Err(ParseError::Invalid(message)) if message == "missing --include-path value"
            ));
        }
    }
}
