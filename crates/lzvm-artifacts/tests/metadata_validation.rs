use lzvm_artifacts::expression_info::parse_expression_info_json;
use lzvm_artifacts::global_info::parse_global_info_json;
use lzvm_artifacts::metadata_validation::{
    validate_global_metadata, validate_unit_metadata, MetadataValidationError,
};
use lzvm_artifacts::setup_info::parse_unit_setup_info_json;
use lzvm_artifacts::verifier_info::parse_verifier_info_json;

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

#[test]
fn validates_consistent_unit_metadata() {
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    let verifier =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");

    validate_unit_metadata(&setup, &expressions, &verifier).expect("metadata should agree");
}

#[test]
fn rejects_constraint_count_mismatches() {
    let setup_json = sample_setup_info_json().replace("\"nConstraints\": 1", "\"nConstraints\": 2");
    let setup = parse_unit_setup_info_json(&setup_json).expect("setup should parse");
    let expressions = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    let verifier =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::ConstraintCountMismatch {
            expected: 2,
            found: 1
        })
    ));
}

#[test]
fn rejects_expression_stages_outside_the_setup_range() {
    let expression_json = sample_expression_info_json().replace("\"stage\": 3", "\"stage\": 4");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions =
        parse_expression_info_json(&expression_json).expect("expressions should parse");
    let verifier =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::ExpressionStageOutOfRange {
            expression_id: 9,
            stage: 4,
            max_stage: 3
        })
    ));
}

#[test]
fn rejects_constraint_stages_outside_the_setup_range() {
    let expression_json = sample_expression_info_json().replace("\"stage\": 2", "\"stage\": 3");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions =
        parse_expression_info_json(&expression_json).expect("expressions should parse");
    let verifier =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::ConstraintStageOutOfRange {
            constraint_index: 0,
            stage: 3,
            max_stage: 2
        })
    ));
}

#[test]
fn rejects_verifier_query_ids_not_declared_by_expression_info() {
    let verifier_json = sample_verifier_info_json().replace("\"expId\": 9", "\"expId\": 11");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    let verifier = parse_verifier_info_json(&verifier_json).expect("verifier should parse");

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::VerifierQueryExpressionMissing { expression_id: 11 })
    ));
}

#[test]
fn rejects_verifier_query_stages_outside_the_setup_range() {
    let verifier_json = sample_verifier_info_json().replace("\"stage\": 3", "\"stage\": 4");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    let verifier = parse_verifier_info_json(&verifier_json).expect("verifier should parse");

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::VerifierQueryStageOutOfRange {
            stage: 4,
            max_stage: 3
        })
    ));
}

#[test]
fn validates_consistent_global_metadata() {
    let global = parse_global_info_json(sample_global_info_json()).expect("global should parse");

    validate_global_metadata(&global).expect("metadata should agree");
}

#[test]
fn rejects_global_metadata_without_challenge_counters() {
    let global_json =
        sample_global_info_json().replace("\"numChallenges\": [1, 2]", "\"numChallenges\": []");
    let global = parse_global_info_json(&global_json).expect("global should parse");

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::NoChallengeStages)
    ));
}

#[test]
fn rejects_global_proof_value_count_mismatches() {
    let global_json =
        sample_global_info_json().replace("\"numProofValues\": [1, 1]", "\"numProofValues\": [1]");
    let global = parse_global_info_json(&global_json).expect("global should parse");

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::ProofValueCountMismatch {
            expected: 1,
            found: 2
        })
    ));
}
