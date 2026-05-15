use lzvm_field::{Ext3, Felt, TranscriptError};
use lzvm_prover::pcs_challenge::{
    derive_fri_queries, find_query_nonce, verify_query_nonce, PcsChallengeError,
};

#[test]
fn verifies_query_nonce_with_the_width_4_hash() {
    let challenge = Ext3::from_u64s([0, 1, 2]);
    let nonce = Felt::from_u64(3);

    assert!(verify_query_nonce(challenge, nonce, 0).expect("zero bits should be valid"));
    assert!(verify_query_nonce(challenge, nonce, 1).expect("one bit should be valid"));
    assert!(!verify_query_nonce(challenge, nonce, 2).expect("two bits should be valid"));
}

#[test]
fn rejects_query_nonce_work_bits_above_the_field_word_size() {
    assert_eq!(
        verify_query_nonce(Ext3::ONE, Felt::ZERO, 65),
        Err(PcsChallengeError::InvalidWorkBits { bits: 65 })
    );
}

#[test]
fn finds_the_first_query_nonce_that_satisfies_the_work_bits() {
    let challenge = Ext3::from_u64s([0, 1, 2]);

    let nonce = find_query_nonce(challenge, 2).expect("nonce search should find a value");

    assert!(verify_query_nonce(challenge, nonce, 2).expect("nonce should verify"));
    for candidate in 0..nonce.to_u64() {
        assert!(
            !verify_query_nonce(challenge, Felt::from_u64(candidate), 2)
                .expect("candidate should evaluate"),
            "found a smaller valid nonce"
        );
    }
}

#[test]
fn query_nonce_search_rejects_invalid_work_bits() {
    assert_eq!(
        find_query_nonce(Ext3::ONE, 65),
        Err(PcsChallengeError::InvalidWorkBits { bits: 65 })
    );
}

#[test]
fn derives_fri_queries_from_challenge_and_nonce() {
    let queries = derive_fri_queries(4, Ext3::from_u64s([11, 22, 33]), Felt::ZERO, 5, 4)
        .expect("query sampling should fit");

    assert_eq!(queries, vec![14, 11, 3, 7, 11]);
}

#[test]
fn rejects_unsupported_transcript_arities_for_query_sampling() {
    assert_eq!(
        derive_fri_queries(3, Ext3::ONE, Felt::ZERO, 1, 1),
        Err(PcsChallengeError::Transcript(
            TranscriptError::UnsupportedArity { arity: 3 }
        ))
    );
}
