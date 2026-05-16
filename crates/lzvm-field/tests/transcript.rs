use lzvm_field::{poseidon2_hash_16, Ext3, Felt, PoseidonTranscript, TranscriptError};

#[test]
fn transcript_arity_4_matches_known_challenge_vector() {
    let mut transcript = PoseidonTranscript::new(4).expect("arity 4 should be supported");
    transcript.put(&[Felt::from_u64(1), Felt::from_u64(2), Felt::from_u64(3)]);

    let challenge = transcript.get_field();

    assert_eq!(
        challenge,
        Ext3::from_u64s([
            17_564_457_412_598_474_136,
            9_659_173_666_345_536_325,
            7_103_278_074_368_904_402,
        ])
    );
}

#[test]
fn transcript_arity_2_matches_known_challenge_vector() {
    let mut transcript = PoseidonTranscript::new(2).expect("arity 2 should be supported");
    transcript.put(&[
        Felt::from_u64(5),
        Felt::from_u64(6),
        Felt::from_u64(7),
        Felt::from_u64(8),
        Felt::from_u64(9),
    ]);

    let challenge = transcript.get_field();

    assert_eq!(
        challenge,
        Ext3::from_u64s([
            13_164_848_049_087_015_226,
            14_407_249_316_318_504_937,
            14_502_241_229_650_658_507,
        ])
    );
}

#[test]
fn transcript_state_keeps_the_capacity_words() {
    let mut transcript = PoseidonTranscript::new(4).expect("arity 4 should be supported");
    transcript.put(&(1_u64..=20).map(Felt::from_u64).collect::<Vec<_>>());

    assert_eq!(
        transcript.get_state(),
        [
            Felt::from_u64(4_598_853_822_911_433_342),
            Felt::from_u64(4_202_911_911_610_835_622),
            Felt::from_u64(11_894_138_926_873_027_182),
            Felt::from_u64(17_771_460_674_839_058_898),
        ]
    );
}

#[test]
fn transcript_arity_4_exposes_full_hash_state() {
    let values = (1_u64..=8).map(Felt::from_u64).collect::<Vec<_>>();
    let mut expected_input = [Felt::ZERO; 16];
    expected_input[..values.len()].copy_from_slice(&values);
    let expected = poseidon2_hash_16(expected_input);

    let mut transcript = PoseidonTranscript::new(4).expect("arity 4 should be supported");
    transcript.put(&values);

    assert_eq!(transcript.get_state_words(), expected.to_vec());
}

#[test]
fn transcript_samples_query_permutations_from_field_bits() {
    let mut transcript = PoseidonTranscript::new(4).expect("arity 4 should be supported");
    transcript.put(&[Felt::from_u64(11), Felt::from_u64(22), Felt::from_u64(33)]);

    let queries = transcript
        .get_permutations(5, 4)
        .expect("query sampling should fit");

    assert_eq!(queries, vec![14, 11, 3, 7, 11]);
}

#[test]
fn transcript_rejects_unsupported_arities() {
    assert_eq!(
        PoseidonTranscript::new(3),
        Err(TranscriptError::UnsupportedArity { arity: 3 })
    );
}
