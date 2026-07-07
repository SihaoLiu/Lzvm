/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningValidation
import Lzvm.RuntimeExternalSource

/-!
Required external source evidence for guarded runtime acceptance.
-/

namespace Lzvm

universe uDigest

theorem runtime_guarded_external_source_required_evidence
    {system : VerifierModel}
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          ExternalSourceOpeningEvidence system sourceValidation publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    external_source_opening_requirement_implies_evidence
      sourceValidation
      publicInput
      proof
      requiresExternalSource
      checked.right
      required

theorem runtime_guarded_external_source_required_provider_obligations
    {system : VerifierModel}
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          sourceValidation.providerTranscriptBound publicInput proof
            /\ sourceValidation.providerMatchesCommittedTrace publicInput proof
            /\ sourceValidation.providerOpeningsRootBound publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have externalEvidence :=
    runtime_guarded_external_source_required_evidence
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro
      (external_source_opening_evidence_provider_transcript_bound
        sourceValidation
        publicInput
        proof
        externalEvidence)
      (And.intro
        (external_source_opening_evidence_provider_matches_committed_trace
          sourceValidation
          publicInput
          proof
          externalEvidence)
        (external_source_opening_evidence_provider_openings_root_bound
          sourceValidation
          publicInput
          proof
          externalEvidence))

theorem runtime_guarded_external_source_required_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have artifactAccepted := checked.left
  have artifactEvidence :=
    runtime_artifact_checked_acceptance_evidence
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have externalEvidence :=
    runtime_guarded_external_source_required_evidence
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have pcsOpenings :=
    external_source_opening_evidence_implies_pcs_openings
      sourceValidation
      publicInput
      proof
      externalEvidence
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      runtimeValidation
      artifact
      publicInput
      proof
      artifactAccepted
  have soundWitness :=
    abstract_verifier_sound assumptions publicInput proof verifierAccepts
  exact
    And.intro artifactEvidence
      (And.intro externalEvidence (And.intro pcsOpenings soundWitness))

theorem runtime_guarded_external_source_required_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  exact
    runtime_guarded_external_source_checked_acceptance_verifier_core_contract
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked

theorem runtime_guarded_external_source_required_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have requiredSound :=
    runtime_guarded_external_source_required_sound
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have coreContract :=
    runtime_guarded_external_source_required_verifier_core_contract
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro requiredSound.left
      (And.intro requiredSound.right.left
        (And.intro requiredSound.right.right.left
          (And.intro coreContract requiredSound.right.right.right)))

theorem runtime_guarded_external_source_required_evidence_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeArtifactEvidence
                system
                runtimeValidation
                artifact
                publicInput
                proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked required
  have auditedAssumptions :=
    assumption_bundle_carries_required_evidence assumptions
  have contracts :=
    runtime_guarded_external_source_required_evidence_core_and_sound
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  exact
    And.intro auditedAssumptions.left
      (And.intro auditedAssumptions.right contracts)

set_option linter.style.longLine false in
theorem runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening
    {Digest : Type uDigest}
    {system : VerifierModel}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (openingValidation : RuntimeOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningCheckedAcceptance
          system
          openingValidation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked openingAccepted required
  have externalEvidence :=
    external_source_opening_requirement_implies_evidence
      sourceValidation
      publicInput
      proof
      requiresExternalSource
      checked.right
      required
  have pcsAndFri :=
    runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
      hashAssumptions
      openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      openingAccepted
  exact And.intro externalEvidence pcsAndFri

set_option linter.style.longLine false in
theorem runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (openingValidation : RuntimeOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningCheckedAcceptance
          system
          openingValidation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked openingAccepted required
  exact
    runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening
      assumptions.crypto.hashCollisionResistance
      runtimeValidation
      sourceValidation
      openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      openingAccepted
      required

set_option linter.style.longLine false in
theorem runtime_guarded_external_source_required_hash_concrete_opening_sound
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (openingValidation : RuntimeOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningCheckedAcceptance
          system
          openingValidation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked openingAccepted required
  have requiredContract :=
    runtime_guarded_external_source_required_evidence_core_and_sound
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have concretePcsFri :=
    runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening
      hashAssumptions
      runtimeValidation
      sourceValidation
      openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      openingAccepted
      required
  exact
    And.intro requiredContract.left
      (And.intro concretePcsFri.left
        (And.intro concretePcsFri.right.left
          (And.intro concretePcsFri.right.right
            (And.intro
              requiredContract.right.right.right.left
              requiredContract.right.right.right.right))))

set_option linter.style.longLine false in
theorem runtime_guarded_external_source_required_audited_hash_concrete_opening_sound
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (openingValidation : RuntimeOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningCheckedAcceptance
          system
          openingValidation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked openingAccepted required
  have requiredContract :=
    runtime_guarded_external_source_required_evidence_core_and_sound
      assumptions
      runtimeValidation
      sourceValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      required
  have concretePcsFri :=
    runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening
      assumptions
      runtimeValidation
      sourceValidation
      openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      openingAccepted
      required
  exact
    And.intro requiredContract.left
      (And.intro concretePcsFri.left
        (And.intro concretePcsFri.right.left
          (And.intro concretePcsFri.right.right
            (And.intro
              requiredContract.right.right.right.left
              requiredContract.right.right.right.right))))

set_option linter.style.longLine false in
theorem runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (runtimeValidation : RuntimeConformanceValidation system)
    (sourceValidation : ExternalSourceOpeningValidation system)
    (openingValidation : RuntimeOpeningValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeGuardedExternalSourceCheckedAcceptance
          system
          runtimeValidation
          sourceValidation
          artifact
          publicInput
          proof
          requiresExternalSource ->
        RuntimeOpeningCheckedAcceptance
          system
          openingValidation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RequiredCryptographicAssumptionStatements assumptions.crypto
            /\ RequiredSemanticAssumptionStatements assumptions.semantic
            /\ RuntimeArtifactEvidence
              system
              runtimeValidation
              artifact
              publicInput
              proof
            /\ ExternalSourceOpeningEvidence system sourceValidation publicInput proof
            /\ system.pcsOpeningsValid publicInput proof
            /\ system.friQueriesValid publicInput proof
            /\ RuntimeVerifierCoreContract system publicInput proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource checked openingAccepted required
  have auditedEvidence :=
    assumption_bundle_carries_required_evidence assumptions
  have sound :=
    runtime_guarded_external_source_required_audited_hash_concrete_opening_sound
      assumptions
      runtimeValidation
      sourceValidation
      openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      requiresExternalSource
      checked
      openingAccepted
      required
  exact
    And.intro auditedEvidence.left
      (And.intro auditedEvidence.right sound)

end Lzvm
