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
  have coreContract :=
    assumption_bundle_verifier_core_contract
      assumptions
      publicInput
      proof
      accepted
  rcases coreContract with
    ⟨transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  cases assumption_bundle_trace_extraction assumptions publicInput proof accepted with
  | intro trace traceConsistent =>
    cases assumption_bundle_constraint_satisfaction
        assumptions
        publicInput proof trace accepted traceConsistent with
    | intro constraints constraintsSatisfied =>
      cases assumption_bundle_witness_extraction
          assumptions
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

theorem abstract_verifier_sound_with_semantic_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    RequiredSemanticAssumptionStatements assumptions.semantic
      /\ ProofSystemSound system := by
  exact
    And.intro
      (assumption_bundle_carries_required_semantic_evidence assumptions)
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

theorem accepted_proof_audited_core_and_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof accepted
  have coreContract :=
    assumption_bundle_verifier_core_contract
      assumptions
      publicInput
      proof
      accepted
  have soundWitness :=
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro coreContract soundWitness))

theorem accepted_proof_crypto_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (assumption_bundle_verifier_core_contract
        assumptions
        publicInput
        proof
        accepted)

theorem accepted_proof_semantic_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have soundWitness :=
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      accepted
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro
      (assumption_bundle_carries_required_semantic_evidence assumptions)
      (Exists.intro witness
        (Exists.intro trace
          (Exists.intro constraints
            (And.intro traceConsistent
              (And.intro constraintsSatisfied witnessMatchesTrace)))))

theorem accepted_proof_audited_core_execution_and_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  have soundWitness :=
    abstract_verifier_sound
      assumptions
      publicInput
      proof
      accepted
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticExecution with
    ⟨semanticEvidence, executionObligations⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      coreContract,
      executionObligations,
      soundWitness⟩

theorem accepted_proof_audited_core_and_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have cryptoCore :=
    accepted_proof_crypto_core_contract
      assumptions
      publicInput
      proof
      accepted
  have semanticExecution :=
    accepted_proof_semantic_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  rcases cryptoCore with
    ⟨cryptoEvidence, coreContract⟩
  rcases semanticExecution with
    ⟨semanticEvidence, executionObligations⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      coreContract,
      executionObligations⟩

theorem accepted_proof_audited_full_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            RuntimeVerifierCoreContract system publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedCoreAndWitness :=
    accepted_proof_audited_core_and_sound_witness
      assumptions
      publicInput
      proof
      accepted
  rcases auditedCoreAndWitness with
    ⟨cryptoEvidence, semanticEvidence, coreContract, soundWitness⟩
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (Exists.intro witness
          (Exists.intro trace
            (Exists.intro constraints
              (And.intro coreContract
                (And.intro traceConsistent
                  (And.intro constraintsSatisfied witnessMatchesTrace)))))))

theorem accepted_proof_audited_sound_witness_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedCoreAndWitness :=
    accepted_proof_audited_core_and_sound_witness
      assumptions
      publicInput
      proof
      accepted
  rcases auditedCoreAndWitness with
    ⟨cryptoEvidence, semanticEvidence, _coreContract, soundWitness⟩
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (Exists.intro witness
          (Exists.intro trace
            (Exists.intro constraints
              (And.intro transcriptBound
                (And.intro publicInputBound
                  (And.intro pcsOpeningsValid
                    (And.intro friQueriesValid
                      (And.intro traceConsistent
                        (And.intro constraintsSatisfied witnessMatchesTrace))))))))))

theorem accepted_proof_audited_core_and_sound_witness_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedCoreAndWitness :=
    accepted_proof_audited_core_and_sound_witness
      assumptions
      publicInput
      proof
      accepted
  rcases auditedCoreAndWitness with
    ⟨cryptoEvidence, semanticEvidence, coreContract, soundWitness⟩
  rcases soundWitness with
    ⟨witness,
      trace,
      constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro coreContract
          (Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpeningsValid
                      (And.intro friQueriesValid
                        (And.intro traceConsistent
                          (And.intro constraintsSatisfied witnessMatchesTrace)))))))))))

theorem accepted_proof_audited_proof_system_and_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.transcriptBound publicInput proof
              /\ system.publicInputBound publicInput proof
              /\ system.pcsOpeningsValid publicInput proof
              /\ system.friQueriesValid publicInput proof
              /\ system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have components :=
    accepted_proof_audited_core_and_sound_witness_components
      assumptions
      publicInput
      proof
      accepted
  rcases components with
    ⟨cryptoEvidence, semanticEvidence, coreContract, witnessComponents⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro (abstract_verifier_sound assumptions)
          (And.intro coreContract witnessComponents)))

theorem accepted_proof_audited_proof_system_core_and_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedCoreExecution :=
    accepted_proof_audited_core_and_execution_obligations
      assumptions
      publicInput
      proof
      accepted
  have proofSystemSound := abstract_verifier_sound assumptions
  have coreExecution :=
    proof_system_sound_accepts_core_contract_and_execution_obligations
      proofSystemSound
      publicInput
      proof
      accepted
  rcases auditedCoreExecution with
    ⟨cryptoEvidence, semanticEvidence, _coreContract, _executionObligations⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      proofSystemSound,
      coreExecution.left,
      coreExecution.right⟩

theorem accepted_proof_audited_flat_proof_system_components
    {system : VerifierModel}
    (assumptions : AssumptionBundle system) :
    forall publicInput proof,
      system.accepts publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ ProofSystemSound system
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace := by
  intro publicInput proof accepted
  have auditedComponents :=
    accepted_proof_audited_proof_system_and_components
      assumptions
      publicInput
      proof
      accepted
  rcases auditedComponents with
    ⟨cryptoEvidence,
      semanticEvidence,
      proofSystemSound,
      coreContract,
      witnessComponents⟩
  rcases coreContract with
    ⟨transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  rcases witnessComponents with
    ⟨witness,
      trace,
      constraints,
      _soundTranscriptBound,
      _soundPublicInputBound,
      _soundPcsOpeningsValid,
      _soundFriQueriesValid,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    ⟨cryptoEvidence,
      semanticEvidence,
      proofSystemSound,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      witness,
      trace,
      constraints,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩

end Lzvm
