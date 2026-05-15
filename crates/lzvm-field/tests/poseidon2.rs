use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt};

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

#[test]
fn poseidon2_width_16_matches_known_vector() {
    let input = std::array::from_fn(|index| Felt::from_u64(index as u64));

    let output = poseidon2_hash_16(input);

    assert_eq!(
        output.map(Felt::to_u64),
        [
            9_639_188_652_563_994_454,
            12_273_372_933_164_734_616,
            2_905_147_255_612_444_119,
            17_581_461_329_934_617_288,
            14_390_794_100_096_760_072,
            5_468_485_695_976_078_057,
            2_832_370_985_856_357_627,
            1_116_111_836_864_400_812,
            14_997_632_823_506_024_332,
            3_976_503_894_892_102_369,
            14_874_978_986_912_301_676,
            12_458_748_982_184_310_703,
            103_345_454_961_107_931,
            3_354_965_064_850_558_444,
            14_413_825_288_474_057_217,
            4_214_638_127_285_300_968,
        ]
    );
}

#[test]
fn poseidon2_width_16_is_not_identity() {
    let input = [Felt::ZERO; 16];

    assert_ne!(poseidon2_hash_16(input), input);
}
