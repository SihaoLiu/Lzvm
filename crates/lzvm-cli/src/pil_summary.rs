use std::io::Write;
use std::path::PathBuf;

use lzvm_pil::{SourceLoaderConfig, SourceProgram, SourceProgramLoader};

pub fn run(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Usage) => return write_usage(stderr),
        Err(ParseError::Invalid(message)) => {
            let _ = writeln!(stderr, "pil summary failed: {message}");
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
            let _ = writeln!(stderr, "pil summary failed: {error}");
            return 1;
        }
    };

    write_summary(stdout, &program);
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

pub(crate) fn write_summary(stdout: &mut dyn Write, program: &SourceProgram) {
    let mut includes = 0;
    let mut uses = 0;
    let mut containers = 0;
    let mut functions = 0;
    let mut constants = 0;
    let mut columns = 0;
    let mut values = 0;
    let mut air_group_values = 0;
    let mut commits = 0;
    let mut publics = 0;
    let mut public_tables = 0;

    for module in &program.modules {
        includes += module.includes.len();
        uses += module.uses.len();
        containers += module.containers.len();
        functions += module.functions.len();
        constants += module.constants.len();
        columns += module.columns.len();
        values += module.values.len();
        air_group_values += module.air_group_values.len();
        commits += module.commits.len();
        publics += module.publics.len();
        public_tables += module.public_tables.len();
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "sources={}", program.graph.sources.len());
    let _ = writeln!(stdout, "edges={}", program.graph.edges.len());
    let _ = writeln!(stdout, "modules={}", program.modules.len());
    let _ = writeln!(stdout, "includes={includes}");
    let _ = writeln!(stdout, "uses={uses}");
    let _ = writeln!(stdout, "containers={containers}");
    let _ = writeln!(stdout, "functions={functions}");
    let _ = writeln!(stdout, "constants={constants}");
    let _ = writeln!(stdout, "columns={columns}");
    let _ = writeln!(stdout, "values={values}");
    let _ = writeln!(stdout, "air_group_values={air_group_values}");
    let _ = writeln!(stdout, "commits={commits}");
    let _ = writeln!(stdout, "publics={publics}");
    let _ = writeln!(stdout, "public_tables={public_tables}");
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm pil summary [--include-path <dir>] [--include-path-first] <main-file>"
    );
    2
}
