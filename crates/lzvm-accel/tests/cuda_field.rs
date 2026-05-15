#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_add, cuda_goldilocks_mul};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_butterfly, cuda_goldilocks_coset_extend, cuda_goldilocks_ntt,
    cuda_poseidon2_width16, cuda_poseidon2_width4, cuda_poseidon2_width4_find_nonce,
    cuda_poseidon2_width8,
};
#[cfg(feature = "cuda")]
use lzvm_field::{
    coset_extend_evaluations, ntt_in_place, poseidon2_hash_16, poseidon2_hash_4, poseidon2_hash_8,
    Felt,
};

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

#[test]
#[cfg(feature = "cuda")]
fn cuda_computes_forward_ntt() {
    let input = vec![3, 5, 7, 11, 13, 17, 19, 23];
    let mut expected = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    ntt_in_place(&mut expected, 3).expect("cpu ntt should run");
    let expected = expected.into_iter().map(Felt::to_u64).collect::<Vec<_>>();

    let actual = cuda_goldilocks_ntt(&input, 3).expect("cuda ntt should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_evaluations_over_shifted_cosets() {
    let input = vec![5, 1, 9, 9];
    let source = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    let expected = coset_extend_evaluations(&source, 2, 4)
        .expect("cpu coset extension should run")
        .into_iter()
        .map(Felt::to_u64)
        .collect::<Vec<_>>();

    let actual =
        cuda_goldilocks_coset_extend(&input, 2, 4).expect("cuda coset extension should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_4_states() {
    let input = (0_u64..12).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(4)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_4(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width4(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_finds_poseidon2_width_4_nonce_ranges() {
    let challenge = [0_u64, 1, 2];
    let target = 1_u64 << 62;
    let expected = (0_u64..128)
        .find(|candidate| {
            let state = [
                Felt::from_u64(challenge[0]),
                Felt::from_u64(challenge[1]),
                Felt::from_u64(challenge[2]),
                Felt::from_u64(*candidate),
            ];
            poseidon2_hash_4(state)[0].to_u64() < target
        })
        .expect("fixture should contain a matching nonce");

    let actual = cuda_poseidon2_width4_find_nonce(challenge, 0, 128, target)
        .expect("cuda nonce search should run");

    assert_eq!(actual, Some(expected));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_reports_empty_poseidon2_width_4_nonce_ranges() {
    let actual = cuda_poseidon2_width4_find_nonce([0, 1, 2], 0, 16, 0)
        .expect("cuda nonce search should run");

    assert_eq!(actual, None);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_8_states() {
    let input = (0_u64..16).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(8)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_8(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width8(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_16_states() {
    let input = (0_u64..32).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(16)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_16(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width16(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}
