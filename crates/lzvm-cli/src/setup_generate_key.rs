use std::io::Write;
use std::path::PathBuf;

use lzvm_setup::{
    write_fixed_columns_from_source_directory, write_key_directory, write_source_companions,
    write_source_key_directory_metadata, FixedExtensionBackend, KeyDirectoryWriteReport,
    SourceCompanionWriteReport, SourceCompanionWriteRequest,
    SourceFixedColumnsDirectoryWriteReport, SourceFixedColumnsDirectoryWriteRequest,
    SourceKeyDirectoryMetadataRequest,
};

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
        if !parsed.setup_dir.join("pilout.globalInfo.bin").is_file() {
            match write_source_key_directory_metadata(&SourceKeyDirectoryMetadataRequest {
                working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                include_paths: parsed.include_paths.clone(),
                include_path_first: parsed.include_path_first,
                main_file: main_file.clone(),
                setup_dir: parsed.setup_dir.clone(),
            }) {
                Ok(_) => {}
                Err(error) => {
                    let _ = writeln!(stderr, "setup key generation failed: {error}");
                    return 1;
                }
            }
        }
        match write_fixed_columns_from_source_directory(&SourceFixedColumnsDirectoryWriteRequest {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            include_paths: parsed.include_paths.clone(),
            include_path_first: parsed.include_path_first,
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
