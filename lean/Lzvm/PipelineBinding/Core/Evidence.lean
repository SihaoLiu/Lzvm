/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.PipelineBinding.Core.Base

/-!
Runtime proof pipeline evidence projections.
-/

namespace Lzvm

theorem runtime_pipeline_binding_evidence_implies_trace_preflight_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeTraceConstraintPreflightBindingEvidence
        system
        validation.traceBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact tracePreflightEvidence

theorem runtime_pipeline_binding_evidence_implies_trace_constraint_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeTraceConstraintEvidence
        system
        validation.traceBindingValidation.traceConstraintValidation
        artifact
        publicInput
        proof
        requiresExternalSource := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact traceConstraintEvidence

theorem runtime_pipeline_binding_evidence_implies_query_plan_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeQueryPlanBindingEvidence
        system
        validation.queryPlanBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact queryPlanEvidence

theorem runtime_pipeline_binding_evidence_implies_challenge_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeChallengeSegmentBindingEvidence
        system
        validation.queryPlanBindingValidation.challengeValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact challengeEvidence

theorem runtime_pipeline_binding_evidence_implies_opening_segment_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeOpeningSegmentBindingEvidence
        system
        validation.queryPlanBindingValidation.openingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact openingSegmentEvidence

theorem runtime_pipeline_binding_evidence_implies_opening_evidence
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeOpeningEvidence
        system
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact openingEvidence

theorem runtime_pipeline_binding_evidence_implies_query_opening_core_contract
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeQueryPlanBindingEvidence
          system
          validation.queryPlanBindingValidation
          artifact
          publicInput
          proof
        /\ RuntimeChallengeSegmentBindingEvidence
          system
          validation.queryPlanBindingValidation.challengeValidation
          artifact
          publicInput
          proof
        /\ RuntimeOpeningSegmentBindingEvidence
          system
          validation.queryPlanBindingValidation.openingValidation
          artifact
          publicInput
          proof
        /\ RuntimeOpeningEvidence
          system
          validation.queryPlanBindingValidation.openingValidation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource
        /\ RuntimeVerifierCoreContract system publicInput proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      queryPlanEvidence,
      challengeEvidence,
      openingSegmentEvidence,
      openingEvidence,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid⟩
  exact
    And.intro queryPlanEvidence
      (And.intro challengeEvidence
        (And.intro openingSegmentEvidence
          (And.intro openingEvidence
            (And.intro transcriptBound
              (And.intro publicInputBound
                (And.intro pcsOpeningsValid friQueriesValid))))))

theorem runtime_pipeline_binding_evidence_implies_external_source_requirements
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      ExternalSourceOpeningRequirement
          system
          (runtime_pipeline_trace_source_validation validation)
          publicInput
          proof
          requiresExternalSource
        /\ ExternalSourceOpeningRequirement
          system
          (runtime_pipeline_opening_source_validation validation)
          publicInput
          proof
          requiresExternalSource := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact
    And.intro
      (runtime_opening_evidence_implies_external_source_requirement
        validation.traceBindingValidation.traceConstraintValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource
        (runtime_trace_constraint_evidence_implies_opening_evidence
          validation.traceBindingValidation.traceConstraintValidation
          artifact
          publicInput
          proof
          requiresExternalSource
          traceConstraintEvidence))
      (runtime_opening_evidence_implies_external_source_requirement
        validation.queryPlanBindingValidation.openingValidation.openingValidation
        artifact
        publicInput
        proof
        requiresExternalSource
        openingEvidence)

theorem runtime_pipeline_binding_evidence_implies_seeded_query_plan_contract
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeQueryPlanBindingSeededContract
        system
        validation.queryPlanBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      _traceConstraintEvidence,
      queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact
    runtime_query_plan_binding_evidence_implies_seeded_contract
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanEvidence

theorem runtime_pipeline_binding_evidence_implies_query_plan_material_manifest_contract
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeQueryPlanMaterialManifestContract
        system
        validation.queryPlanBindingValidation
        artifact
        publicInput
        proof := by
  intro evidence
  have queryPlanEvidence :=
    runtime_pipeline_binding_evidence_implies_query_plan_evidence evidence
  exact
    runtime_query_plan_binding_evidence_implies_material_manifest_contract
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      queryPlanEvidence

theorem runtime_pipeline_binding_evidence_implies_query_plan_segment_canonical
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      validation.queryPlanBindingValidation.queryPlanSegmentCanonical
        artifact
        publicInput
        proof := by
  intro evidence
  have materialManifest :=
    runtime_pipeline_binding_evidence_implies_query_plan_material_manifest_contract
      evidence
  exact
    runtime_query_plan_material_manifest_contract_implies_segment_canonical
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      materialManifest

theorem runtime_pipeline_binding_evidence_implies_query_plan_material_manifest_matches_schedule
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      validation.queryPlanBindingValidation.queryPlanMaterialManifestMatchesSchedule
        artifact
        publicInput
        proof := by
  intro evidence
  have materialManifest :=
    runtime_pipeline_binding_evidence_implies_query_plan_material_manifest_contract
      evidence
  exact
    runtime_query_plan_material_manifest_contract_implies_matches_schedule
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      materialManifest

theorem runtime_pipeline_binding_evidence_implies_trace_semantic_evidence_complete
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      RuntimeTraceConstraintSemanticEvidenceComplete
        system
        validation.traceBindingValidation.traceConstraintValidation
        artifact
        publicInput
        proof := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact
    runtime_trace_constraint_evidence_implies_semantic_evidence_complete
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintEvidence

theorem runtime_pipeline_binding_evidence_implies_execution_obligations
    {system : VerifierModel}
    {validation : RuntimePipelineBindingValidation system}
    {artifact : RuntimeArtifact}
    {publicInput : PublicInput}
    {proof : Proof}
    {requiresExternalSource : Prop} :
    RuntimePipelineBindingEvidence
        system
        validation
        artifact
        publicInput
        proof
        requiresExternalSource ->
      exists witness trace constraints,
        system.traceConsistent publicInput proof trace
          /\ system.constraintsSatisfied constraints trace
          /\ system.witnessMatchesTrace witness trace := by
  intro evidence
  rcases evidence with
    ⟨_ethEvidence,
      _artifactEvidence,
      _runtimeArtifactEvidence,
      _tracePreflightEvidence,
      traceConstraintEvidence,
      _queryPlanEvidence,
      _challengeEvidence,
      _openingSegmentEvidence,
      _openingEvidence,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  have traceWitnessEvidence :=
    runtime_trace_constraint_evidence_implies_trace_witness_evidence
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceConstraintEvidence
  rcases traceWitnessEvidence with
    ⟨witness,
      trace,
      constraints,
      _traceExtracted,
      _constraintsEvaluated,
      _witnessExtracted,
      _backendConformant,
      traceConsistent,
      constraintsSatisfied,
      witnessMatchesTrace⟩
  exact
    Exists.intro witness
      (Exists.intro trace
        (Exists.intro constraints
          (And.intro traceConsistent
            (And.intro constraintsSatisfied witnessMatchesTrace))))

end Lzvm
