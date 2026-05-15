use lzvm_field::{Ext3, Felt, PoseidonTranscript, TranscriptError};
use lzvm_prover::pcs_transcript::{
    absorb_commit_values, derive_pcs_final_query_challenge, PcsTranscriptError, PcsTranscriptInputs,
};

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

#[test]
fn derives_final_query_challenge_from_direct_transcript_events() {
    let constant_root = root(1);
    let public_values = values(&[7, 8]);
    let witness_roots = vec![root(10), root(20), root(30)];
    let evaluations = vec![ext(40), ext(50)];
    let fri_roots = vec![root(60), root(70)];
    let final_polynomial = vec![ext(80), ext(90)];

    let actual = derive_pcs_final_query_challenge(PcsTranscriptInputs {
        arity: 4,
        hash_values: false,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &fri_roots,
        final_polynomial: &final_polynomial,
    })
    .expect("challenge should derive");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&constant_root);
    expected.put(&public_values);
    put_root_and_draw(&mut expected, &witness_roots[0], 2);
    put_root_and_draw(&mut expected, &witness_roots[1], 1);
    put_root_and_draw(&mut expected, &witness_roots[2], 1);
    expected.put(&flatten_ext(&evaluations));
    draw(&mut expected, 2);
    expected.put(&fri_roots[0]);
    draw(&mut expected, 1);
    expected.put(&fri_roots[1]);
    draw(&mut expected, 1);
    expected.put(&flatten_ext(&final_polynomial));

    assert_eq!(actual, expected.get_field());
}

#[test]
fn derives_final_query_challenge_from_hashed_transcript_events() {
    let constant_root = root(2);
    let public_values = values(&[3, 4, 5]);
    let witness_roots = vec![root(10), root(20), root(30)];
    let evaluations = vec![ext(40), ext(50), ext(60)];
    let final_polynomial = vec![ext(70), ext(80)];

    let actual = derive_pcs_final_query_challenge(PcsTranscriptInputs {
        arity: 4,
        hash_values: true,
        constant_root,
        public_values: &public_values,
        witness_roots: &witness_roots,
        root_challenge_draws: &[2, 1, 1],
        evaluation_values: &evaluations,
        evaluation_challenge_draws: 2,
        fri_roots: &[],
        final_polynomial: &final_polynomial,
    })
    .expect("challenge should derive");

    let mut expected = PoseidonTranscript::new(4).expect("arity should be supported");
    expected.put(&constant_root);
    absorb_commit_values(&mut expected, 4, true, &public_values)
        .expect("public values should absorb");
    put_root_and_draw(&mut expected, &witness_roots[0], 2);
    put_root_and_draw(&mut expected, &witness_roots[1], 1);
    put_root_and_draw(&mut expected, &witness_roots[2], 1);
    absorb_commit_values(&mut expected, 4, true, &flatten_ext(&evaluations))
        .expect("evaluations should absorb");
    draw(&mut expected, 2);
    absorb_commit_values(&mut expected, 4, true, &flatten_ext(&final_polynomial))
        .expect("final polynomial should absorb");

    assert_eq!(actual, expected.get_field());
}

#[test]
fn rejects_root_challenge_draw_mismatches() {
    assert_eq!(
        derive_pcs_final_query_challenge(PcsTranscriptInputs {
            arity: 4,
            hash_values: false,
            constant_root: root(1),
            public_values: &[],
            witness_roots: &[root(10)],
            root_challenge_draws: &[1, 2],
            evaluation_values: &[],
            evaluation_challenge_draws: 0,
            fri_roots: &[],
            final_polynomial: &[ext(20)],
        }),
        Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: 1,
            draw_count: 2
        })
    );
}

#[test]
fn rejects_empty_final_polynomials() {
    assert_eq!(
        derive_pcs_final_query_challenge(PcsTranscriptInputs {
            arity: 4,
            hash_values: false,
            constant_root: root(1),
            public_values: &[],
            witness_roots: &[],
            root_challenge_draws: &[],
            evaluation_values: &[],
            evaluation_challenge_draws: 0,
            fri_roots: &[],
            final_polynomial: &[],
        }),
        Err(PcsTranscriptError::EmptyFinalPolynomial)
    );
}

fn root(seed: u64) -> [Felt; 4] {
    [
        Felt::from_u64(seed),
        Felt::from_u64(seed + 1),
        Felt::from_u64(seed + 2),
        Felt::from_u64(seed + 3),
    ]
}

fn ext(seed: u64) -> Ext3 {
    Ext3::from_u64s([seed, seed + 1, seed + 2])
}

fn flatten_ext(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn put_root_and_draw(transcript: &mut PoseidonTranscript, root: &[Felt; 4], count: usize) {
    transcript.put(root);
    draw(transcript, count);
}

fn draw(transcript: &mut PoseidonTranscript, count: usize) {
    for _ in 0..count {
        transcript.get_field();
    }
}
