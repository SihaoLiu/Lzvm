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
        "source_fixed_file_manifest_bytes={}",
        report.source_fixed_file_manifest.bytes_written
    );
    let _ = writeln!(
        stdout,
        "source_fixed_file_manifest_entries={}",
        report.source_fixed_file_manifest.entry_count
    );
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
    0
}

struct ParsedArgs {
    main_file: PathBuf,
    setup_dir: PathBuf,
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
                let value = args.get(index).ok_or_else(|| {
                    ParseError::Invalid("missing --include-path value".to_owned())
                })?;
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

    let [main_file, setup_dir] = positionals.as_slice() else {
        return Err(ParseError::Usage);
    };

    Ok(ParsedArgs {
        main_file: main_file.clone(),
        setup_dir: setup_dir.clone(),
        include_paths,
        include_path_first,
    })
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-source-companions [--include-path <dir>] [--include-path-first] <main-file> <setup-dir>"
    );
    2
}
