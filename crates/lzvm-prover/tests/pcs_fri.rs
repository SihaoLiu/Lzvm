use lzvm_field::{Ext3, Felt, SHIFT};
use lzvm_prover::pcs_fri::{verify_fri_fold, PcsFriFoldError};

#[test]
fn verifies_binary_fri_fold_values() {
    let constant = Ext3::from_u64s([1, 2, 3]);
    let slope = Ext3::from_u64s([4, 5, 6]);
    let values = vec![constant + slope, constant - slope];
    let challenge = Ext3::from_u64s([7, 8, 9]);
    let point_inverse = SHIFT.inverse().expect("shift is nonzero");
    let expected = constant + slope * scale(challenge, point_inverse);

    let folded = verify_fri_fold(2, 1, 2, challenge, 0, &values)
        .expect("fold should verify over a binary group");

    assert_eq!(folded, expected);
}

#[test]
fn rejects_fri_fold_values_with_wrong_group_size() {
    let result = verify_fri_fold(2, 1, 2, Ext3::ONE, 0, &[Ext3::ONE]);

    assert!(matches!(
        result,
        Err(PcsFriFoldError::ValueLengthMismatch {
            expected: 2,
            found: 1
        })
    ));
}

fn scale(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
}
