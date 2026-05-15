use lzvm_field::{coset_extend_evaluations, intt_in_place, ntt_in_place, DomainError, Felt, SHIFT};

#[test]
fn ntt_and_intt_round_trip_single_column() {
    let mut values = [
        Felt::from_u64(3),
        Felt::from_u64(5),
        Felt::from_u64(7),
        Felt::from_u64(11),
    ];
    let original = values;

    ntt_in_place(&mut values, 2).expect("ntt should run");
    assert_ne!(values, original);
    intt_in_place(&mut values, 2).expect("intt should run");

    assert_eq!(values, original);
}

#[test]
fn ntt_rejects_lengths_that_do_not_match_domain_bits() {
    let mut values = [Felt::ONE, Felt::from_u64(2), Felt::from_u64(3)];

    assert!(matches!(
        ntt_in_place(&mut values, 2),
        Err(DomainError::LengthMismatch {
            expected: 4,
            found: 3
        })
    ));
}

#[test]
fn coset_extension_preserves_constant_polynomials() {
    let values = [Felt::from_u64(9), Felt::from_u64(9)];

    let extended = coset_extend_evaluations(&values, 1, 3).expect("extension should run");

    assert_eq!(extended, vec![Felt::from_u64(9); 8]);
}

#[test]
fn coset_extension_evaluates_linear_polynomials_on_shifted_domain() {
    let source = [Felt::from_u64(5), Felt::from_u64(1)];
    let root = Felt::root_of_unity(2).expect("root should exist");
    let two = Felt::from_u64(2);
    let three = Felt::from_u64(3);
    let shifted_root = SHIFT * root;

    let extended = coset_extend_evaluations(&source, 1, 2).expect("extension should run");

    assert_eq!(
        extended,
        vec![
            three + two * SHIFT,
            three + two * shifted_root,
            three - two * SHIFT,
            three - two * shifted_root,
        ]
    );
}

#[test]
fn coset_extension_rejects_invalid_domain_ordering() {
    let values = [
        Felt::ONE,
        Felt::from_u64(2),
        Felt::from_u64(3),
        Felt::from_u64(4),
    ];

    assert!(matches!(
        coset_extend_evaluations(&values, 2, 1),
        Err(DomainError::InvalidExtensionBits {
            source_bits: 2,
            target_bits: 1
        })
    ));
}
