use std::fs;
use std::path::{Path, PathBuf};

use lzvm_pil::{
    ColumnKind, ConstantDeclarationKind, FixedFilePragmaKind, FunctionStatementKind,
    SourceLoaderConfig, SourceProgramLoader, ValueDeclarationKind,
};

fn case_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-pil-source-program-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("case directory should be created");
    dir
}

fn write_file(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("test file should have a parent"))
        .expect("parent directory should be created");
    fs::write(path, contents).expect("test file should be written");
}

#[test]
fn loads_source_program_with_declarations_from_graph_sources() {
    let root = case_dir("declarations");
    write_file(
        &root,
        "main.pil",
        "include \"shared.pil\";\n\
         #pragma arg -I pil,lib\n\
         #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
         use lib.shared;\n\
         container air.main;\n\
         const int ROWS = 2**16;\n\
         airtemplate Main(int N = 2**16) { finalize(); }\n\
         airgroup Main { Main(N: 2**16); }\n\
         function finalize(): int { int local = 1; return local; }\n\
         col witness main.trace[2];\n\
         challenge stage(3) alpha;\n\
         commit stage(2) public(main.trace) main_commit;\n\
         public output = main.trace[0];\n\
         publictable aggregate(sum, fold) table[cols][rows];",
    );
    write_file(
        &root,
        "shared.pil",
        "col fixed shared = [1, 2];\n\
         proofval proof.value;\n\
         airgroupval aggregate(sum) group.total;",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");

    assert_eq!(
        program
            .modules
            .iter()
            .map(|module| module.source_name.as_str())
            .collect::<Vec<_>>(),
        vec!["main.pil", "shared.pil"]
    );
    assert_eq!(program.graph.edges.len(), 1);
    assert_eq!(program.graph.edges[0].from, "main.pil");
    assert_eq!(program.graph.edges[0].to, "shared.pil");

    let main = &program.modules[0];
    assert_eq!(main.pragmas.len(), 2);
    assert_eq!(main.pragmas[0].value, "arg -I pil,lib");
    assert_eq!(main.fixed_file_pragmas.len(), 1);
    assert_eq!(
        main.fixed_file_pragmas[0].kind,
        FixedFilePragmaKind::OutputFixedFile
    );
    assert_eq!(
        main.fixed_file_pragmas[0]
            .path
            .as_ref()
            .map(|path| path.value.as_str()),
        Some("${AIR_NAME}.fixed")
    );
    assert!(main.fixed_file_pragmas[0]
        .path
        .as_ref()
        .is_some_and(|path| path.template));
    assert_eq!(main.includes.len(), 1);
    assert_eq!(main.uses.len(), 1);
    assert_eq!(main.containers.len(), 1);
    assert_eq!(main.air_templates.len(), 1);
    assert_eq!(main.air_groups.len(), 1);
    assert_eq!(main.air_groups[0].statements.len(), 1);
    assert_eq!(
        main.air_groups[0].statements[0].kind,
        FunctionStatementKind::Expression
    );
    assert_eq!(main.air_instances.len(), 1);
    assert_eq!(main.air_instances[0].template, "Main");
    assert_eq!(main.functions.len(), 1);
    assert_eq!(main.functions[0].name, "finalize");
    assert_eq!(main.constants.len(), 1);
    assert_eq!(main.constants[0].kind, ConstantDeclarationKind::Const);
    assert_eq!(main.constants[0].type_name.as_deref(), Some("int"));
    assert_eq!(main.constants[0].name, "ROWS");
    assert_eq!(main.air_templates[0].statements.len(), 1);
    assert_eq!(
        main.air_templates[0].statements[0].kind,
        FunctionStatementKind::Expression
    );
    assert_eq!(main.variables.len(), 1);
    assert_eq!(main.variables[0].type_name, "int");
    assert_eq!(main.variables[0].name, "local");
    assert_eq!(main.columns.len(), 1);
    assert_eq!(main.columns[0].kind, ColumnKind::Witness);
    assert_eq!(main.values.len(), 1);
    assert_eq!(main.values[0].kind, ValueDeclarationKind::Challenge);
    assert_eq!(main.air_group_values.len(), 0);
    assert_eq!(main.commits.len(), 1);
    assert_eq!(main.publics.len(), 1);
    assert_eq!(main.public_tables.len(), 1);

    let shared = &program.modules[1];
    assert!(shared.includes.is_empty());
    assert_eq!(shared.constants.len(), 0);
    assert_eq!(shared.variables.len(), 0);
    assert_eq!(shared.columns.len(), 1);
    assert_eq!(shared.columns[0].kind, ColumnKind::Fixed);
    assert_eq!(shared.values.len(), 1);
    assert_eq!(shared.values[0].kind, ValueDeclarationKind::ProofValue);
    assert_eq!(shared.air_group_values.len(), 1);

    fs::remove_dir_all(&root).expect("case directory should be removed");
}
