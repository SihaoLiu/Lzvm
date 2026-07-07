/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.RetainedParentCheckpointOpening.Core

/-!
Compact retained parent checkpoint opening contracts.
-/

namespace Lzvm

universe uDigest

set_option linter.style.longLine false in
theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have batchAccepted :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  have batchSound :=
    runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening
      assumptions
      hashAssumptions
      validation.batchRowsValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      batchAccepted
  exact And.intro evidence batchSound.right

set_option linter.style.longLine false in
theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
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
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening
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

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have evidence :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have batchAccepted :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  have batchSound :=
    runtime_batch_witness_opening_rows_checked_acceptance_sound
      assumptions
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      batchAccepted
  exact And.intro evidence batchSound.right

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  have batchAccepted :=
    validation.retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract
      assumptions
      validation.batchRowsValidation
      artifact
      publicInput
      proof
      batchAccepted

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_opening_checked_acceptance_digest_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract
            validation
            artifact
            publicInput
            proof
            accepted)
          (And.intro
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted)
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted))))

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningEvidence
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have checkedSound :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have openingAndCore :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract
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
            (And.intro openingAndCore.right.right.right.left
              (And.intro openingAndCore.right.right.right.right checkedSound.right)))))

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeRetainedParentCheckpointOpeningEvidence
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have checkedSound :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have openingAndCore :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        (And.intro checkedSound.left
          (And.intro openingAndCore.left
            (And.intro openingAndCore.right.left
              (And.intro openingAndCore.right.right.left
                (And.intro openingAndCore.right.right.right.left
                  (And.intro
                    openingAndCore.right.right.right.right
                    checkedSound.right)))))))

theorem
  runtime_retained_parent_checkpoint_concrete_path_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : Digest -> Digest -> Digest}
    (centralized :
      CentralizedMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract
            validation
            artifact
            publicInput
            proof
            accepted)
          (And.intro
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted)
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted))))

theorem
  runtime_retained_parent_checkpoint_nary_path_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcretePathBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract
            validation
            artifact
            publicInput
            proof
            accepted)
          (And.intro
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted)
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted))))

theorem
  runtime_retained_parent_checkpoint_nary_opening_opening_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
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
          /\ RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningPrefixBatchContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence
        assumptions
        validation
        artifact
        publicInput
        proof
        requiresExternalSource
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle
          assumptions
          validation
          centralized
          binding
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract
            validation
            artifact
            publicInput
            proof
            accepted)
          (And.intro
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted)
            (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
              assumptions
              validation
              artifact
              publicInput
              proof
              accepted))))

theorem
  runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle
        assumptions
        validation
        centralized
        binding
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)
          (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
            assumptions
            validation
            artifact
            publicInput
            proof
            accepted)))

theorem
  runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
        system
        validation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sourceCore :=
    runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle
      assumptions
      validation
      centralized
      binding
      artifact
      publicInput
      proof
      accepted
  have sound :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound
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
theorem
  runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_concrete_opening_bundle
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system)
    {Digest : Type uDigest}
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (binding :
      RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding
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
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningDigestContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof accepted
  have sourceCore :=
    runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle
      assumptions
      validation
      centralized
      binding
      artifact
      publicInput
      proof
      accepted
  have sound :=
    runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle
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

theorem runtime_retained_parent_checkpoint_opening_checked_acceptance_source_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeRetainedParentCheckpointOpeningValidation system) :
    forall artifact publicInput proof,
      RuntimeRetainedParentCheckpointOpeningCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeRetainedParentCheckpointOpeningSourceContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeRetainedParentCheckpointOpeningRetainedRowsContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract
          assumptions
          validation
          artifact
          publicInput
          proof
          accepted))

end Lzvm
