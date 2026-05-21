use lzvm_artifacts::metadata_validation::{
    validate_global_metadata, validate_unit_metadata, MetadataValidationError,
};
mod fixtures;

#[test]
fn validates_consistent_unit_metadata() {
    let setup = fixtures::sample_metadata_bundle_setup_info();
    let expressions = fixtures::sample_metadata_bundle_expression_info();
    let verifier = fixtures::sample_metadata_bundle_verifier_info();

    validate_unit_metadata(&setup, &expressions, &verifier).expect("metadata should agree");
}

#[test]
fn rejects_constraint_count_mismatches() {
    let mut setup = fixtures::sample_metadata_bundle_setup_info();
    setup.n_constraints = Some(2);
    let expressions = fixtures::sample_metadata_bundle_expression_info();
    let verifier = fixtures::sample_metadata_bundle_verifier_info();

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
    let setup = fixtures::sample_metadata_bundle_setup_info();
    let mut expressions = fixtures::sample_metadata_bundle_expression_info();
    expressions.expressions[0].stage = 4;
    let verifier = fixtures::sample_metadata_bundle_verifier_info();

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
    let setup = fixtures::sample_metadata_bundle_setup_info();
    let mut expressions = fixtures::sample_metadata_bundle_expression_info();
    expressions.constraints[0].stage = 3;
    let verifier = fixtures::sample_metadata_bundle_verifier_info();

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
    let setup = fixtures::sample_metadata_bundle_setup_info();
    let expressions = fixtures::sample_metadata_bundle_expression_info();
    let mut verifier = fixtures::sample_metadata_bundle_verifier_info();
    verifier.query.expression_id = Some(11);

    assert!(matches!(
        validate_unit_metadata(&setup, &expressions, &verifier),
        Err(MetadataValidationError::VerifierQueryExpressionMissing { expression_id: 11 })
    ));
}

#[test]
fn rejects_verifier_query_stages_outside_the_setup_range() {
    let setup = fixtures::sample_metadata_bundle_setup_info();
    let expressions = fixtures::sample_metadata_bundle_expression_info();
    let mut verifier = fixtures::sample_metadata_bundle_verifier_info();
    verifier.query.stage = Some(4);

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
    let global = fixtures::sample_metadata_bundle_global_info();

    validate_global_metadata(&global).expect("metadata should agree");
}

#[test]
fn validates_global_proof_value_counts_with_array_lengths() {
    let mut global = fixtures::sample_metadata_bundle_global_info();
    global.proof_values_map[0].lengths = vec![2];
    global.num_proof_values = vec![3];

    validate_global_metadata(&global).expect("metadata should agree");
}

#[test]
fn rejects_global_proof_value_count_overflows() {
    let mut global = fixtures::sample_metadata_bundle_global_info();
    global.num_proof_values = vec![u64::MAX, 1];

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::ProofValueCountOverflow)
    ));
}

#[test]
fn rejects_global_proof_value_array_dimension_overflows() {
    let mut global = fixtures::sample_metadata_bundle_global_info();
    global.proof_values_map[0].lengths = vec![u64::MAX, 2];

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::ProofValueCountOverflow)
    ));
}

#[test]
fn rejects_global_metadata_without_challenge_counters() {
    let mut global = fixtures::sample_metadata_bundle_global_info();
    global.num_challenges.clear();

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::NoChallengeStages)
    ));
}

#[test]
fn rejects_global_proof_value_count_mismatches() {
    let mut global = fixtures::sample_metadata_bundle_global_info();
    global.num_proof_values = vec![1];

    assert!(matches!(
        validate_global_metadata(&global),
        Err(MetadataValidationError::ProofValueCountMismatch {
            expected: 1,
            found: 2
        })
    ));
}
