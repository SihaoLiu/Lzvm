/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.GuestReportMemoryAccess
import Lzvm.GuestReportRegisterWrite
import Lzvm.Soundness

/-!
Runtime conformance bridge for checked proof artifacts.
-/

namespace Lzvm

/-!
The checker-facing artifact model is intentionally small. Concrete Rust and
CUDA checks should later discharge the validation predicates below, while the
Lean theorem records how accepted runtime artifacts enter the abstract verifier
soundness model.
-/

structure RuntimeArtifact where
  id : Nat
deriving DecidableEq, Repr

structure RuntimeGuestReportStorageEvidence where
  registerDestination : GuestRegisterWriteDestination
  compactRegisterWrite : CompactGuestRegisterWrite
  registerWrites : List GuestRegisterWrite
  memoryStorage : GuestMemoryAccessStorage
  normalMemoryAccesses : List GuestMemoryAccess
  precompileMemoryAccesses : List GuestMemoryAccess
  precompileResult : Option Nat
  registerCanonical :
    CompactGuestRegisterWriteCanonical
      registerDestination
      compactRegisterWrite
      registerWrites
  memoryCanonical :
    FoldedGuestMemoryEffectsCanonical
      memoryStorage
      normalMemoryAccesses
      precompileMemoryAccesses
      precompileResult

def RuntimeGuestReportStorageLogicalViews
    (evidence : RuntimeGuestReportStorageEvidence) : Prop :=
  evidence.registerWrites =
      reconstructGuestRegisterWrites
        evidence.registerDestination
        evidence.compactRegisterWrite
    /\ evidence.memoryStorage.normalAccesses = evidence.normalMemoryAccesses
    /\ evidence.memoryStorage.precompileAccesses = evidence.precompileMemoryAccesses
    /\ evidence.memoryStorage.precompileResult = evidence.precompileResult

theorem runtime_guest_report_storage_evidence_register_writes
    (evidence : RuntimeGuestReportStorageEvidence) :
    evidence.registerWrites =
      reconstructGuestRegisterWrites
        evidence.registerDestination
        evidence.compactRegisterWrite := by
  exact evidence.registerCanonical

theorem runtime_guest_report_storage_evidence_register_write_length_le_one
    (evidence : RuntimeGuestReportStorageEvidence) :
    evidence.registerWrites.length <= 1 := by
  exact
    compact_guest_register_write_canonical_length_le_one
      evidence.registerCanonical

theorem runtime_guest_report_storage_evidence_normal_memory_accesses
    (evidence : RuntimeGuestReportStorageEvidence) :
    evidence.memoryStorage.normalAccesses = evidence.normalMemoryAccesses := by
  exact evidence.memoryCanonical.left

theorem runtime_guest_report_storage_evidence_precompile_memory_accesses
    (evidence : RuntimeGuestReportStorageEvidence) :
    evidence.memoryStorage.precompileAccesses =
      evidence.precompileMemoryAccesses := by
  exact evidence.memoryCanonical.right.left

theorem runtime_guest_report_storage_evidence_precompile_result
    (evidence : RuntimeGuestReportStorageEvidence) :
    evidence.memoryStorage.precompileResult = evidence.precompileResult := by
  exact evidence.memoryCanonical.right.right

theorem runtime_guest_report_storage_evidence_logical_views
    (evidence : RuntimeGuestReportStorageEvidence) :
    RuntimeGuestReportStorageLogicalViews evidence := by
  exact
    And.intro
      (runtime_guest_report_storage_evidence_register_writes evidence)
      (And.intro
        (runtime_guest_report_storage_evidence_normal_memory_accesses evidence)
        (And.intro
          (runtime_guest_report_storage_evidence_precompile_memory_accesses evidence)
          (runtime_guest_report_storage_evidence_precompile_result evidence)))

structure RuntimeConformanceValidation (system : VerifierModel) where
  artifactAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactPublicInputMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactProofMatches : RuntimeArtifact -> PublicInput -> Proof -> Prop
  artifactAcceptedImpliesVerifierAccepts :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        system.accepts publicInput proof
  artifactAcceptedImpliesPublicInputMatches :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        artifactPublicInputMatches artifact publicInput proof
  artifactAcceptedImpliesProofMatches :
    forall artifact publicInput proof,
      artifactAccepted artifact publicInput proof ->
        artifactProofMatches artifact publicInput proof

def RuntimeConformanceValidationAgreement
    {system : VerifierModel}
    (left right : RuntimeConformanceValidation system) : Prop :=
  (forall artifact publicInput proof,
      left.artifactAccepted artifact publicInput proof <->
        right.artifactAccepted artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.artifactPublicInputMatches artifact publicInput proof <->
        right.artifactPublicInputMatches artifact publicInput proof)
    /\ (forall artifact publicInput proof,
      left.artifactProofMatches artifact publicInput proof <->
        right.artifactProofMatches artifact publicInput proof)

def RuntimeArtifactEvidence
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactPublicInputMatches artifact publicInput proof
    /\ validation.artifactProofMatches artifact publicInput proof

def RuntimeArtifactCheckedAcceptance
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.artifactAccepted artifact publicInput proof

def RuntimeArtifactSoundnessObligations
    (system : VerifierModel)
    (validation : RuntimeConformanceValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeArtifactEvidence system validation artifact publicInput proof
    /\ system.accepts publicInput proof
    /\ RuntimeVerifierCoreContract system publicInput proof

theorem runtime_conformance_agreement_checked_acceptance_iff
    {system : VerifierModel}
    {left right : RuntimeConformanceValidation system}
    (agreement : RuntimeConformanceValidationAgreement left right) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system left artifact publicInput proof <->
        RuntimeArtifactCheckedAcceptance system right artifact publicInput proof := by
  intro artifact publicInput proof
  exact agreement.left artifact publicInput proof

theorem runtime_conformance_agreement_evidence_iff
    {system : VerifierModel}
    {left right : RuntimeConformanceValidation system}
    (agreement : RuntimeConformanceValidationAgreement left right) :
    forall artifact publicInput proof,
      RuntimeArtifactEvidence system left artifact publicInput proof <->
        RuntimeArtifactEvidence system right artifact publicInput proof := by
  intro artifact publicInput proof
  constructor
  · intro evidence
    exact
      And.intro
        ((agreement.right.left artifact publicInput proof).mp evidence.left)
        ((agreement.right.right artifact publicInput proof).mp evidence.right)
  · intro evidence
    exact
      And.intro
        ((agreement.right.left artifact publicInput proof).mpr evidence.left)
        ((agreement.right.right artifact publicInput proof).mpr evidence.right)

theorem runtime_artifact_checked_acceptance_implies_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  exact
    validation.artifactAcceptedImpliesVerifierAccepts
      artifact
      publicInput
      proof
      artifactAccepted

theorem runtime_artifact_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactEvidence system validation artifact publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  exact
    And.intro
      (validation.artifactAcceptedImpliesPublicInputMatches
        artifact
        publicInput
        proof
        artifactAccepted)
      (validation.artifactAcceptedImpliesProofMatches
        artifact
        publicInput
        proof
        artifactAccepted)

theorem runtime_artifact_checked_acceptance_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof artifactAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have evidence :=
    runtime_artifact_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have transcriptBound :=
    assumption_bundle_fiat_shamir_transcript_binding
      assumptions
      publicInput
      proof
      verifierAccepts
  have publicInputBound :=
    assumption_bundle_public_input_binding
      assumptions
      publicInput
      proof
      verifierAccepts
  have pcsOpenings :=
    assumption_bundle_pcs_opening_soundness
      assumptions
      publicInput
      proof
      verifierAccepts
  have friQueries :=
    assumption_bundle_fri_query_soundness
      assumptions
      publicInput
      proof
      verifierAccepts
  exact
    And.intro evidence
      (And.intro verifierAccepts
        (And.intro transcriptBound
          (And.intro publicInputBound
            (And.intro pcsOpenings friQueries))))

theorem runtime_artifact_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation
          artifact
          publicInput
          proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have obligations :=
    runtime_artifact_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have verifierAccepts := obligations.right.left
  exact
    And.intro obligations
      (abstract_verifier_sound assumptions publicInput proof verifierAccepts)

theorem runtime_artifact_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have obligations :=
    runtime_artifact_checked_acceptance_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  exact obligations.right.right

theorem runtime_artifact_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeArtifactEvidence system validation artifact publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have checkedSound :=
    runtime_artifact_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    And.intro checkedSound.left.left
      (And.intro checkedSound.left.right.right checkedSound.right)

theorem runtime_artifact_checked_acceptance_audited_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeArtifactSoundnessObligations
            system
            validation
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have checkedSound :=
    runtime_artifact_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    And.intro
      (assumption_bundle_carries_required_crypto_evidence assumptions)
      (And.intro
        (assumption_bundle_carries_required_semantic_evidence assumptions)
        checkedSound)

theorem runtime_artifact_checked_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeConformanceValidation system) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeArtifactEvidence system validation artifact publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof artifactAccepted
  have audited :=
    runtime_artifact_checked_acceptance_audited_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      artifactAccepted
  have obligations := audited.right.right.left
  exact
    And.intro
      audited.left
      (And.intro
        audited.right.left
        (And.intro
          obligations.left
          (And.intro obligations.right.right audited.right.right.right)))

theorem runtime_conformance_agreement_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (left right : RuntimeConformanceValidation system)
    (agreement : RuntimeConformanceValidationAgreement left right) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system left artifact publicInput proof ->
        RuntimeArtifactSoundnessObligations
          system
          right
          artifact
          publicInput
          proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof leftAccepted
  have rightAccepted :
      RuntimeArtifactCheckedAcceptance system right artifact publicInput proof :=
    (runtime_conformance_agreement_checked_acceptance_iff
      agreement
      artifact
      publicInput
      proof).mp leftAccepted
  exact
    runtime_artifact_checked_acceptance_sound
      assumptions
      right
      artifact
      publicInput
      proof
      rightAccepted

theorem runtime_conformance_agreement_checked_acceptance_audited_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (left right : RuntimeConformanceValidation system)
    (agreement : RuntimeConformanceValidationAgreement left right) :
    forall artifact publicInput proof,
      RuntimeArtifactCheckedAcceptance system left artifact publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeArtifactSoundnessObligations
            system
            right
            artifact
            publicInput
            proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof leftAccepted
  have rightAccepted :
      RuntimeArtifactCheckedAcceptance system right artifact publicInput proof :=
    (runtime_conformance_agreement_checked_acceptance_iff
      agreement
      artifact
      publicInput
      proof).mp leftAccepted
  exact
    runtime_artifact_checked_acceptance_audited_sound
      assumptions
      right
      artifact
      publicInput
      proof
      rightAccepted

end Lzvm
