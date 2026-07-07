/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RetainedLeafDigestOpening.Core

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
        (HashCollisionResistanceAssumption.merkle_hash_collision_resistance
          hashAssumptions))
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
  have traceIdentities :=
    runtime_opening_segment_binding_evidence_implies_trace_identities_match
      segmentValidation
      artifact
      publicInput
      proof
      batchEvidence.left
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
      traceIdentities
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

set_option linter.style.longLine false in
theorem runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_concrete_opening_bundle
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
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.batchRowsValidation.openingSegmentValidation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.batchRowsValidation.openingSegmentValidation.openingValidation
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
    runtime_retained_leaf_digest_opening_checked_acceptance_sound_from_concrete_nary_merkle
      assumptions
      validation
      centralized
      constantBinding
      witnessBinding
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

theorem runtime_retained_leaf_digest_opening_checked_acceptance_opening_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_opening_checked_acceptance_digest_contract
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
            accepted)))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
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
            requiresExternalSource
          /\ RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  have checkedSound :=
    runtime_retained_leaf_digest_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have openingAndCore :=
    runtime_retained_leaf_digest_opening_checked_acceptance_opening_and_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro checkedSound.left
      (And.intro openingAndCore.left
        (And.intro openingAndCore.right.left
          (And.intro openingAndCore.right.right.left
            (And.intro openingAndCore.right.right.right checkedSound.right))))

theorem runtime_retained_leaf_digest_opening_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedLeafDigestOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeRetainedLeafDigestOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  have contracts :=
    runtime_retained_leaf_digest_opening_checked_acceptance_evidence_core_and_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right contracts)

theorem runtime_retained_leaf_digest_concrete_path_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedLeafDigestOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedLeafDigestConcretePathBinding
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
        RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
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
            accepted)))

theorem runtime_retained_leaf_digest_nary_path_opening_and_core_contract_from_bundle
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
      RuntimeRetainedLeafDigestNAryConcretePathBinding
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
        RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
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
            accepted)))

theorem runtime_retained_leaf_digest_nary_opening_opening_and_core_contract_from_bundle
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
        RuntimeOpeningEvidence
            system
            validation.batchRowsValidation.openingSegmentValidation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeRetainedLeafDigestOpeningDigestContract
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
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
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
            accepted)))

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
