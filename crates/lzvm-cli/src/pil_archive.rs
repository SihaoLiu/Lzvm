use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::source_program::encode_source_program_archive;
use lzvm_pil::{build_source_program_archive, SourceLoaderConfig, SourceProgramLoader};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "pil archive failed: {message}");
            return 1;
        }
    };

    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        include_paths: parsed.include_paths,
        include_path_first: parsed.include_path_first,
    });
    let program = match loader.load_main(&parsed.main_file) {
        Ok(program) => program,
        Err(error) => {
            let _ = writeln!(stderr, "pil archive failed: {error}");
            return 1;
        }
    };

    let archive = match build_source_program_archive(&program) {
        Ok(archive) => archive,
        Err(error) => {
            let _ = writeln!(stderr, "pil archive failed: {error}");
            return 1;
        }
    };
    let bytes = match encode_source_program_archive(&archive) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "pil archive failed: {error}");
            return 1;
        }
    };
    if let Err(message) = write_output(&parsed.output_path, &bytes) {
        let _ = writeln!(stderr, "pil archive failed: {message}");
        return 1;
    }

    let bytes_written = std::fs::metadata(&parsed.output_path)
        .map(|meta| meta.len())
        .unwrap_or(bytes.len() as u64);
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "bytes_written={bytes_written}");
    let _ = writeln!(stdout, "output={}", parsed.output_path.display());
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

fn write_output(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm pil archive [--include-path <dir>] [--include-path-first] <main-file> <output-file>"
    );
    2
}
