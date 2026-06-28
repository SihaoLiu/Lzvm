/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.ChallengeSegmentBinding
import Lzvm.OpeningSegmentBinding

/-!
Runtime query plan binding obligations.
-/

namespace Lzvm

universe uDigest

structure RuntimeQueryPlanBindingValidation (system : VerifierModel) where
  challengeValidation : RuntimeChallengeSegmentBindingValidation system
  openingValidation : RuntimeOpeningSegmentBindingValidation system
  queryPlanBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanSegmentCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanTranscriptInputsCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanMaterialManifestMatchesSchedule : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanDerivedFromTranscript : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanMatchesOpenedArtifacts : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanSeedBindsWitnessTreeDigests : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanSeededFriOpeningRequirementsChecked : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanBindingAcceptedImpliesChallengeAccepted :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        challengeValidation.challengeBindingAccepted artifact publicInput proof
  queryPlanBindingAcceptedImpliesOpeningAccepted :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        openingValidation.openingSegmentBindingAccepted artifact publicInput proof
  queryPlanBindingAcceptedImpliesSegmentCanonical :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanSegmentCanonical artifact publicInput proof
  queryPlanBindingAcceptedImpliesTranscriptInputsCanonical :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanTranscriptInputsCanonical artifact publicInput proof
  queryPlanBindingAcceptedImpliesMaterialManifestMatchesSchedule :
    forall artifact publicInput proof, queryPlanBindingAccepted artifact publicInput proof ->
      queryPlanMaterialManifestMatchesSchedule artifact publicInput proof
  queryPlanBindingAcceptedImpliesDerivedFromTranscript :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanDerivedFromTranscript artifact publicInput proof
  queryPlanBindingAcceptedImpliesMatchesOpenedArtifacts :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanMatchesOpenedArtifacts artifact publicInput proof
  queryPlanBindingAcceptedImpliesSeedBindsWitnessTreeDigests :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof
  queryPlanBindingAcceptedImpliesSeededFriOpeningRequirementsChecked :
    forall artifact publicInput proof,
      queryPlanBindingAccepted artifact publicInput proof ->
        queryPlanSeededFriOpeningRequirementsChecked artifact publicInput proof
  queryPlanChecksImplyTranscriptQueryPlanBound :
    forall artifact publicInput proof,
      queryPlanSegmentCanonical artifact publicInput proof ->
        queryPlanTranscriptInputsCanonical artifact publicInput proof ->
          queryPlanDerivedFromTranscript artifact publicInput proof ->
            challengeValidation.transcriptValidation.queryPlanBound artifact publicInput proof
  queryPlanChecksImplyOpeningQueryPlanBound :
    forall artifact publicInput proof,
      queryPlanSegmentCanonical artifact publicInput proof ->
        queryPlanMatchesOpenedArtifacts artifact publicInput proof ->
          openingValidation.queryPlanBound artifact publicInput proof

def RuntimeQueryPlanBindingBoundContract
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanSegmentCanonical artifact publicInput proof
    /\ validation.queryPlanTranscriptInputsCanonical artifact publicInput proof
    /\ validation.queryPlanDerivedFromTranscript artifact publicInput proof
    /\ validation.queryPlanMatchesOpenedArtifacts artifact publicInput proof
    /\ validation.challengeValidation.transcriptValidation.queryPlanBound
      artifact
      publicInput
      proof
    /\ validation.openingValidation.queryPlanBound artifact publicInput proof

def RuntimeQueryPlanBindingSeededContract (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system) (artifact : RuntimeArtifact)
    (publicInput : PublicInput) (proof : Proof) : Prop :=
  validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof
    /\ validation.queryPlanSeededFriOpeningRequirementsChecked artifact publicInput proof

def RuntimeQueryPlanMaterialManifestContract (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system) (artifact : RuntimeArtifact)
    (publicInput : PublicInput) (proof : Proof) : Prop :=
  validation.queryPlanSegmentCanonical artifact publicInput proof /\
    validation.queryPlanMaterialManifestMatchesSchedule artifact publicInput proof

def RuntimeQueryPlanBindingEvidence (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system) (artifact : RuntimeArtifact)
    (publicInput : PublicInput) (proof : Proof) : Prop :=
  RuntimeQueryPlanBindingBoundContract _system validation artifact publicInput proof
    /\ RuntimeQueryPlanMaterialManifestContract _system validation artifact publicInput proof
    /\ RuntimeQueryPlanBindingSeededContract _system validation artifact publicInput proof

def RuntimeQueryPlanBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBindingAccepted artifact publicInput proof

theorem runtime_query_plan_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
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
          proof := by
  intro artifact publicInput proof accepted
  have segmentCanonical :=
    validation.queryPlanBindingAcceptedImpliesSegmentCanonical
      artifact
      publicInput
      proof
      accepted
  have transcriptInputsCanonical :=
    validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical
      artifact
      publicInput
      proof
      accepted
  have derivedFromTranscript :=
    validation.queryPlanBindingAcceptedImpliesDerivedFromTranscript
      artifact
      publicInput
      proof
      accepted
  have matchesOpenedArtifacts :=
    validation.queryPlanBindingAcceptedImpliesMatchesOpenedArtifacts
      artifact
      publicInput
      proof
      accepted
  have transcriptQueryPlanBound :=
    validation.queryPlanChecksImplyTranscriptQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      transcriptInputsCanonical
      derivedFromTranscript
  have openingQueryPlanBound :=
    validation.queryPlanChecksImplyOpeningQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      matchesOpenedArtifacts
  have seedBindsWitnessTreeDigests :=
    validation.queryPlanBindingAcceptedImpliesSeedBindsWitnessTreeDigests
      artifact
      publicInput
      proof
      accepted
  have seededFriOpeningRequirementsChecked :=
    validation.queryPlanBindingAcceptedImpliesSeededFriOpeningRequirementsChecked
      artifact
      publicInput
      proof
      accepted
  have seededContract :
      RuntimeQueryPlanBindingSeededContract
        system
        validation
        artifact
        publicInput
        proof :=
    And.intro
      seedBindsWitnessTreeDigests
      seededFriOpeningRequirementsChecked
  exact
    And.intro
      (And.intro segmentCanonical
        (And.intro transcriptInputsCanonical
          (And.intro derivedFromTranscript
            (And.intro matchesOpenedArtifacts
              (And.intro transcriptQueryPlanBound openingQueryPlanBound)))))
      (And.intro
        (And.intro segmentCanonical
          (validation.queryPlanBindingAcceptedImpliesMaterialManifestMatchesSchedule
            _ _ _ accepted))
        seededContract)

theorem runtime_query_plan_binding_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
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
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.left

theorem runtime_query_plan_binding_evidence_implies_transcript_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeValidation.transcriptValidation.queryPlanBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_segmentCanonical,
      _transcriptInputsCanonical,
      _derivedFromTranscript,
      _matchesOpenedArtifacts,
      transcriptQueryPlanBound,
      _openingQueryPlanBound⟩
  exact transcriptQueryPlanBound

theorem runtime_query_plan_binding_evidence_implies_opening_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.openingValidation.queryPlanBound artifact publicInput proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_segmentCanonical,
      _transcriptInputsCanonical,
      _derivedFromTranscript,
      _matchesOpenedArtifacts,
      _transcriptQueryPlanBound,
      openingQueryPlanBound⟩
  exact openingQueryPlanBound

theorem runtime_query_plan_binding_evidence_implies_transcript_inputs_canonical
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanTranscriptInputsCanonical artifact publicInput proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_segmentCanonical,
      transcriptInputsCanonical,
      _derivedFromTranscript,
      _matchesOpenedArtifacts,
      _transcriptQueryPlanBound,
      _openingQueryPlanBound⟩
  exact transcriptInputsCanonical

theorem runtime_query_plan_binding_evidence_implies_seeded_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence
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
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.right.right

theorem runtime_query_plan_binding_evidence_implies_material_manifest_contract
    {system : VerifierModel} (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingEvidence system validation artifact publicInput proof ->
        RuntimeQueryPlanMaterialManifestContract system validation artifact publicInput proof := by
  intro _artifact _publicInput _proof evidence
  exact evidence.right.left

theorem runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingSeededContract
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof := by
  intro artifact publicInput proof seeded
  exact seeded.left

theorem runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingSeededContract
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanSeededFriOpeningRequirementsChecked
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof seeded
  exact seeded.right

theorem runtime_query_plan_binding_checked_acceptance_challenge
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeChallengeSegmentBindingCheckedAcceptance
          system
          validation.challengeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.queryPlanBindingAcceptedImpliesChallengeAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_query_plan_binding_checked_acceptance_artifact_finalized
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactFinalized
          system
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_artifact_finalized
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

theorem runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactBindingStructuralObligations
          system
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have artifactFinalized :=
    runtime_query_plan_binding_checked_acceptance_artifact_finalized
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_proof_artifact_finalized_structural_obligations
      validation.challengeValidation.transcriptValidation.artifactBindingValidation
      artifact
      publicInput
      proof
      artifactFinalized

theorem runtime_query_plan_binding_checked_acceptance_opening
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.queryPlanBindingAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_query_plan_binding_checked_acceptance_bound_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
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
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_query_plan_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.challengeValidation.transcriptValidation.queryPlanBound
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have segmentCanonical :=
    validation.queryPlanBindingAcceptedImpliesSegmentCanonical
      artifact
      publicInput
      proof
      accepted
  have transcriptInputsCanonical :=
    validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical
      artifact
      publicInput
      proof
      accepted
  have derivedFromTranscript :=
    validation.queryPlanBindingAcceptedImpliesDerivedFromTranscript
      artifact
      publicInput
      proof
      accepted
  exact
    validation.queryPlanChecksImplyTranscriptQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      transcriptInputsCanonical
      derivedFromTranscript

theorem runtime_query_plan_binding_checked_acceptance_transcript_inputs_canonical
    {system : VerifierModel} (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance system validation artifact publicInput proof ->
        validation.queryPlanTranscriptInputsCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical _ _ _ accepted

theorem runtime_query_plan_binding_checked_acceptance_material_manifest_contract
    {system : VerifierModel} (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance system validation artifact publicInput proof ->
        RuntimeQueryPlanMaterialManifestContract system validation artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact ⟨validation.queryPlanBindingAcceptedImpliesSegmentCanonical _ _ _ accepted,
    validation.queryPlanBindingAcceptedImpliesMaterialManifestMatchesSchedule _ _ _ accepted⟩

theorem runtime_query_plan_binding_checked_acceptance_opening_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.openingValidation.queryPlanBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have segmentCanonical :=
    validation.queryPlanBindingAcceptedImpliesSegmentCanonical
      artifact
      publicInput
      proof
      accepted
  have matchesOpenedArtifacts :=
    validation.queryPlanBindingAcceptedImpliesMatchesOpenedArtifacts
      artifact
      publicInput
      proof
      accepted
  exact
    validation.queryPlanChecksImplyOpeningQueryPlanBound
      artifact
      publicInput
      proof
      segmentCanonical
      matchesOpenedArtifacts

theorem runtime_query_plan_binding_checked_acceptance_seeded_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
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
          proof := by
  intro artifact publicInput proof accepted
  have seedBindsWitnessTreeDigests :=
    validation.queryPlanBindingAcceptedImpliesSeedBindsWitnessTreeDigests
      artifact
      publicInput
      proof
      accepted
  have seededFriOpeningRequirementsChecked :=
    validation.queryPlanBindingAcceptedImpliesSeededFriOpeningRequirementsChecked
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro seedBindsWitnessTreeDigests seededFriOpeningRequirementsChecked

theorem runtime_query_plan_binding_checked_acceptance_seed_binds_witness_tree_digests
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have seeded :=
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests
      validation
      artifact
      publicInput
      proof
      seeded

theorem runtime_query_plan_binding_checked_acceptance_seeded_fri_opening_requirements_checked
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanSeededFriOpeningRequirementsChecked
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have seeded :=
    runtime_query_plan_binding_checked_acceptance_seeded_contract
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked
      validation
      artifact
      publicInput
      proof
      seeded

theorem runtime_query_plan_binding_checked_acceptance_segment_ids_unique
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentIdsUnique artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.left

theorem runtime_query_plan_binding_checked_acceptance_unit_values_trace_identity_coverage
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.right.right

theorem runtime_query_plan_binding_checked_acceptance_container_canonical
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofContainerCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.left

theorem runtime_query_plan_binding_checked_acceptance_metadata_canonical
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofMetadataCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.left

theorem runtime_query_plan_binding_checked_acceptance_segment_payloads_nonempty
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.left

theorem runtime_query_plan_binding_checked_acceptance_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentIdsAllowed artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.right.right.left

theorem runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system)
    (binding :
      RuntimeProofArtifactConcreteSegmentIdBinding
        validation.challengeValidation.transcriptValidation.artifactBindingValidation) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeProofArtifactConcreteSegmentIdsAllowed proof := by
  intro artifact publicInput proof accepted
  have challengeAccepted :=
    validation.queryPlanBindingAcceptedImpliesChallengeAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed
      validation.challengeValidation
      binding
      artifact
      publicInput
      proof
      challengeAccepted

theorem runtime_query_plan_binding_checked_acceptance_segments_present
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        let artifactValidation :=
          validation.challengeValidation.transcriptValidation.artifactBindingValidation
        artifactValidation.proofSegmentsPresent artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have artifactStructural :=
    runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations
      validation
      artifact
      publicInput
      proof
      accepted
  exact artifactStructural.right.right.left

theorem runtime_query_plan_binding_checked_acceptance_opening_segment_evidence
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation.openingValidation
      artifact
      publicInput
      proof
      openingAccepted

theorem runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have openingSegmentEvidence :=
    runtime_query_plan_binding_checked_acceptance_opening_segment_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation.openingValidation
      artifact
      publicInput
      proof
      openingSegmentEvidence

theorem runtime_query_plan_binding_checked_acceptance_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_pcs_and_fri
      validation.openingValidation
      artifact
      publicInput
      proof
      openingAccepted


end Lzvm
