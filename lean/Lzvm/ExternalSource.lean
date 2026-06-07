/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

/-!
External source opening obligations for compact witness commitments.
-/

namespace Lzvm

/-!
An external source provider may be used to answer openings without storing the
host trace inside the commitment object. This model keeps the cryptographic
obligations explicit: the provider must be bound to the proof transcript, its
values must match the committed trace source, and the produced openings must
bind to the commitment roots.
-/

structure ExternalSourceOpeningValidation (system : VerifierModel) where
  providerTranscriptBound : PublicInput -> Proof -> Prop
  providerMatchesCommittedTrace : PublicInput -> Proof -> Prop
  providerOpeningsRootBound : PublicInput -> Proof -> Prop
  providerEvidenceImpliesPcsOpenings :
    forall publicInput proof,
      providerTranscriptBound publicInput proof ->
        providerMatchesCommittedTrace publicInput proof ->
          providerOpeningsRootBound publicInput proof ->
            system.pcsOpeningsValid publicInput proof

def ExternalSourceOpeningEvidence
    (system : VerifierModel)
    (validation : ExternalSourceOpeningValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.providerTranscriptBound publicInput proof
    /\ validation.providerMatchesCommittedTrace publicInput proof
    /\ validation.providerOpeningsRootBound publicInput proof

def ExternalSourceOpeningRequirement
    (system : VerifierModel)
    (validation : ExternalSourceOpeningValidation system)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  requiresExternalSource ->
    ExternalSourceOpeningEvidence system validation publicInput proof

def ExternalSourceOpeningCheckedAcceptance
    (system : VerifierModel)
    (validation : ExternalSourceOpeningValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  system.accepts publicInput proof
    /\ ExternalSourceOpeningEvidence system validation publicInput proof

def ExternalSourceOpeningSoundnessObligations
    (system : VerifierModel)
    (validation : ExternalSourceOpeningValidation system)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  ExternalSourceOpeningEvidence system validation publicInput proof
    /\ RuntimeVerifierCoreContract system publicInput proof

theorem external_source_opening_evidence_implies_pcs_openings
    {system : VerifierModel}
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof,
      ExternalSourceOpeningEvidence system validation publicInput proof ->
        system.pcsOpeningsValid publicInput proof := by
  intro publicInput proof evidence
  exact
    validation.providerEvidenceImpliesPcsOpenings
      publicInput
      proof
      evidence.left
      evidence.right.left
      evidence.right.right

theorem external_source_opening_requirement_from_evidence
    {system : VerifierModel}
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof requiresExternalSource,
      ExternalSourceOpeningEvidence system validation publicInput proof ->
        ExternalSourceOpeningRequirement
          system
          validation
          publicInput
          proof
          requiresExternalSource := by
  intro publicInput proof requiresExternalSource evidence _
  exact evidence

theorem external_source_opening_requirement_not_required
    {system : VerifierModel}
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof requiresExternalSource,
      Not requiresExternalSource ->
        ExternalSourceOpeningRequirement
          system
          validation
          publicInput
          proof
          requiresExternalSource := by
  intro publicInput proof requiresExternalSource notRequired required
  exact False.elim (notRequired required)

theorem external_source_opening_requirement_implies_evidence
    {system : VerifierModel}
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof requiresExternalSource,
      ExternalSourceOpeningRequirement
          system
          validation
          publicInput
          proof
          requiresExternalSource ->
        requiresExternalSource ->
          ExternalSourceOpeningEvidence system validation publicInput proof := by
  intro publicInput proof requiresExternalSource requirement required
  exact requirement required

theorem external_source_opening_checked_acceptance_implies_pcs_openings
    {system : VerifierModel}
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof,
      ExternalSourceOpeningCheckedAcceptance system validation publicInput proof ->
        system.pcsOpeningsValid publicInput proof := by
  intro publicInput proof acceptedWithExternalSource
  exact
    external_source_opening_evidence_implies_pcs_openings
      validation
      publicInput
      proof
      acceptedWithExternalSource.right

theorem external_source_opening_checked_acceptance_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof,
      ExternalSourceOpeningCheckedAcceptance system validation publicInput proof ->
        ExternalSourceOpeningSoundnessObligations
          system
          validation
          publicInput
          proof := by
  intro publicInput proof acceptedWithExternalSource
  have accepted := acceptedWithExternalSource.left
  have evidence := acceptedWithExternalSource.right
  have transcriptBound :=
    assumptions.crypto.transcript_binding publicInput proof accepted
  have publicInputBound :=
    assumptions.semantic.public_input_binding publicInput proof accepted
  have pcsOpenings :=
    external_source_opening_checked_acceptance_implies_pcs_openings
      validation
      publicInput
      proof
      acceptedWithExternalSource
  have friQueries :=
    assumptions.crypto.fri_query_sound publicInput proof accepted
  exact
    And.intro evidence
      (And.intro transcriptBound
        (And.intro publicInputBound
          (And.intro pcsOpenings friQueries)))

theorem external_source_opening_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof,
      ExternalSourceOpeningCheckedAcceptance system validation publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof acceptedWithExternalSource
  exact
    (external_source_opening_checked_acceptance_obligations
      assumptions
      validation
      publicInput
      proof
      acceptedWithExternalSource).right

theorem external_source_opening_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : ExternalSourceOpeningValidation system) :
    forall publicInput proof,
      ExternalSourceOpeningCheckedAcceptance system validation publicInput proof ->
        ExternalSourceOpeningEvidence system validation publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithExternalSource
  have evidence := acceptedWithExternalSource.right
  have pcsOpenings :=
    external_source_opening_checked_acceptance_implies_pcs_openings
      validation
      publicInput
      proof
      acceptedWithExternalSource
  exact
    And.intro evidence
      (And.intro pcsOpenings
        (abstract_verifier_sound
          assumptions
          publicInput
          proof
          acceptedWithExternalSource.left))

end Lzvm
