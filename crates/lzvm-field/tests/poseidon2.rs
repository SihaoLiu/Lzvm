use lzvm_field::{poseidon2_hash_8, Felt};

#[test]
fn poseidon2_width_8_matches_known_vector() {
    let input = [
        Felt::from_u64(0),
        Felt::from_u64(1),
        Felt::from_u64(2),
        Felt::from_u64(3),
        Felt::from_u64(4),
        Felt::from_u64(5),
        Felt::from_u64(6),
        Felt::from_u64(7),
    ];

    let output = poseidon2_hash_8(input);

    assert_eq!(
        output.map(Felt::to_u64),
        [
            14_266_028_122_062_624_699,
            5_353_147_180_106_052_723,
            15_203_350_112_844_181_434,
            17_630_919_042_639_565_165,
            16_601_551_015_858_213_987,
            10_184_091_939_013_874_068,
            16_774_100_645_754_596_496,
            12_047_415_603_622_314_780,
        ]
    );
}

#[test]
fn poseidon2_width_8_is_not_identity() {
    let input = [Felt::ZERO; 8];

    assert_ne!(poseidon2_hash_8(input), input);
}
