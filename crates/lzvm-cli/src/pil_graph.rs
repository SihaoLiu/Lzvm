use std::io::Write;
use std::path::PathBuf;

use lzvm_pil::{
    IncludeKind, IncludeVisibility, SourceGraph, SourceGraphLoader, SourceLoaderConfig,
};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "pil graph failed: {message}");
            return 1;
        }
    };

    let mut loader = SourceGraphLoader::new(SourceLoaderConfig {
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        include_paths: parsed.include_paths,
        include_path_first: parsed.include_path_first,
    });
    let graph = match loader.load_main(&parsed.main_file) {
        Ok(graph) => graph,
        Err(error) => {
            let _ = writeln!(stderr, "pil graph failed: {error}");
            return 1;
        }
    };

    write_graph(stdout, &graph);
    0
}

struct ParsedArgs {
    main_file: PathBuf,
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

    let [main_file] = positionals.as_slice() else {
        return Err(ParseError::Usage);
    };

    Ok(ParsedArgs {
        main_file: main_file.clone(),
        include_paths,
        include_path_first,
    })
}

fn write_graph(stdout: &mut dyn Write, graph: &SourceGraph) {
    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "sources={}", graph.sources.len());
    let _ = writeln!(stdout, "edges={}", graph.edges.len());
    for (index, source) in graph.sources.iter().enumerate() {
        let _ = writeln!(stdout, "source[{index}]={}", source.source_name);
    }
    for (index, edge) in graph.edges.iter().enumerate() {
        let _ = writeln!(
            stdout,
            "edge[{index}]={}|{}|{}|{}|{}",
            edge.from,
            edge.to,
            edge.request,
            format_kind(edge.kind),
            format_visibility(edge.visibility)
        );
    }
}

fn format_kind(kind: IncludeKind) -> &'static str {
    match kind {
        IncludeKind::Include => "include",
        IncludeKind::Require => "require",
    }
}

fn format_visibility(visibility: IncludeVisibility) -> &'static str {
    match visibility {
        IncludeVisibility::Public => "public",
        IncludeVisibility::Private => "private",
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm pil graph [--include-path <dir>] [--include-path-first] <main-file>"
    );
    2
}
