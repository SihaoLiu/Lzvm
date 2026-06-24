/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AssumptionAudit

/-!
Composition theorem for the abstract Lzvm verifier soundness model.
-/

namespace Lzvm

theorem abstract_verifier_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    ProofSystemSound system := by
  intro publicInput proof accepted
  have transcriptBound :=
    assumptions.crypto.transcript_binding publicInput proof accepted
  have publicInputBound :=
    assumptions.semantic.public_input_binding publicInput proof accepted
  have pcsOpeningsValid :=
    assumptions.crypto.pcs_opening_sound publicInput proof accepted
  have friQueriesValid :=
    assumptions.crypto.fri_query_sound publicInput proof accepted
  cases assumptions.semantic.trace_extraction publicInput proof accepted with
  | intro trace traceConsistent =>
    cases assumptions.semantic.constraint_satisfaction
        publicInput proof trace accepted traceConsistent with
    | intro constraints constraintsSatisfied =>
      cases assumptions.semantic.witness_extraction
          publicInput
          proof
          trace
          constraints
          accepted
          publicInputBound
          traceConsistent
          constraintsSatisfied with
      | intro witness witnessMatchesTrace =>
        exact
          Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpeningsValid
                      (And.intro friQueriesValid
                        (And.intro traceConsistent
                          (And.intro constraintsSatisfied witnessMatchesTrace))))))))

theorem abstract_verifier_sound_with_audited_assumptions
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ ProofSystemSound system := by
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (abstract_verifier_sound assumptions)

theorem abstract_verifier_sound_with_audited_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredCryptographicAssumptionStatements assumptions.crypto
      /\ RequiredSemanticAssumptionStatements assumptions.semantic
      /\ ProofSystemSound system := by
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (abstract_verifier_sound assumptions))

end Lzvm
