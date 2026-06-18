/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RetainedLeafDigestOpening

/-!
Compact source/core contracts for retained leaf digest opening checks.
-/

namespace Lzvm

universe uDigest

theorem runtime_retained_leaf_digest_nary_opening_position_bound_from_hash_assumption
    {system : VerifierModel}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.retainedLeafDigestPathBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision
      validation
      binding
      (Eq.mp
        centralized
        hashAssumptions.merkleHashCollisionResistance.evidence)
      artifact
      publicInput
      proof
      accepted

theorem runtime_retained_leaf_digest_nary_opening_digest_contract_from_hash_assumption
    {system : VerifierModel}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningDigestContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have levelAvailable :=
    validation.retainedLeafDigestOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have pathBound :=
    runtime_retained_leaf_digest_nary_opening_position_bound_from_hash_assumption
      hashAssumptions
      validation
      centralized
      binding
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro levelAvailable
      (And.intro pathBound
        (And.intro rootMatches
          (And.intro rowsFromSource rowsBoundToQueryPlan)))

theorem runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have batchAccepted :=
    validation.retainedLeafDigestOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  have batchEvidence :=
    runtime_batch_witness_opening_rows_checked_acceptance_evidence
      assumptions
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchAccepted
  have queryPlanBound :=
    validation.batchRowsValidation.batchWitnessOpeningRowsAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      batchAccepted
  have levelAvailable :=
    validation.retainedLeafDigestOpeningAcceptedImpliesLevelAvailable
      artifact
      publicInput
      proof
      accepted
  have pathBound :=
    runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle
      assumptions
      validation
      centralized
      binding
      artifact
      publicInput
      proof
      accepted
  have rootMatches :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot
      artifact
      publicInput
      proof
      accepted
  have rowsFromSource :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsFromSource
      artifact
      publicInput
      proof
      accepted
  have rowsBoundToQueryPlan :=
    validation.retainedLeafDigestOpeningAcceptedImpliesRowsBoundToQueryPlan
      artifact
      publicInput
      proof
      accepted
  have retainedPerRow :=
    validation.retainedLeafDigestChecksImplyPerRowWitnessOpeningRowsBound
      artifact
      publicInput
      proof
      queryPlanBound
      rowsBoundToQueryPlan
      rowsFromSource
      pathBound
      rootMatches
  let segmentValidation := validation.batchRowsValidation.openingSegmentValidation
  let openingValidation := segmentValidation.openingValidation
  have witnessSegments :=
    validation.batchRowsValidation.perRowWitnessOpeningRowsImplyWitnessOpeningSegmentsValid
      artifact
      publicInput
      proof
      queryPlanBound
      retainedPerRow
  have witnessOpeningsBound : openingValidation.witnessOpeningsBound
      artifact
      publicInput
      proof :=
    segmentValidation.openingSegmentChecksImplyWitnessOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      witnessSegments
  exact
    And.intro batchEvidence
      (And.intro
        (And.intro levelAvailable
          (And.intro pathBound
            (And.intro rootMatches
              (And.intro rowsFromSource rowsBoundToQueryPlan))))
        (And.intro rowsBoundToQueryPlan
          (And.intro rowsFromSource
            (And.intro retainedPerRow
              (And.intro witnessSegments witnessOpeningsBound)))))

theorem runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle
        assumptions
        validation
        centralized
        binding
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
            validation
            artifact
            publicInput
            proof
            accepted)
        (And.intro
          (runtime_retained_leaf_digest_opening_evidence_implies_retained_rows_contract
            validation
            artifact
            publicInput
            proof
            False
            (runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle
              assumptions
              validation
              centralized
              binding
              artifact
              publicInput
              proof
              False
              accepted))
          (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sourceCore :=
    runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle
      assumptions
      validation
      centralized
      binding
      artifact
      publicInput
      proof
      accepted
  have sound :=
    runtime_retained_leaf_digest_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  exact
    And.intro sourceCore.left
      (And.intro sourceCore.right.left
        (And.intro sourceCore.right.right.left
          (And.intro sourceCore.right.right.right sound.right)))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_source_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedLeafDigestOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted))

end Lzvm
