use std::io::Write;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::SOURCE_FIXED_FILE_MANIFEST_FILE;
use lzvm_artifacts::source_program::SOURCE_PROGRAM_ARCHIVE_FILE;
use lzvm_setup::{
    write_fixed_columns_from_source_directory, write_key_directory, write_source_companions,
    write_source_key_directory_metadata, FixedExtensionBackend, KeyDirectoryWriteReport,
    SourceCompanionWriteReport, SourceCompanionWriteRequest,
    SourceFixedColumnsDirectoryWriteReport, SourceFixedColumnsDirectoryWriteRequest,
    SourceKeyDirectoryMetadataRequest,
};

const SOURCE_GENERATION_STACK_BYTES: usize = 128 * 1024 * 1024;

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "setup key generation failed: {message}");
            return 1;
        }
    };

    let source_report = if let Some(main_file) = parsed.source.as_ref() {
        match run_source_generation(
            &parsed,
            main_file.clone(),
            should_write_source_metadata(&parsed.setup_dir),
        ) {
            Ok(report) => Some(report),
            Err(error) => {
                let _ = writeln!(stderr, "setup key generation failed: {error}");
                return 1;
            }
        }
    } else {
        None
    };

    match write_key_directory(&parsed.setup_dir, parsed.backend) {
        Ok(report) => {
            let companion_report = if let Some(main_file) = parsed.source.as_ref() {
                match write_source_companions(&SourceCompanionWriteRequest {
                    working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    include_paths: parsed.include_paths,
                    include_path_first: parsed.include_path_first,
                    refresh_setup_directory_manifest: true,
                    main_file: main_file.clone(),
                    setup_dir: parsed.setup_dir.clone(),
                }) {
                    Ok(report) => Some(report),
                    Err(error) => {
                        let _ = writeln!(stderr, "setup key generation failed: {error}");
                        return 1;
                    }
                }
            } else {
                None
            };
            write_report(
                stdout,
                source_report.as_ref(),
                companion_report.as_ref(),
                &report,
            );
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup key directory write failed: {error}");
            1
        }
    }
}

fn run_source_generation(
    parsed: &ParsedArgs,
    main_file: PathBuf,
    write_source_metadata: bool,
) -> Result<SourceFixedColumnsDirectoryWriteReport, String> {
    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let include_paths = parsed.include_paths.clone();
    let include_path_first = parsed.include_path_first;
    let setup_dir = parsed.setup_dir.clone();
    std::thread::Builder::new()
        .name("lzvm-source-generation".to_owned())
        .stack_size(SOURCE_GENERATION_STACK_BYTES)
        .spawn(move || {
            if write_source_metadata {
                write_source_key_directory_metadata(&SourceKeyDirectoryMetadataRequest {
                    working_dir: working_dir.clone(),
                    include_paths: include_paths.clone(),
                    include_path_first,
                    main_file: main_file.clone(),
                    setup_dir: setup_dir.clone(),
                })
                .map_err(|error| error.to_string())?;
            }
            write_fixed_columns_from_source_directory(&SourceFixedColumnsDirectoryWriteRequest {
                working_dir,
                include_paths,
                include_path_first,
                main_file,
                setup_dir,
            })
            .map_err(|error| error.to_string())
        })
        .map_err(|error| format!("source generation worker failed to start: {error}"))?
        .join()
        .map_err(|_| "source generation worker panicked".to_owned())?
}

fn should_write_source_metadata(setup_dir: &Path) -> bool {
    !setup_dir.join("pilout.globalInfo.bin").is_file()
        || setup_dir.join(SOURCE_PROGRAM_ARCHIVE_FILE).is_file()
        || setup_dir.join(SOURCE_FIXED_FILE_MANIFEST_FILE).is_file()
}

struct ParsedArgs {
    setup_dir: PathBuf,
    backend: FixedExtensionBackend,
    source: Option<PathBuf>,
    include_paths: Vec<PathBuf>,
    include_path_first: bool,
}

enum ParseError {
    Usage,
    Invalid(String),
}

fn parse_args(args: &[&str]) -> Result<ParsedArgs, ParseError> {
    let mut backend = FixedExtensionBackend::Cpu;
    let mut source = None;
    let mut include_paths = Vec::new();
    let mut include_path_first = false;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "--backend" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| ParseError::Invalid("missing --backend value".to_owned()))?;
                backend = parse_backend(value)?;
            }
            "--source" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| ParseError::Invalid("missing --source value".to_owned()))?;
                source = Some(PathBuf::from(value));
            }
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

    if source.is_none() && (!include_paths.is_empty() || include_path_first) {
        return Err(ParseError::Invalid(
            "source include options require --source".to_owned(),
        ));
    }

    let [setup_dir] = positionals.as_slice() else {
        return Err(ParseError::Usage);
    };

    Ok(ParsedArgs {
        setup_dir: setup_dir.clone(),
        backend,
        source,
        include_paths,
        include_path_first,
    })
}

fn parse_backend(value: &str) -> Result<FixedExtensionBackend, ParseError> {
    match value {
        "cpu" => Ok(FixedExtensionBackend::Cpu),
        "cuda" => Ok(FixedExtensionBackend::Cuda),
        _ => Err(ParseError::Invalid(format!("unsupported backend {value}"))),
    }
}

fn write_report(
    stdout: &mut dyn Write,
    source_report: Option<&SourceFixedColumnsDirectoryWriteReport>,
    companion_report: Option<&SourceCompanionWriteReport>,
    report: &KeyDirectoryWriteReport,
) {
    let manifest = companion_report
        .and_then(|report| report.setup_directory_manifest.as_ref())
        .unwrap_or(&report.manifest);

    let _ = writeln!(stdout, "status=ok");
    if let Some(source_report) = source_report {
        let _ = writeln!(stdout, "source_fixed_units={}", source_report.unit_count);
        let _ = writeln!(stdout, "source_fixed_bytes={}", source_report.bytes_written);
    }
    if let Some(companion_report) = companion_report {
        let _ = writeln!(
            stdout,
            "source_program_archive={}",
            companion_report
                .source_program_archive
                .output_path
                .display()
        );
        let _ = writeln!(
            stdout,
            "source_fixed_file_manifest={}",
            companion_report
                .source_fixed_file_manifest
                .output_path
                .display()
        );
    }
    let _ = writeln!(stdout, "units={}", report.base.unit_count);
    let _ = writeln!(stdout, "fixed_bytes={}", report.base.fixed_bytes);
    let _ = writeln!(stdout, "tree_bytes={}", report.base.tree_bytes);
    if let Some(verkey_bytes) = report.base.verkey_bytes {
        let _ = writeln!(stdout, "verkey_bytes={verkey_bytes}");
    }
    let _ = writeln!(stdout, "pcs_plan_bytes={}", report.pcs_plan.bytes_written);
    let _ = writeln!(
        stdout,
        "pcs_material_bytes={}",
        report.pcs_material.bytes_written
    );
    let _ = writeln!(stdout, "manifest_bytes={}", manifest.bytes_written);
    let _ = writeln!(stdout, "setup_hash={}", manifest.fingerprint);
    let _ = writeln!(
        stdout,
        "setup_directory_manifest={}",
        manifest.path.display()
    );
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup generate-key [--backend cpu|cuda] [--source <main-file>] [--include-path <dir>] [--include-path-first] <setup-dir>"
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_source_option_values_during_parse() {
        for (args, expected) in [
            (&["--backend"][..], "missing --backend value"),
            (&["--source"][..], "missing --source value"),
            (&["--include-path"][..], "missing --include-path value"),
        ] {
            let result = parse_args(args);

            assert!(
                matches!(result, Err(ParseError::Invalid(message)) if message == expected),
                "{expected}"
            );
        }
    }

    #[test]
    fn rejects_source_include_options_without_source_during_parse() {
        for args in [
            &["--include-path", "include", "setup-dir"][..],
            &["--include-path-first", "setup-dir"][..],
        ] {
            let result = parse_args(args);

            assert!(matches!(
                result,
                Err(ParseError::Invalid(message))
                    if message == "source include options require --source"
            ));
        }
    }
}
