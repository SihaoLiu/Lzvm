use std::hint::black_box;
use std::time::{Duration, Instant};

use lzvm_field::{Felt, FieldError, MODULUS};

#[test]
fn exposes_the_expected_modulus() {
    assert_eq!(MODULUS, 0xffff_ffff_0000_0001);
}

#[test]
fn rejects_non_canonical_field_elements() {
    assert!(matches!(
        Felt::from_canonical(MODULUS),
        Err(FieldError::NonCanonical { .. })
    ));
}

#[test]
fn reduces_u64_inputs() {
    assert_eq!(Felt::from_u64(MODULUS + 5).to_u64(), 5);
}

#[test]
fn adds_with_modular_wraparound() {
    let lhs = Felt::from_canonical(MODULUS - 1).expect("input is canonical");
    let rhs = Felt::from_canonical(2).expect("input is canonical");

    assert_eq!((lhs + rhs).to_u64(), 1);
}

#[test]
fn subtracts_with_modular_wraparound() {
    let lhs = Felt::from_canonical(3).expect("input is canonical");
    let rhs = Felt::from_canonical(5).expect("input is canonical");

    assert_eq!((lhs - rhs).to_u64(), MODULUS - 2);
}

#[test]
fn multiplies_with_modular_wraparound() {
    let minus_one = Felt::from_canonical(MODULUS - 1).expect("input is canonical");

    assert_eq!((minus_one * minus_one).to_u64(), 1);
}

#[test]
fn multiplication_matches_reference_for_boundary_values() {
    let values = [
        0,
        1,
        2,
        3,
        (1_u64 << 32) - 2,
        (1_u64 << 32) - 1,
        1_u64 << 32,
        (1_u64 << 32) + 1,
        MODULUS / 2,
        MODULUS - 3,
        MODULUS - 2,
        MODULUS - 1,
    ];

    for lhs in values {
        for rhs in values {
            let actual = (Felt::from_canonical(lhs).expect("input is canonical")
                * Felt::from_canonical(rhs).expect("input is canonical"))
            .to_u64();
            let expected = ((lhs as u128 * rhs as u128) % MODULUS as u128) as u64;
            assert_eq!(actual, expected, "product mismatch for {lhs} * {rhs}");
        }
    }
}

#[test]
fn multiplies_many_field_elements_responsively() {
    let mut acc = Felt::from_canonical(1_234_567_891).expect("input is canonical");
    let mut rhs = Felt::from_canonical(MODULUS - 987_654_321).expect("input is canonical");
    let started = Instant::now();

    for index in 0..20_000_000_u64 {
        let lhs = Felt::from_u64(index.wrapping_mul(6_364_136_223_846_793_005));
        acc = black_box((black_box(acc + lhs) * black_box(rhs)) + lhs);
        rhs = black_box(rhs + Felt::from_u64(index ^ 0x9e37_79b9_7f4a_7c15));
    }

    let elapsed = started.elapsed();
    assert_ne!(acc.to_u64(), 0);
    assert!(
        elapsed < Duration::from_millis(250),
        "field multiplication took {elapsed:?}"
    );
}

#[test]
fn round_trips_little_endian_bytes() {
    let value = Felt::from_canonical(0x1122_3344_5566_7788).expect("input is canonical");

    assert_eq!(
        Felt::from_le_bytes(value.to_le_bytes()).to_u64(),
        value.to_u64()
    );
}

#[test]
fn raises_values_to_powers() {
    let value = Felt::from_canonical(5).expect("input is canonical");

    assert_eq!(value.pow(0).to_u64(), 1);
    assert_eq!(value.pow(3).to_u64(), 125);
}

#[test]
fn inverts_nonzero_values() {
    let value = Felt::from_canonical(7).expect("input is canonical");
    let inverse = value.inverse().expect("nonzero value has an inverse");

    assert_eq!((value * inverse).to_u64(), 1);
}

#[test]
fn zero_has_no_inverse() {
    assert!(Felt::ZERO.inverse().is_none());
}
