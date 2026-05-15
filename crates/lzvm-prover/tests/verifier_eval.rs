use lzvm_artifacts::verifier_info::{VerifierCode, VerifierOperation, VerifierOperationKind};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::verifier_eval::{
    evaluate_verifier_code, VerifierCommitmentColumn, VerifierEvalError, VerifierEvalInputs,
    VerifierOpenedStage,
};
use serde_json::json;

fn tmp(id: u32) -> serde_json::Value {
    json!({"type": "tmp", "id": id, "dim": 3})
}

fn source(kind: &str, id: u32) -> serde_json::Value {
    json!({"type": kind, "id": id, "dim": 3})
}

fn scalar_source(kind: &str, id: u32) -> serde_json::Value {
    json!({"type": kind, "id": id, "dim": 1})
}

fn number(value: &str) -> serde_json::Value {
    json!({"type": "number", "value": value, "dim": 1})
}

fn operation(
    op: VerifierOperationKind,
    destination: serde_json::Value,
    sources: Vec<serde_json::Value>,
) -> VerifierOperation {
    VerifierOperation {
        op,
        destination,
        sources,
    }
}

fn code(temporary_count: u32, operations: Vec<VerifierOperation>) -> VerifierCode {
    VerifierCode {
        expression_id: None,
        stage: None,
        line: String::new(),
        temporary_count,
        operations,
    }
}

#[test]
fn evaluates_verifier_code_arithmetic() {
    let code = code(
        3,
        vec![
            operation(
                VerifierOperationKind::Add,
                tmp(0),
                vec![number("3"), source("public", 0)],
            ),
            operation(
                VerifierOperationKind::Mul,
                tmp(1),
                vec![tmp(0), source("challenge", 0)],
            ),
            operation(
                VerifierOperationKind::Sub,
                tmp(2),
                vec![tmp(1), source("eval", 0)],
            ),
            operation(VerifierOperationKind::Copy, tmp(0), vec![tmp(2)]),
        ],
    );
    let challenges = [Ext3::from_u64s([2, 1, 0])];
    let evaluations = [Ext3::from_u64s([5, 0, 0])];
    let publics = [Felt::from_u64(4)];
    let inputs = VerifierEvalInputs {
        challenges: &challenges,
        evaluations: &evaluations,
        publics: &publics,
        constants: &[],
        commitments: &[],
        opened_stages: &[],
        commitment_columns: &[],
        zi: &[],
        proof_values: &[],
        x_div_x_sub: &[],
    };

    let value = evaluate_verifier_code(&code, &inputs).expect("code should evaluate");

    assert_eq!(
        value,
        (Ext3::from_u64s([3, 0, 0]) + Ext3::from_u64s([4, 0, 0])) * challenges[0] - evaluations[0]
    );
}

#[test]
fn evaluates_verifier_code_with_auxiliary_vectors() {
    let code = code(
        2,
        vec![
            operation(
                VerifierOperationKind::Add,
                tmp(0),
                vec![
                    json!({"type": "Zi", "boundaryId": 0, "dim": 3}),
                    source("proofvalue", 0),
                ],
            ),
            operation(
                VerifierOperationKind::Mul,
                tmp(1),
                vec![tmp(0), json!({"type": "xDivXSub", "id": 1, "dim": 3})],
            ),
            operation(VerifierOperationKind::Copy, tmp(0), vec![tmp(1)]),
        ],
    );
    let zi = [Ext3::from_u64s([7, 0, 0])];
    let proof_values = [Ext3::from_u64s([8, 1, 0])];
    let x_div_x_sub = [Ext3::ONE, Ext3::from_u64s([3, 0, 1])];
    let inputs = VerifierEvalInputs {
        challenges: &[],
        evaluations: &[],
        publics: &[],
        constants: &[],
        commitments: &[],
        opened_stages: &[],
        commitment_columns: &[],
        zi: &zi,
        proof_values: &proof_values,
        x_div_x_sub: &x_div_x_sub,
    };

    let value = evaluate_verifier_code(&code, &inputs).expect("code should evaluate");

    assert_eq!(value, (zi[0] + proof_values[0]) * x_div_x_sub[1]);
}

#[test]
fn evaluates_verifier_code_with_opened_values() {
    let code = code(
        3,
        vec![
            operation(
                VerifierOperationKind::Copy,
                tmp(0),
                vec![scalar_source("const", 1)],
            ),
            operation(
                VerifierOperationKind::Copy,
                tmp(1),
                vec![json!({"type": "cm", "id": 2, "dim": 3})],
            ),
            operation(VerifierOperationKind::Add, tmp(2), vec![tmp(0), tmp(1)]),
            operation(VerifierOperationKind::Copy, tmp(0), vec![tmp(2)]),
        ],
    );
    let constants = [Felt::from_u64(7), Felt::from_u64(11)];
    let commitments = [
        Felt::from_u64(1),
        Felt::from_u64(2),
        Felt::from_u64(13),
        Felt::from_u64(17),
        Felt::from_u64(19),
    ];
    let inputs = VerifierEvalInputs {
        constants: &constants,
        commitments: &commitments,
        ..VerifierEvalInputs::default()
    };

    let value = evaluate_verifier_code(&code, &inputs).expect("code should evaluate");

    assert_eq!(
        value,
        Ext3::from_u64s([11, 0, 0]) + Ext3::from_u64s([13, 17, 19])
    );
}

#[test]
fn evaluates_verifier_code_with_mapped_commitment_values() {
    let code = code(
        1,
        vec![operation(
            VerifierOperationKind::Copy,
            tmp(0),
            vec![json!({"type": "cm", "id": 1, "dim": 3})],
        )],
    );
    let stage_one_values = [
        Felt::from_u64(101),
        Felt::from_u64(103),
        Felt::from_u64(107),
        Felt::from_u64(109),
        Felt::from_u64(113),
    ];
    let stage_two_values = [Felt::from_u64(13), Felt::from_u64(17), Felt::from_u64(19)];
    let opened_stages = [
        VerifierOpenedStage {
            stage_index: 1,
            values: &stage_one_values,
        },
        VerifierOpenedStage {
            stage_index: 2,
            values: &stage_two_values,
        },
    ];
    let commitment_columns = [
        VerifierCommitmentColumn {
            stage_index: 1,
            position: 4,
        },
        VerifierCommitmentColumn {
            stage_index: 2,
            position: 0,
        },
    ];
    let inputs = VerifierEvalInputs {
        opened_stages: &opened_stages,
        commitment_columns: &commitment_columns,
        ..VerifierEvalInputs::default()
    };

    let value = evaluate_verifier_code(&code, &inputs).expect("code should evaluate");

    assert_eq!(value, Ext3::from_u64s([13, 17, 19]));
}

#[test]
fn rejects_verifier_code_source_indexes_outside_inputs() {
    let code = code(
        1,
        vec![operation(
            VerifierOperationKind::Copy,
            tmp(0),
            vec![source("eval", 2)],
        )],
    );
    let inputs = VerifierEvalInputs::default();

    let result = evaluate_verifier_code(&code, &inputs);

    assert!(matches!(
        result,
        Err(VerifierEvalError::SourceIndexOutOfRange { kind, index, len })
            if kind == "eval" && index == 2 && len == 0
    ));
}
