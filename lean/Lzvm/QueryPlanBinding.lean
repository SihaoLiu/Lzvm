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

def RuntimeQueryPlanBindingSeededContract
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof
    /\ validation.queryPlanSeededFriOpeningRequirementsChecked artifact publicInput proof

def RuntimeQueryPlanBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeQueryPlanBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeQueryPlanBindingBoundContract
    _system
    validation
    artifact
    publicInput
    proof
    /\ RuntimeQueryPlanBindingSeededContract
      _system
      validation
      artifact
      publicInput
      proof

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
      seededContract

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
  exact evidence.right

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
    {system : VerifierModel}
    (validation : RuntimeQueryPlanBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanTranscriptInputsCanonical artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical
      artifact
      publicInput
      proof
      accepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_container_canonical
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_metadata_canonical
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_segment_payloads_nonempty
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_segment_ids_allowed
      validation.challengeValidation
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
  have challengeAccepted :=
    runtime_query_plan_binding_checked_acceptance_challenge
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_challenge_segment_binding_checked_acceptance_segments_present
      validation.challengeValidation
      artifact
      publicInput
      proof
      challengeAccepted

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
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  exact
    And.intro queryPlanEvidence
      (And.intro challengeSound.left
        (And.intro openingFull.left
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
  have openingAccepted :=
    runtime_query_plan_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingFull :=
    runtime_opening_segment_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  rcases openingFull with
    ⟨_openingSegmentBoundFromFull,
      _openingEvidenceFromFull,
      openingBound,
      _pcsOpeningsFromFull,
      _friQueriesFromFull,
      _openingCoreFromFull,
      _soundWitnessFromFull⟩
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
