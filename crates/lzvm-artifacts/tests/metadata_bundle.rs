use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::fixed::{encode_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::metadata_bundle::{
    read_global_metadata_bundle, read_unit_artifact_bundle, read_unit_metadata_bundle,
    GlobalMetadataPaths, MetadataBundleError, UnitArtifactPaths, UnitMetadataPaths,
};
use lzvm_artifacts::metadata_validation::MetadataValidationError;
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
use std::fs;
use std::path::{Path, PathBuf};

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 2,
        "nConstants": 5,
        "nPublics": 2,
        "nConstraints": 1,
        "qDeg": 7,
        "openingPoints": [0, 1, -1],
        "mapSectionsN": {
            "const": 5,
            "cm1": 2,
            "cm2": 3,
            "cm3": 1
        },
        "challengesMap": [{}, {}],
        "evMap": [{}],
        "boundaries": [],
        "starkStruct": {
            "nBits": 10,
            "nBitsExt": 13,
            "nQueries": 4,
            "steps": [
                {"nBits": 13},
                {"nBits": 9},
                {"nBits": 5}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 20,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_expression_info_json() -> &'static str {
    r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {
                "expId": 9,
                "stage": 3,
                "line": "query-expression",
                "tmpUsed": 0,
                "code": []
            }
        ],
        "constraints": [
            {
                "tmpUsed": 0,
                "code": [],
                "boundary": "everyRow",
                "line": "constraint-a",
                "imPol": 0,
                "stage": 2
            }
        ]
    }"#
}

fn sample_verifier_info_json() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 1,
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "number", "value": "1", "dim": 1}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 9,
            "stage": 3,
            "tmpUsed": 1,
            "line": "query-expression",
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "eval", "id": 0, "dim": 3}]
                }
            ]
        }
    }"#
}

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 1024}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 1,
        "numChallenges": [1, 2],
        "numProofValues": [1, 1],
        "proofValuesMap": [
            {"name": "proof-a", "stage": 1},
            {"name": "proof-b", "stage": 2}
        ],
        "publicsMap": [
            {"name": "public-a", "stage": 1}
        ],
        "transcriptArity": 4
    }"#
}

fn sample_expression_program(expression_id: u32) -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 1,
        max_args: 1,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id,
            destination_dimension: 3,
            destination_id: 0,
            stage: 1,
            temp1_count: 0,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 1,
            args_offset: 0,
            source_line: "program-line".to_owned(),
        }],
        ops: vec![1],
        args: vec![2],
        numbers: vec![],
    }
}

fn sample_fixed_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 3,
        columns: vec![FixedColumn {
            name: "main.value".to_owned(),
            dimensions: vec![1],
            values: vec![7, 8, 9],
        }],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-metadata-bundle-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("fixture should be written");
}

fn create_clean_dir(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    dir
}

fn write_unit_fixture(paths: &UnitMetadataPaths, setup: &str, expressions: &str, verifier: &str) {
    write_file(&paths.setup_info, setup);
    write_file(&paths.expression_info, expressions);
    write_file(&paths.verifier_info, verifier);
}

fn write_root_binary(path: &Path, values: Vec<u64>) {
    let bytes = encode_verification_key_binary(&VerificationKeyRoot::FieldElements(values))
        .expect("fixture should encode");
    fs::write(path, bytes).expect("fixture should be written");
}

fn write_expression_program(path: &Path, expression_id: u32) {
    let bytes = encode_expression_program(&sample_expression_program(expression_id))
        .expect("fixture should encode");
    fs::write(path, bytes).expect("fixture should be written");
}

fn write_fixed_columns(path: &Path) {
    let bytes = encode_fixed_columns(&sample_fixed_columns()).expect("fixture should encode");
    fs::write(path, bytes).expect("fixture should be written");
}

#[test]
fn derives_unit_metadata_paths_from_a_unit_prefix() {
    let paths = UnitMetadataPaths::from_unit_prefix(Path::new("/tmp/unit-a"));

    assert_eq!(
        paths.setup_info,
        PathBuf::from("/tmp/unit-a.starkinfo.json")
    );
    assert_eq!(
        paths.expression_info,
        PathBuf::from("/tmp/unit-a.expressionsinfo.json")
    );
    assert_eq!(
        paths.verifier_info,
        PathBuf::from("/tmp/unit-a.verifierinfo.json")
    );
}

#[test]
fn reads_and_validates_unit_metadata_from_paths() {
    let dir = create_clean_dir("unit-valid");
    let paths = UnitMetadataPaths::from_unit_prefix(dir.join("unit-a"));
    write_unit_fixture(
        &paths,
        sample_setup_info_json(),
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );

    let bundle = read_unit_metadata_bundle(&paths).expect("bundle should load");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(bundle.setup.n_stages, 2);
    assert_eq!(bundle.expressions.constraints.len(), 1);
    assert_eq!(bundle.verifier.query.expression_id, Some(9));
}

#[test]
fn rejects_unit_metadata_bundles_that_fail_cross_file_validation() {
    let dir = create_clean_dir("unit-invalid");
    let paths = UnitMetadataPaths::from_unit_prefix(dir.join("unit-a"));
    let setup_json = sample_setup_info_json().replace("\"nConstraints\": 1", "\"nConstraints\": 2");
    write_unit_fixture(
        &paths,
        &setup_json,
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );

    let error = read_unit_metadata_bundle(&paths).expect_err("bundle should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        error,
        MetadataBundleError::Validation(MetadataValidationError::ConstraintCountMismatch {
            expected: 2,
            found: 1
        })
    ));
}

#[test]
fn reads_and_validates_global_metadata_from_a_path() {
    let dir = create_clean_dir("global-valid");
    let path = dir.join("global_info.json");
    write_file(&path, sample_global_info_json());

    let bundle =
        read_global_metadata_bundle(&GlobalMetadataPaths::new(path)).expect("bundle should load");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(bundle.info.name, "sample-program");
    assert_eq!(bundle.info.total_air_count(), 1);
}

#[test]
fn rejects_global_metadata_bundles_that_fail_cross_file_validation() {
    let dir = create_clean_dir("global-invalid");
    let path = dir.join("global_info.json");
    let global_json =
        sample_global_info_json().replace("\"numChallenges\": [1, 2]", "\"numChallenges\": []");
    write_file(&path, &global_json);

    let error = read_global_metadata_bundle(&GlobalMetadataPaths::new(path))
        .expect_err("bundle should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        error,
        MetadataBundleError::Validation(MetadataValidationError::NoChallengeStages)
    ));
}

#[test]
fn derives_unit_artifact_paths_from_a_unit_prefix() {
    let paths = UnitArtifactPaths::from_unit_prefix(Path::new("/tmp/unit-a"));

    assert_eq!(
        paths.metadata.setup_info,
        PathBuf::from("/tmp/unit-a.starkinfo.json")
    );
    assert_eq!(
        paths.verification_key_json,
        PathBuf::from("/tmp/unit-a.verkey.json")
    );
    assert_eq!(
        paths.verification_key_binary,
        PathBuf::from("/tmp/unit-a.verkey.bin")
    );
    assert_eq!(paths.expression_program, PathBuf::from("/tmp/unit-a.bin"));
    assert_eq!(
        paths.verifier_program,
        PathBuf::from("/tmp/unit-a.verifier.bin")
    );
    assert_eq!(paths.fixed_columns, PathBuf::from("/tmp/unit-a.const"));
}

#[test]
fn reads_and_validates_unit_artifacts_from_paths() {
    let dir = create_clean_dir("unit-artifacts-valid");
    let paths = UnitArtifactPaths::from_unit_prefix(dir.join("unit-a"));
    write_unit_fixture(
        &paths.metadata,
        sample_setup_info_json(),
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );
    write_file(&paths.verification_key_json, "[1,2,3,4]");
    write_root_binary(&paths.verification_key_binary, vec![1, 2, 3, 4]);
    write_expression_program(&paths.expression_program, 17);
    write_expression_program(&paths.verifier_program, 9);
    write_fixed_columns(&paths.fixed_columns);

    let bundle = read_unit_artifact_bundle(&paths).expect("bundle should load");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(bundle.metadata.setup.n_stages, 2);
    assert_eq!(
        bundle.verification_key,
        VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4])
    );
    assert_eq!(bundle.expression_program.entries[0].expression_id, 17);
    assert_eq!(bundle.verifier_program.entries[0].expression_id, 9);
    assert_eq!(bundle.fixed_columns.row_count, 3);
}

#[test]
fn rejects_unit_artifacts_with_mismatched_key_roots() {
    let dir = create_clean_dir("unit-artifacts-invalid");
    let paths = UnitArtifactPaths::from_unit_prefix(dir.join("unit-a"));
    write_unit_fixture(
        &paths.metadata,
        sample_setup_info_json(),
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );
    write_file(&paths.verification_key_json, "[1,2,3,4]");
    write_root_binary(&paths.verification_key_binary, vec![1, 2, 3, 5]);
    write_expression_program(&paths.expression_program, 17);
    write_expression_program(&paths.verifier_program, 9);
    write_fixed_columns(&paths.fixed_columns);

    let error = read_unit_artifact_bundle(&paths).expect_err("bundle should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        error,
        MetadataBundleError::VerificationKeyMismatch {
            json_root: VerificationKeyRoot::FieldElements(_),
            binary_root: VerificationKeyRoot::FieldElements(_)
        }
    ));
}

#[test]
fn rejects_unit_artifacts_with_missing_verifier_programs() {
    let dir = create_clean_dir("unit-artifacts-missing-verifier-program");
    let paths = UnitArtifactPaths::from_unit_prefix(dir.join("unit-a"));
    write_unit_fixture(
        &paths.metadata,
        sample_setup_info_json(),
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );
    write_file(&paths.verification_key_json, "[1,2,3,4]");
    write_root_binary(&paths.verification_key_binary, vec![1, 2, 3, 4]);
    write_expression_program(&paths.expression_program, 17);
    write_fixed_columns(&paths.fixed_columns);

    let error = read_unit_artifact_bundle(&paths).expect_err("bundle should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(error, MetadataBundleError::ExpressionProgram(_)));
}

#[test]
fn rejects_unit_artifacts_with_missing_fixed_columns() {
    let dir = create_clean_dir("unit-artifacts-missing-fixed");
    let paths = UnitArtifactPaths::from_unit_prefix(dir.join("unit-a"));
    write_unit_fixture(
        &paths.metadata,
        sample_setup_info_json(),
        sample_expression_info_json(),
        sample_verifier_info_json(),
    );
    write_file(&paths.verification_key_json, "[1,2,3,4]");
    write_root_binary(&paths.verification_key_binary, vec![1, 2, 3, 4]);
    write_expression_program(&paths.expression_program, 17);
    write_expression_program(&paths.verifier_program, 9);

    let error = read_unit_artifact_bundle(&paths).expect_err("bundle should be rejected");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(error, MetadataBundleError::FixedColumns(_)));
}
