use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use lzvm_artifacts::source_program::{
    encode_source_program_archive, SourceProgramArchive, SourceProgramArchiveEdge,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
    SourceProgramArchiveSource,
};
use lzvm_pil::{IncludeKind, IncludeVisibility, SourceLoaderConfig, SourceProgramLoader};

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

    let archive = match build_archive(&program) {
        Ok(archive) => archive,
        Err(message) => {
            let _ = writeln!(stderr, "pil archive failed: {message}");
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

fn build_archive(program: &lzvm_pil::SourceProgram) -> Result<SourceProgramArchive, String> {
    let sources = program
        .graph
        .sources
        .iter()
        .map(|source| SourceProgramArchiveSource {
            source_name: source.source_name.clone(),
            contents: source.contents.clone(),
        })
        .collect::<Vec<_>>();
    let mut source_indexes = BTreeMap::new();
    for (index, source) in program.graph.sources.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| "source program has too many sources to archive".to_owned())?;
        source_indexes.insert(source.source_name.clone(), index);
    }

    let mut edges = Vec::with_capacity(program.graph.edges.len());
    for edge in &program.graph.edges {
        let from_index = source_indexes
            .get(&edge.from)
            .copied()
            .ok_or_else(|| format!("missing source index for {}", edge.from))?;
        let to_index = source_indexes
            .get(&edge.to)
            .copied()
            .ok_or_else(|| format!("missing source index for {}", edge.to))?;
        edges.push(SourceProgramArchiveEdge {
            from_index,
            to_index,
            request: edge.request.clone(),
            kind: match edge.kind {
                IncludeKind::Include => SourceProgramArchiveIncludeKind::Include,
                IncludeKind::Require => SourceProgramArchiveIncludeKind::Require,
            },
            visibility: match edge.visibility {
                IncludeVisibility::Public => SourceProgramArchiveIncludeVisibility::Public,
                IncludeVisibility::Private => SourceProgramArchiveIncludeVisibility::Private,
            },
        });
    }

    Ok(SourceProgramArchive { sources, edges })
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
