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
         use lib.shared;\n\
         container air.main;\n\
         const int ROWS = 2**16;\n\
         airtemplate Main(int N = 2**16) {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
             finalize();\n\
         }\n\
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
    assert_eq!(main.air_template_fixed_file_pragmas.len(), 1);
    assert_eq!(
        main.air_template_fixed_file_pragmas[0].template_name,
        "Main"
    );
    assert_eq!(
        main.air_template_fixed_file_pragmas[0].pragma.kind,
        FixedFilePragmaKind::OutputFixedFile
    );
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

#[test]
fn loads_require_directives_terminated_by_newline() {
    let root = case_dir("newline-require");
    write_file(
        &root,
        "main.pil",
        "require \"shared.pil\"\n\
         col witness main.trace;",
    );
    write_file(&root, "shared.pil", "col fixed shared = [1, 2];");
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
    assert_eq!(program.modules[0].includes.len(), 1);
    assert_eq!(program.modules[0].columns.len(), 1);
    assert_eq!(program.modules[1].columns.len(), 1);

    fs::remove_dir_all(&root).expect("case directory should be removed");
}

#[test]
fn indexes_air_units_with_group_and_unit_context() {
    let root = case_dir("air-units");
    write_file(
        &root,
        "main.pil",
        "airtemplate Main() { }\n\
         airtemplate Aux() { }\n\
         airgroup Alpha {\n\
             Main();\n\
             Main() alias AlphaMain;\n\
             virtual Aux() alias AuxVirtual;\n\
             Aux();\n\
         }\n\
         airgroup Beta { Main() alias BetaMain; }\n\
         airgroup Alpha { Aux() alias AlphaLater; }",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");
    let units = program.air_units();

    assert_eq!(
        units
            .iter()
            .map(|unit| (
                unit.group_name.as_str(),
                unit.group_id,
                unit.unit_id,
                unit.unit_name.as_str(),
                unit.template_name.as_str(),
                unit.virtual_instance
            ))
            .collect::<Vec<_>>(),
        vec![
            ("Alpha", 0, 0, "Main", "Main", false),
            ("Alpha", 0, 1, "AlphaMain", "Main", false),
            ("Alpha", 0, 10_000, "AuxVirtual", "Aux", true),
            ("Alpha", 0, 2, "Aux", "Aux", false),
            ("Beta", 1, 0, "BetaMain", "Main", false),
            ("Alpha", 0, 3, "AlphaLater", "Aux", false),
        ]
    );

    fs::remove_dir_all(&root).expect("case directory should be removed");
}

#[test]
fn resolves_template_fixed_file_pragmas_for_air_units() {
    let root = case_dir("resolved-fixed-files");
    write_file(
        &root,
        "main.pil",
        "airtemplate Table() {\n\
             #pragma output_fixed_file `${AIRGROUP}/${AIRGROUP_ID}/${AIR_ID}/${AIR_NAME}/${AIRTEMPLATE}.fixed`\n\
         }\n\
         airgroup GroupA {\n\
             Table();\n\
             Table() alias Second;\n\
         }",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");
    let fixed_files = program
        .resolved_fixed_file_pragmas()
        .expect("fixed-file pragmas should resolve");

    assert_eq!(
        fixed_files
            .iter()
            .map(|fixed_file| (
                fixed_file.kind,
                fixed_file.path.as_deref(),
                fixed_file.group_name.as_str(),
                fixed_file.group_id,
                fixed_file.unit_id,
                fixed_file.unit_name.as_str(),
                fixed_file.template_name.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                FixedFilePragmaKind::OutputFixedFile,
                Some("GroupA/0/0/Table/Table.fixed"),
                "GroupA",
                0,
                0,
                "Table",
                "Table"
            ),
            (
                FixedFilePragmaKind::OutputFixedFile,
                Some("GroupA/0/1/Second/Table.fixed"),
                "GroupA",
                0,
                1,
                "Second",
                "Table"
            ),
        ]
    );

    fs::remove_dir_all(&root).expect("case directory should be removed");
}

#[test]
fn resolves_fixed_file_pragmas_with_template_parameters() {
    let root = case_dir("fixed-file-params");
    write_file(
        &root,
        "main.pil",
        "airtemplate Table(const string bin_file = \"default\", const int RC = 2) {\n\
             #pragma extern_fixed_file `${bin_file}/${RC}.bin`\n\
         }\n\
         airgroup GroupA {\n\
             Table();\n\
             Table(bin_file: \"override\", RC: 4) alias Second;\n\
         }",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");
    let fixed_files = program
        .resolved_fixed_file_pragmas()
        .expect("fixed-file pragmas should resolve");

    assert_eq!(
        fixed_files
            .iter()
            .map(|fixed_file| fixed_file.path.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("default/2.bin"), Some("override/4.bin")]
    );

    fs::remove_dir_all(&root).expect("case directory should be removed");
}

#[test]
fn resolves_fixed_file_pragmas_with_constant_expression_template_parameters() {
    let root = case_dir("fixed-file-expression-params");
    write_file(
        &root,
        "main.pil",
        "airtemplate Table(const int RC = 2**4) {\n\
             #pragma extern_fixed_file `${RC}.bin`\n\
         }\n\
         airgroup GroupA {\n\
             Table();\n\
             Table(RC: 1 + 1) alias Second;\n\
         }",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");
    let fixed_files = program
        .resolved_fixed_file_pragmas()
        .expect("fixed-file pragmas should resolve");

    assert_eq!(
        fixed_files
            .iter()
            .map(|fixed_file| fixed_file.path.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("16.bin"), Some("2.bin")]
    );

    fs::remove_dir_all(&root).expect("case directory should be removed");
}

#[test]
fn resolves_fixed_file_pragmas_with_uppercase_hex_template_parameters() {
    let root = case_dir("fixed-file-uppercase-hex-params");
    write_file(
        &root,
        "main.pil",
        "airtemplate Table(const int RC = 0X10) {\n\
             #pragma extern_fixed_file `${RC}.bin`\n\
         }\n\
         airgroup GroupA {\n\
             Table();\n\
             Table(RC: 0X20) alias Second;\n\
         }",
    );
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: root.clone(),
        ..SourceLoaderConfig::default()
    });

    let program = loader
        .load_main("main.pil")
        .expect("source program should load");
    let fixed_files = program
        .resolved_fixed_file_pragmas()
        .expect("fixed-file pragmas should resolve");

    assert_eq!(
        fixed_files
            .iter()
            .map(|fixed_file| fixed_file.path.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("16.bin"), Some("32.bin")]
    );

    fs::remove_dir_all(&root).expect("case directory should be removed");
}
