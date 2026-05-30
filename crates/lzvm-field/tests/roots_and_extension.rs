use lzvm_field::{Ext3, Felt, MAX_ROOT_OF_UNITY_BITS, MODULUS, SHIFT};

#[test]
fn exposes_shift_and_known_roots() {
    assert_eq!(SHIFT.to_u64(), 7);
    assert_eq!(MAX_ROOT_OF_UNITY_BITS, 32);
    assert_eq!(Felt::root_of_unity(0).unwrap().to_u64(), 1);
    assert_eq!(Felt::root_of_unity(1).unwrap().to_u64(), MODULUS - 1);
    assert_eq!(Felt::root_of_unity(5).unwrap().to_u64(), 64);
    assert_eq!(
        Felt::root_of_unity(32).unwrap().to_u64(),
        7_277_203_076_849_721_926
    );
}

#[test]
fn roots_have_the_expected_order() {
    for bits in 1..=8 {
        let root = Felt::root_of_unity(bits).expect("root should exist");
        assert_eq!(root.pow(1_u64 << bits).to_u64(), 1);
        assert_eq!(root.pow(1_u64 << (bits - 1)).to_u64(), MODULUS - 1);
    }
}

#[test]
fn rejects_roots_outside_the_table() {
    assert!(Felt::root_of_unity(MAX_ROOT_OF_UNITY_BITS + 1).is_none());
}

#[test]
fn cubic_extension_reduces_the_defining_polynomial() {
    let x = Ext3::new(Felt::ZERO, Felt::ONE, Felt::ZERO);

    assert_eq!(x * x * x, Ext3::new(Felt::ONE, Felt::ONE, Felt::ZERO));
}

#[test]
fn cubic_extension_adds_subtracts_and_multiplies() {
    let lhs = Ext3::from_u64s([3, 4, 5]);
    let rhs = Ext3::from_u64s([7, 8, 9]);

    assert_eq!((lhs + rhs).to_u64s(), [10, 12, 14]);
    assert_eq!(
        (lhs - rhs).to_u64s(),
        [MODULUS - 4, MODULUS - 4, MODULUS - 4]
    );
    assert_eq!((lhs * rhs).to_u64s(), [97, 173, 139]);
}

#[test]
fn cubic_extension_inverts_nonzero_values() {
    let value = Ext3::from_u64s([3, 4, 5]);
    let inverse = value
        .inverse()
        .expect("nonzero extension value has inverse");

    assert_eq!(value * inverse, Ext3::ONE);
}

#[test]
fn cubic_extension_zero_has_no_inverse() {
    assert!(Ext3::ZERO.inverse().is_none());
}
