use lzvm_field::{Felt, PoseidonTranscript, TranscriptError};
use lzvm_prover::pcs_transcript::{absorb_commit_values, PcsTranscriptError};

fn values(items: &[u64]) -> Vec<Felt> {
    items.iter().copied().map(Felt::from_u64).collect()
}

#[test]
fn direct_commit_values_match_plain_transcript_absorption() {
    let mut actual = PoseidonTranscript::new(4).expect("arity should be supported");
    actual.put(&values(&[1, 2, 3]));

    absorb_commit_values(&mut actual, 4, false, &values(&[10, 20, 30, 40, 50]))
        .expect("direct absorption should succeed");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&values(&[1, 2, 3]));
    expected.put(&values(&[10, 20, 30, 40, 50]));

    assert_eq!(actual.get_field(), expected.get_field());
}

#[test]
fn hashed_commit_values_absorb_inner_state_words() {
    let commit_values = values(&[10, 20, 30, 40, 50]);
    let mut actual = PoseidonTranscript::new(4).expect("arity should be supported");
    actual.put(&values(&[1, 2, 3]));

    absorb_commit_values(&mut actual, 4, true, &commit_values)
        .expect("hashed absorption should succeed");

    let mut inner = PoseidonTranscript::new(4).expect("arity should be supported");
    inner.put(&commit_values);
    let state = inner.get_state();
    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&values(&[1, 2, 3]));
    expected.put(&state);

    assert_eq!(actual.get_field(), expected.get_field());
}

#[test]
fn hashed_commit_values_reject_unsupported_inner_arities() {
    let mut transcript = PoseidonTranscript::new(4).expect("arity should be supported");

    assert_eq!(
        absorb_commit_values(&mut transcript, 3, true, &values(&[1])),
        Err(PcsTranscriptError::Transcript(
            TranscriptError::UnsupportedArity { arity: 3 }
        ))
    );
}
