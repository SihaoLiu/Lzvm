#[cfg(feature = "cuda")]
use lzvm_accel::cuda_goldilocks_butterfly;
#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_add, cuda_goldilocks_mul};

#[cfg(feature = "cuda")]
const MODULUS: u64 = 0xffff_ffff_0000_0001;

#[cfg(feature = "cuda")]
fn add_mod(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 + rhs as u128) % MODULUS as u128) as u64
}

#[cfg(feature = "cuda")]
fn mul_mod(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 * rhs as u128) % MODULUS as u128) as u64
}

#[cfg(feature = "cuda")]
fn sub_mod(lhs: u64, rhs: u64) -> u64 {
    if lhs >= rhs {
        lhs - rhs
    } else {
        MODULUS - (rhs - lhs)
    }
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_adds_goldilocks_vectors() {
    let lhs = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let rhs = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let expected = lhs
        .iter()
        .zip(&rhs)
        .map(|(lhs, rhs)| add_mod(*lhs, *rhs))
        .collect::<Vec<_>>();

    let actual = cuda_goldilocks_add(&lhs, &rhs).expect("cuda addition should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_rejects_mismatched_vector_lengths() {
    let error = cuda_goldilocks_add(&[1, 2, 3], &[4, 5]).expect_err("lengths should be checked");

    assert!(error.to_string().contains("length mismatch"));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_multiplies_goldilocks_vectors() {
    let lhs = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let rhs = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let expected = lhs
        .iter()
        .zip(&rhs)
        .map(|(lhs, rhs)| mul_mod(*lhs, *rhs))
        .collect::<Vec<_>>();

    let actual = cuda_goldilocks_mul(&lhs, &rhs).expect("cuda multiplication should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_computes_goldilocks_butterflies() {
    let even = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let odd = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let twiddle = vec![1, 3, 5, 7, 11, 13, 17, 19];
    let expected = even
        .iter()
        .zip(&odd)
        .zip(&twiddle)
        .map(|((even, odd), twiddle)| {
            let scaled = mul_mod(*odd, *twiddle);
            (add_mod(*even, scaled), sub_mod(*even, scaled))
        })
        .collect::<Vec<_>>();
    let expected_even = expected.iter().map(|(value, _)| *value).collect::<Vec<_>>();
    let expected_odd = expected.iter().map(|(_, value)| *value).collect::<Vec<_>>();

    let (actual_even, actual_odd) =
        cuda_goldilocks_butterfly(&even, &odd, &twiddle).expect("cuda butterfly should run");

    assert_eq!(actual_even, expected_even);
    assert_eq!(actual_odd, expected_odd);
}
