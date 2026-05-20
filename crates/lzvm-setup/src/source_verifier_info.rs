use lzvm_artifacts::verifier_info::{
    VerifierCode, VerifierDestination, VerifierInfo, VerifierOperand, VerifierOperation,
    VerifierOperationKind,
};

pub(crate) fn source_verifier_info() -> VerifierInfo {
    let code = VerifierCode {
        expression_id: None,
        stage: None,
        line: "constant verifier expression".to_owned(),
        temporary_count: 1,
        operations: vec![VerifierOperation {
            op: VerifierOperationKind::Copy,
            destination: VerifierDestination {
                temporary_id: 0,
                dimension: 3,
            },
            sources: vec![VerifierOperand::Number {
                value: 1,
                dimension: 1,
            }],
        }],
    };
    VerifierInfo {
        quotient: code.clone(),
        query: code,
    }
}
