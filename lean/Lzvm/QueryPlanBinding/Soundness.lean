/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.QueryPlanBinding.Core

/-!
Runtime query plan binding soundness and core-contract obligations.
-/

namespace Lzvm

universe uDigest

theorem runtime_query_plan_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have queryPlanEvidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeSound :=
    runtime_challenge_segment_binding_checked_acceptance_sound
      assumptions
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  have openingFull :=
    runtime_opening_segment_binding_checked_acceptance_full_soundness_contract
      assumptions validation.openingValidation artifact publicInput proof requiresExternalSource
      openingAccepted
  have openingSegmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation.openingValidation artifact publicInput proof openingAccepted
  exact
    And.intro queryPlanEvidence
      (And.intro challengeSound.left
        (And.intro openingSegmentEvidence
          (And.intro openingFull.right.left
            (And.intro challengeSound.right.right.left
              (And.intro openingFull.right.right.right.left
                (And.intro openingFull.right.right.right.right.left
                  openingFull.right.right.right.right.right.right))))))

set_option linter.style.longLine false in
theorem runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeQueryPlanBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have queryPlanEvidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have challengeSound :=
    runtime_challenge_segment_binding_checked_acceptance_sound
      assumptions
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted
  have openingSound :=
    runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have pcsAndFri :=
    runtime_opening_evidence_implies_pcs_and_fri
      validation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingSound.right.left
  exact
    And.intro queryPlanEvidence
      (And.intro challengeSound.left
        (And.intro openingSound.left
          (And.intro openingSound.right.left
            (And.intro challengeSound.right.right.left
              (And.intro pcsAndFri.left
                (And.intro pcsAndFri.right openingSound.right.right))))))

set_option linter.style.longLine false in
theorem runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      assumptions.crypto.hashCollisionResistance
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted

theorem runtime_query_plan_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      _requiresExternalSource
      openingAccepted

theorem runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have sound :=
    runtime_query_plan_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have core :=
    runtime_query_plan_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro sound.left
      (And.intro sound.right.left
        (And.intro sound.right.right.left
          (And.intro sound.right.right.right.left
            (And.intro sound.right.right.right.right.left
              (And.intro sound.right.right.right.right.right.left
                (And.intro sound.right.right.right.right.right.right.left
                  (And.intro core sound.right.right.right.right.right.right.right)))))))

theorem runtime_query_plan_binding_checked_acceptance_opening_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have queryPlanEvidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanBound :=
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      queryPlanEvidence
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAndCore :=
    runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  rcases openingAndCore with
    ⟨openingSegmentBound, openingEvidence, _openingCoreContract⟩
  have coreContract :=
    runtime_query_plan_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases coreContract with
    ⟨transcriptBound, publicInputBound, pcsOpeningsValid, friQueriesValid⟩
  exact
    And.intro queryPlanBound
      (And.intro openingSegmentBound
        (And.intro openingEvidence
          (And.intro transcriptBound
            (And.intro pcsOpeningsValid
              (And.intro friQueriesValid
                (And.intro transcriptBound
                  (And.intro publicInputBound
                    (And.intro pcsOpeningsValid friQueriesValid))))))))

theorem runtime_query_plan_binding_checked_acceptance_full_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeQueryPlanBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeChallengeSegmentBindingEvidence
            system
            validation.challengeValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have sound :=
    runtime_query_plan_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases sound with
    ⟨queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      soundWitness⟩
  have queryPlanBound :=
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      queryPlanEvidence
  have openingSegmentBound :=
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentEvidence
  have openingBound :=
    runtime_opening_evidence_implies_bound_contract
      validation.openingValidation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingEvidence
  have coreContract :=
    runtime_query_plan_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro queryPlanEvidence
      (And.intro queryPlanBound
        (And.intro challengeEvidence
          (And.intro openingSegmentEvidence
            (And.intro openingSegmentBound
              (And.intro openingEvidence
                (And.intro openingBound
                  (And.intro transcriptBound
                    (And.intro pcsOpeningsValid
                      (And.intro friQueriesValid
                        (And.intro coreContract soundWitness))))))))))

theorem runtime_query_plan_binding_checked_acceptance_seeded_opening_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingSeededContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeQueryPlanBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have seededContract :=
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAndCore :=
    runtime_query_plan_binding_checked_acceptance_opening_and_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact And.intro seededContract openingAndCore

set_option linter.style.longLine false in
theorem runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeQueryPlanBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingSeededContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeQueryPlanBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have seededContract :=
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have concreteSound :=
    runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases concreteSound with
    ⟨queryPlanEvidence,
      _challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      _soundWitness⟩
  have queryPlanBound :=
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      queryPlanEvidence
  have openingSegmentBound :=
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentEvidence
  have coreContract :=
    runtime_query_plan_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases coreContract with
    ⟨coreTranscriptBound, publicInputBound, corePcsOpeningsValid, coreFriQueriesValid⟩
  exact
    And.intro seededContract
      (And.intro queryPlanBound
        (And.intro openingSegmentBound
          (And.intro openingEvidence
            (And.intro transcriptBound
              (And.intro publicInputBound
                (And.intro pcsOpeningsValid
                  (And.intro friQueriesValid
                    (And.intro coreTranscriptBound
                      (And.intro publicInputBound
                        (And.intro corePcsOpeningsValid coreFriQueriesValid))))))))))

set_option linter.style.longLine false in
theorem runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeQueryPlanBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingSeededContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeQueryPlanBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningSegmentBindingBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have seededContract :=
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have concreteSound :=
    runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases concreteSound with
    ⟨queryPlanEvidence,
      _challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      pcsOpeningsValid,
      friQueriesValid,
      _soundWitness⟩
  have queryPlanBound :=
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      queryPlanEvidence
  have openingSegmentBound :=
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentEvidence
  have coreContract :=
    runtime_query_plan_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  rcases coreContract with
    ⟨coreTranscriptBound, publicInputBound, corePcsOpeningsValid, coreFriQueriesValid⟩
  exact
    And.intro seededContract
      (And.intro queryPlanBound
        (And.intro openingSegmentBound
          (And.intro openingEvidence
            (And.intro transcriptBound
              (And.intro publicInputBound
                (And.intro pcsOpeningsValid
                  (And.intro friQueriesValid
                    (And.intro coreTranscriptBound
                      (And.intro publicInputBound
                        (And.intro corePcsOpeningsValid coreFriQueriesValid))))))))))

end Lzvm
