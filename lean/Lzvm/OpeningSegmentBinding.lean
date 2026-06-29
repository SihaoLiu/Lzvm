/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningValidation

/-!
Runtime opening segment binding obligations.
-/

namespace Lzvm

universe uDigest

structure RuntimeOpeningSegmentBindingValidation (system : VerifierModel) where
  openingValidation : RuntimeOpeningValidation system
  openingSegmentBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryPlanBound : RuntimeArtifact -> PublicInput -> Proof -> Prop
  openingUnitTraceIdentitiesMatch : RuntimeArtifact -> PublicInput -> Proof -> Prop
  openingUnitTraceIdentityCoverageExact : RuntimeArtifact -> PublicInput -> Proof -> Prop
  constantOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  witnessOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  witnessOpeningStageOrderPreserved : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friOpeningSegmentsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friFoldsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friFoldQueryPlanOrderPreserved : RuntimeArtifact -> PublicInput -> Proof -> Prop
  verifierQueryOutputsValid : RuntimeArtifact -> PublicInput -> Proof -> Prop
  openingSegmentBindingAcceptedImpliesOpeningAccepted :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        openingValidation.openingAccepted artifact publicInput proof
  openingSegmentBindingAcceptedImpliesQueryPlanBound :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        queryPlanBound artifact publicInput proof
  openingSegmentBindingAcceptedImpliesTraceIdentitiesMatch :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        openingUnitTraceIdentitiesMatch artifact publicInput proof
  openingSegmentBindingAcceptedImpliesTraceIdentityCoverageExact :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        openingUnitTraceIdentityCoverageExact artifact publicInput proof
  openingSegmentBindingAcceptedImpliesConstantOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        constantOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesWitnessOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        witnessOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesWitnessOpeningStageOrderPreserved :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        witnessOpeningStageOrderPreserved artifact publicInput proof
  openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        friOpeningSegmentsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesFriFoldsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        friFoldsValid artifact publicInput proof
  openingSegmentBindingAcceptedImpliesFriFoldQueryPlanOrderPreserved :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        friFoldQueryPlanOrderPreserved artifact publicInput proof
  openingSegmentBindingAcceptedImpliesVerifierQueryOutputsValid :
    forall artifact publicInput proof,
      openingSegmentBindingAccepted artifact publicInput proof ->
        verifierQueryOutputsValid artifact publicInput proof
  openingSegmentChecksImplyConstantOpeningsBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        constantOpeningSegmentsValid artifact publicInput proof ->
          openingUnitTraceIdentitiesMatch artifact publicInput proof ->
          openingValidation.constantOpeningsBound artifact publicInput proof
  openingSegmentChecksImplyWitnessOpeningsBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        witnessOpeningSegmentsValid artifact publicInput proof ->
          openingUnitTraceIdentitiesMatch artifact publicInput proof ->
          openingValidation.witnessOpeningsBound artifact publicInput proof
  openingSegmentChecksImplyFriOpeningBound :
    forall artifact publicInput proof,
      queryPlanBound artifact publicInput proof ->
        friOpeningSegmentsValid artifact publicInput proof ->
          friFoldsValid artifact publicInput proof ->
            verifierQueryOutputsValid artifact publicInput proof ->
              openingUnitTraceIdentitiesMatch artifact publicInput proof ->
              openingValidation.friOpeningBound artifact publicInput proof

def RuntimeOpeningSegmentBindingBoundContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBound artifact publicInput proof
    /\ validation.openingUnitTraceIdentitiesMatch artifact publicInput proof
    /\ validation.constantOpeningSegmentsValid artifact publicInput proof
    /\ validation.witnessOpeningSegmentsValid artifact publicInput proof
    /\ validation.friOpeningSegmentsValid artifact publicInput proof
    /\ validation.friFoldsValid artifact publicInput proof
    /\ validation.verifierQueryOutputsValid artifact publicInput proof
    /\ validation.openingValidation.constantOpeningsBound artifact publicInput proof
    /\ validation.openingValidation.witnessOpeningsBound artifact publicInput proof
    /\ validation.openingValidation.friOpeningBound artifact publicInput proof

def RuntimeOpeningSegmentExactIdentityContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBound artifact publicInput proof
    /\ validation.openingUnitTraceIdentitiesMatch artifact publicInput proof
    /\ validation.openingUnitTraceIdentityCoverageExact artifact publicInput proof

def RuntimeOpeningSegmentBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  RuntimeOpeningSegmentBindingBoundContract
    _system
    validation
    artifact
    publicInput
    proof
    /\ RuntimeOpeningSegmentExactIdentityContract
      _system
      validation
      artifact
      publicInput
      proof

def RuntimeFriFoldTraceIdentityContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBound artifact publicInput proof
    /\ validation.openingUnitTraceIdentitiesMatch artifact publicInput proof
    /\ validation.friFoldsValid artifact publicInput proof

def RuntimeFriFoldQueryPlanOrderContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.queryPlanBound artifact publicInput proof
    /\ validation.openingUnitTraceIdentitiesMatch artifact publicInput proof
    /\ validation.friFoldsValid artifact publicInput proof
    /\ validation.friFoldQueryPlanOrderPreserved artifact publicInput proof

def RuntimeWitnessOpeningStageOrderContract
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.witnessOpeningSegmentsValid artifact publicInput proof
    /\ validation.witnessOpeningStageOrderPreserved artifact publicInput proof

def RuntimeOpeningSegmentBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.openingSegmentBindingAccepted artifact publicInput proof

structure RuntimeFriOpeningSegmentParserBoundary
    (_system : VerifierModel)
    (validation : RuntimeOpeningSegmentBindingValidation _system) where
  supportedEncodingVersion : RuntimeArtifact -> PublicInput -> Proof -> Prop
  finalPolynomialValuesCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  queryValuesCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  layerDigestRootsCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  lastLevelDigestRootsCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  siblingDigestRootsCanonical : RuntimeArtifact -> PublicInput -> Proof -> Prop
  segmentLayoutWalked : RuntimeArtifact -> PublicInput -> Proof -> Prop
  friOpeningSegmentsValidImpliesSupportedEncodingVersion :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        supportedEncodingVersion artifact publicInput proof
  friOpeningSegmentsValidImpliesFinalPolynomialValuesCanonical :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        finalPolynomialValuesCanonical artifact publicInput proof
  friOpeningSegmentsValidImpliesQueryValuesCanonical :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        queryValuesCanonical artifact publicInput proof
  friOpeningSegmentsValidImpliesLayerDigestRootsCanonical :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        layerDigestRootsCanonical artifact publicInput proof
  friOpeningSegmentsValidImpliesLastLevelDigestRootsCanonical :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        lastLevelDigestRootsCanonical artifact publicInput proof
  friOpeningSegmentsValidImpliesSiblingDigestRootsCanonical :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        siblingDigestRootsCanonical artifact publicInput proof
  friOpeningSegmentsValidImpliesSegmentLayoutWalked :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        segmentLayoutWalked artifact publicInput proof

def RuntimeFriOpeningSegmentParserContract
    {system : VerifierModel}
    {validation : RuntimeOpeningSegmentBindingValidation system}
    (boundary : RuntimeFriOpeningSegmentParserBoundary system validation)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  boundary.supportedEncodingVersion artifact publicInput proof
    /\ boundary.finalPolynomialValuesCanonical artifact publicInput proof
    /\ boundary.queryValuesCanonical artifact publicInput proof
    /\ boundary.layerDigestRootsCanonical artifact publicInput proof
    /\ boundary.lastLevelDigestRootsCanonical artifact publicInput proof
    /\ boundary.siblingDigestRootsCanonical artifact publicInput proof
    /\ boundary.segmentLayoutWalked artifact publicInput proof

theorem runtime_opening_segment_binding_fri_segments_valid_parser_contract
    {system : VerifierModel}
    {validation : RuntimeOpeningSegmentBindingValidation system}
    (boundary : RuntimeFriOpeningSegmentParserBoundary system validation) :
    forall artifact publicInput proof,
      validation.friOpeningSegmentsValid artifact publicInput proof ->
        RuntimeFriOpeningSegmentParserContract
          boundary
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof friSegments
  exact
    And.intro
      (boundary.friOpeningSegmentsValidImpliesSupportedEncodingVersion
        artifact
        publicInput
        proof
        friSegments)
      (And.intro
        (boundary.friOpeningSegmentsValidImpliesFinalPolynomialValuesCanonical
          artifact
          publicInput
          proof
          friSegments)
        (And.intro
          (boundary.friOpeningSegmentsValidImpliesQueryValuesCanonical
            artifact
            publicInput
            proof
            friSegments)
          (And.intro
            (boundary.friOpeningSegmentsValidImpliesLayerDigestRootsCanonical
              artifact
              publicInput
              proof
              friSegments)
            (And.intro
              (boundary.friOpeningSegmentsValidImpliesLastLevelDigestRootsCanonical
                artifact
                publicInput
                proof
                friSegments)
              (And.intro
                (boundary.friOpeningSegmentsValidImpliesSiblingDigestRootsCanonical
                  artifact
                  publicInput
                  proof
                  friSegments)
                (boundary.friOpeningSegmentsValidImpliesSegmentLayoutWalked
                  artifact
                  publicInput
                  proof
                  friSegments))))))

theorem runtime_opening_segment_binding_checked_acceptance_evidence
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have queryPlanBound :=
    validation.openingSegmentBindingAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      accepted
  have traceIdentities :=
    validation.openingSegmentBindingAcceptedImpliesTraceIdentitiesMatch
      artifact
      publicInput
      proof
      accepted
  have constantSegments :=
    validation.openingSegmentBindingAcceptedImpliesConstantOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have witnessSegments :=
    validation.openingSegmentBindingAcceptedImpliesWitnessOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have friSegments :=
    validation.openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  have friFolds :=
    validation.openingSegmentBindingAcceptedImpliesFriFoldsValid
      artifact
      publicInput
      proof
      accepted
  have verifierQueries :=
    validation.openingSegmentBindingAcceptedImpliesVerifierQueryOutputsValid
      artifact
      publicInput
      proof
      accepted
  have constantBound :=
    validation.openingSegmentChecksImplyConstantOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      constantSegments
      traceIdentities
  have witnessBound :=
    validation.openingSegmentChecksImplyWitnessOpeningsBound
      artifact
      publicInput
      proof
      queryPlanBound
      witnessSegments
      traceIdentities
  have friOpeningBound :=
    validation.openingSegmentChecksImplyFriOpeningBound
      artifact
      publicInput
      proof
      queryPlanBound
      friSegments
      friFolds
      verifierQueries
      traceIdentities
  have boundContract :
      RuntimeOpeningSegmentBindingBoundContract system validation artifact publicInput proof :=
    And.intro queryPlanBound
      (And.intro traceIdentities
        (And.intro constantSegments
          (And.intro witnessSegments
            (And.intro friSegments
              (And.intro friFolds
                (And.intro verifierQueries
                  (And.intro constantBound
                    (And.intro witnessBound friOpeningBound))))))))
  exact
    And.intro boundContract
      (And.intro queryPlanBound
        (And.intro traceIdentities
          (validation.openingSegmentBindingAcceptedImpliesTraceIdentityCoverageExact
            _ _ _ accepted)))

theorem runtime_opening_segment_binding_evidence_implies_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  exact evidence.left

theorem runtime_opening_segment_binding_evidence_implies_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanBound artifact publicInput proof := by
  intro artifact publicInput proof evidence
  exact evidence.left.left

theorem runtime_opening_segment_binding_evidence_implies_trace_identities_match
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.openingUnitTraceIdentitiesMatch artifact publicInput proof := by
  intro artifact publicInput proof evidence
  exact evidence.left.right.left

theorem runtime_opening_segment_binding_evidence_implies_exact_identity_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence system validation artifact publicInput proof ->
        RuntimeOpeningSegmentExactIdentityContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro _artifact _publicInput _proof evidence
  exact evidence.right

theorem runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.openingUnitTraceIdentityCoverageExact
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.openingSegmentBindingAcceptedImpliesTraceIdentityCoverageExact
      artifact
      publicInput
      proof
      accepted

theorem runtime_opening_segment_binding_checked_acceptance_exact_identity_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentExactIdentityContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.openingSegmentBindingAcceptedImpliesQueryPlanBound
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.openingSegmentBindingAcceptedImpliesTraceIdentitiesMatch
          artifact
          publicInput
          proof
          accepted)
        (runtime_opening_segment_binding_checked_acceptance_trace_identity_coverage_exact
          validation
          artifact
          publicInput
          proof
          accepted))

theorem runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldTraceIdentityContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨queryPlanBound, traceIdentitiesMatch, _, _, _, friFoldsValid, _, _, _, _⟩
  exact
    And.intro queryPlanBound
      (And.intro traceIdentitiesMatch friFoldsValid)

theorem runtime_opening_segment_binding_evidence_implies_fri_opening_checks
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        validation.friOpeningSegmentsValid artifact publicInput proof
          /\ validation.friFoldsValid artifact publicInput proof
          /\ validation.verifierQueryOutputsValid artifact publicInput proof
          /\ validation.openingValidation.friOpeningBound artifact publicInput proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_queryPlanBound,
      _traceIdentities,
      _constantSegments,
      _witnessSegments,
      friSegments,
      friFolds,
      verifierQueries,
      _constantBound,
      _witnessBound,
      friOpeningBound⟩
  exact
    And.intro friSegments
      (And.intro friFolds
        (And.intro verifierQueries friOpeningBound))

theorem runtime_opening_segment_binding_evidence_implies_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_queryPlanBound,
      _traceIdentities,
      _constantSegments,
      _witnessSegments,
      _friSegments,
      _friFolds,
      _verifierQueries,
      constantBound,
      witnessBound,
      friOpeningBound⟩
  have pcsOpenings :=
    validation.openingValidation.openingChecksImplyPcsOpeningsValid
      artifact
      publicInput
      proof
      constantBound
      witnessBound
      friOpeningBound
  have friQueries :=
    validation.openingValidation.friOpeningImpliesFriQueriesValid
      artifact
      publicInput
      proof
      friOpeningBound
  exact And.intro pcsOpenings friQueries

theorem runtime_opening_segment_binding_evidence_implies_opening_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingEvidence
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningBoundContract
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof evidence
  rcases evidence.left with
    ⟨_queryPlanBound,
      _traceIdentities,
      _constantSegments,
      _witnessSegments,
      _friSegments,
      _friFolds,
      _verifierQueries,
      constantBound,
      witnessBound,
      friOpeningBound⟩
  exact And.intro constantBound (And.intro witnessBound friOpeningBound)

theorem runtime_opening_segment_binding_checked_acceptance_opening
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningCheckedAcceptance
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.openingSegmentBindingAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_opening_segment_binding_checked_acceptance_query_plan_bound
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.queryPlanBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    validation.openingSegmentBindingAcceptedImpliesQueryPlanBound
      artifact
      publicInput
      proof
      accepted

theorem runtime_opening_segment_binding_checked_acceptance_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_bound_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningBoundContract
          system
          validation.openingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_opening_bound_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_fri_opening_checks
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        validation.friOpeningSegmentsValid artifact publicInput proof
          /\ validation.friFoldsValid artifact publicInput proof
          /\ validation.verifierQueryOutputsValid artifact publicInput proof
          /\ validation.openingValidation.friOpeningBound artifact publicInput proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_fri_opening_checks
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldTraceIdentityContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_fri_fold_trace_identity_contract
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_fri_fold_query_plan_order_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriFoldQueryPlanOrderContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.openingSegmentBindingAcceptedImpliesQueryPlanBound
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (validation.openingSegmentBindingAcceptedImpliesTraceIdentitiesMatch
          artifact
          publicInput
          proof
          accepted)
        (And.intro
          (validation.openingSegmentBindingAcceptedImpliesFriFoldsValid
            artifact
            publicInput
            proof
            accepted)
          (validation.openingSegmentBindingAcceptedImpliesFriFoldQueryPlanOrderPreserved
            artifact
            publicInput
            proof
            accepted)))

theorem runtime_opening_segment_binding_checked_acceptance_witness_stage_order_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeWitnessOpeningStageOrderContract
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.openingSegmentBindingAcceptedImpliesWitnessOpeningSegmentsValid
        artifact
        publicInput
        proof
        accepted)
      (validation.openingSegmentBindingAcceptedImpliesWitnessOpeningStageOrderPreserved
        artifact
        publicInput
        proof
        accepted)

theorem runtime_opening_segment_binding_checked_acceptance_fri_parser_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (boundary : RuntimeFriOpeningSegmentParserBoundary system validation) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeFriOpeningSegmentParserContract
          boundary
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have friSegments :=
    validation.openingSegmentBindingAcceptedImpliesFriOpeningSegmentsValid
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_fri_segments_valid_parser_contract
      boundary
      artifact
      publicInput
      proof
      friSegments

theorem runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have evidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_segment_binding_evidence_implies_pcs_and_fri
      validation
      artifact
      publicInput
      proof
      evidence

theorem runtime_opening_segment_binding_checked_acceptance_pcs_and_fri
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_without_assumptions
      validation
      artifact
      publicInput
      proof
      accepted

theorem runtime_opening_segment_binding_checked_acceptance_opening_pcs_fri_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningCheckedAcceptance
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_opening_segment_binding_checked_acceptance_opening
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_opening_segment_binding_checked_acceptance_pcs_and_fri
          validation
          artifact
          publicInput
          proof
          accepted))

theorem runtime_opening_segment_binding_checked_acceptance_bound_pcs_fri_contract
    {system : VerifierModel}
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (runtime_opening_segment_binding_checked_acceptance_bound_contract
        validation
        artifact
        publicInput
        proof
        accepted)
      (And.intro
        (runtime_opening_segment_binding_checked_acceptance_opening_bound_contract
          validation
          artifact
          publicInput
          proof
          accepted)
        (runtime_opening_segment_binding_checked_acceptance_pcs_and_fri
          validation
          artifact
          publicInput
          proof
          accepted))

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
      hashAssumptions
      validation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      openingAccepted

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle
      assumptions.crypto.hashCollisionResistance
      validation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      accepted

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (hashAssumptions : HashCollisionResistanceAssumption)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        hashAssumptions
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have runtimeEvidence :=
    runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening
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
  have boundPcsFri :=
    runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening
      hashAssumptions
      validation.openingValidation
      centralized
      constantBinding
      witnessBinding
      artifact
      publicInput
      proof
      openingAccepted
  rcases boundPcsFri with
    ⟨boundContract, pcsOpenings, friQueries⟩
  rcases boundContract with
    ⟨constantOpenings, witnessOpenings, friOpening⟩
  have openingEvidence :
      RuntimeOpeningEvidence
        system
        validation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource :=
    And.intro runtimeEvidence
      (And.intro constantOpenings
        (And.intro witnessOpenings
          (And.intro friOpening
            (And.intro pcsOpenings friQueries))))
  have runtimeAccepted :=
    validation.openingValidation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have runtimeSound :=
    runtime_soundness_checked_acceptance_sound
      assumptions
      validation.openingValidation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      runtimeAccepted
  exact And.intro segmentEvidence (And.intro openingEvidence runtimeSound.right)

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_sound_from_concrete_nary_merkle
    {Digest : Type uDigest}
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    {compress : List Digest -> Digest}
    (centralized :
      CentralizedNAryMerkleCompressionCollisionResistance
        assumptions.crypto.hashCollisionResistance
        compress)
    (constantBinding :
      RuntimeConstantOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress)
    (witnessBinding :
      RuntimeWitnessOpeningNAryConcreteBinding
        system
        validation.openingValidation
        Digest
        compress) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  exact
    runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening
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

theorem runtime_opening_segment_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingSound :=
    runtime_opening_checked_acceptance_sound
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  exact And.intro segmentEvidence openingSound

theorem runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof (_requiresExternalSource : Prop),
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof _requiresExternalSource accepted
  have openingAccepted :=
    validation.openingSegmentBindingAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_checked_acceptance_verifier_core_contract
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      _requiresExternalSource
      openingAccepted

theorem runtime_opening_segment_binding_checked_acceptance_evidence_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have sound :=
    runtime_opening_segment_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have core :=
    runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro sound.left
      (And.intro sound.right.left (And.intro core sound.right.right))

theorem runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingEvidence :=
    runtime_opening_checked_acceptance_evidence
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have coreContract :=
    runtime_opening_segment_binding_checked_acceptance_verifier_core_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  exact
    And.intro
      (runtime_opening_segment_binding_evidence_implies_bound_contract
        validation
        artifact
        publicInput
        proof
        segmentEvidence)
      (And.intro openingEvidence coreContract)

theorem runtime_opening_segment_binding_checked_acceptance_full_soundness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingFull :=
    runtime_opening_checked_acceptance_full_soundness_contract
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  exact
    And.intro
      (runtime_opening_segment_binding_evidence_implies_bound_contract
        validation
        artifact
        publicInput
        proof
        segmentEvidence)
      openingFull

theorem runtime_opening_segment_binding_audited_core_sound_witness_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeOpeningSegmentBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ (exists witness trace constraints,
            system.traceConsistent publicInput proof trace
              /\ system.constraintsSatisfied constraints trace
              /\ system.witnessMatchesTrace witness trace)
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have segmentEvidence :=
    runtime_opening_segment_binding_checked_acceptance_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have openingAccepted :=
    runtime_opening_segment_binding_checked_acceptance_opening
      validation
      artifact
      publicInput
      proof
      accepted
  have openingEvidence :=
    runtime_opening_checked_acceptance_evidence
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have runtimeChecked :=
    validation.openingValidation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have verifierAccepts :=
    runtime_soundness_checked_acceptance_verifier_accepts
      validation.openingValidation.runtimeSoundnessValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      runtimeChecked
  have auditedCoreSound :=
    accepted_proof_audited_core_and_sound_witness
      assumptions
      publicInput
      proof
      verifierAccepts
  rcases auditedCoreSound with
    ⟨cryptoEvidence, semanticEvidence, coreContract, soundWitness⟩
  have executionObligations :
      exists witness trace constraints,
        system.traceConsistent publicInput proof trace
          /\ system.constraintsSatisfied constraints trace
          /\ system.witnessMatchesTrace witness trace := by
    rcases soundWitness with
      ⟨witness,
        trace,
        constraints,
        _transcriptBound,
        _publicInputBound,
        _pcsOpenings,
        _friQueries,
        traceConsistent,
        constraintsSatisfied,
        witnessMatchesTrace⟩
    exact
      ⟨witness,
        trace,
        constraints,
        traceConsistent,
        constraintsSatisfied,
        witnessMatchesTrace⟩
  exact
    And.intro cryptoEvidence
      (And.intro semanticEvidence
        (And.intro segmentEvidence
          (And.intro openingEvidence
            (And.intro coreContract
              (And.intro executionObligations soundWitness)))))

set_option linter.style.longLine false in
theorem runtime_opening_segment_binding_checked_acceptance_full_soundness_with_fri_parser_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeOpeningSegmentBindingValidation system)
    (boundary : RuntimeFriOpeningSegmentParserBoundary system validation) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeOpeningSegmentBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningSegmentBindingBoundContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeOpeningEvidence
            system
            validation.openingValidation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ RuntimeOpeningBoundContract
            system
            validation.openingValidation
            artifact
            publicInput
            proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof
          /\ RuntimeFriOpeningSegmentParserContract
            boundary
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldTraceIdentityContract
            system
            validation
            artifact
            publicInput
            proof
          /\ RuntimeFriFoldQueryPlanOrderContract
            system
            validation
            artifact
            publicInput
            proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have fullContract :=
    runtime_opening_segment_binding_checked_acceptance_full_soundness_contract
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have parserContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_parser_contract
      validation
      boundary
      artifact
      publicInput
      proof
      accepted
  have foldTraceIdentityContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_fold_trace_identity_contract
      validation
      artifact
      publicInput
      proof
      accepted
  have foldQueryPlanOrderContract :=
    runtime_opening_segment_binding_checked_acceptance_fri_fold_query_plan_order_contract
      validation
      artifact
      publicInput
      proof
      accepted
  rcases fullContract with
    ⟨segmentBound,
      openingEvidence,
      openingBound,
      pcsOpenings,
      friQueries,
      coreContract,
      soundWitness⟩
  exact
    And.intro segmentBound
      (And.intro openingEvidence
        (And.intro openingBound
          (And.intro pcsOpenings
            (And.intro friQueries
              (And.intro coreContract
                (And.intro soundWitness
                  (And.intro parserContract
                    (And.intro foldTraceIdentityContract foldQueryPlanOrderContract))))))))

end Lzvm
