/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.EthBlockPublicInputBinding
import Lzvm.TraceConstraintArtifactBinding
import Lzvm.QueryPlanBinding

/-!
Runtime proof pipeline binding obligations.
-/

namespace Lzvm

structure RuntimePipelineBindingValidation (system : VerifierModel) where
  ethBindingValidation : RuntimeEthBlockPublicInputBindingValidation system
  traceBindingValidation : RuntimeTraceConstraintArtifactBindingValidation system
  queryPlanBindingValidation : RuntimeQueryPlanBindingValidation system
  pipelineBindingAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  pipelineBindingAcceptedImpliesEthBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        ethBindingValidation.ethBlockBindingAccepted artifact publicInput proof
  pipelineBindingAcceptedImpliesTraceBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        traceBindingValidation.traceArtifactBindingAccepted artifact publicInput proof
  pipelineBindingAcceptedImpliesQueryPlanBindingAccepted :
    forall artifact publicInput proof,
      pipelineBindingAccepted artifact publicInput proof ->
        queryPlanBindingValidation.queryPlanBindingAccepted artifact publicInput proof

def RuntimePipelineBindingEvidence
    (system : VerifierModel)
    (validation : RuntimePipelineBindingValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeEthBlockPublicInputBindingEvidence
      system
      validation.ethBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeProofArtifactBindingEvidence
      system
      validation.ethBindingValidation.proofArtifactBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeArtifactEvidence
      system
      validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
      artifact
      publicInput
      proof
    /\ RuntimeTraceConstraintPreflightBindingEvidence
      system
      validation.traceBindingValidation
      artifact
      publicInput
      proof
    /\ RuntimeTraceConstraintEvidence
      system
      validation.traceBindingValidation.traceConstraintValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ RuntimeQueryPlanBindingEvidence
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
    /\ system.transcriptBound publicInput proof
    /\ system.publicInputBound publicInput proof
    /\ system.pcsOpeningsValid publicInput proof
    /\ system.friQueriesValid publicInput proof

theorem runtime_pipeline_binding_evidence_implies_public_input_bound
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
      system.publicInputBound publicInput proof := by
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
      _openingEvidence,
      _transcriptBound,
      publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid⟩
  exact publicInputBound

def RuntimePipelineBindingCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimePipelineBindingValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.pipelineBindingAccepted artifact publicInput proof

theorem runtime_pipeline_binding_checked_acceptance_eth
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeEthBlockPublicInputBindingCheckedAcceptance
          system
          validation.ethBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesEthBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_trace
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintArtifactBindingCheckedAcceptance
          system
          validation.traceBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesTraceBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_query_plan
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeQueryPlanBindingCheckedAcceptance
          system
          validation.queryPlanBindingValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    validation.pipelineBindingAcceptedImpliesQueryPlanBindingAccepted
      artifact
      publicInput
      proof
      accepted

theorem runtime_pipeline_binding_checked_acceptance_verifier_accepts
    {system : VerifierModel}
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.accepts publicInput proof := by
  intro artifact publicInput proof accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have artifactAccepted :=
    runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  let proofArtifactValidation :=
    validation.ethBindingValidation.proofArtifactBindingValidation
  have runtimeAccepted :=
    proofArtifactValidation.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactAccepted
  exact
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      proofArtifactValidation.runtimeValidation
      artifact
      publicInput
      proof
      runtimeAccepted

theorem runtime_pipeline_binding_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimePipelineBindingEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have ethAccepted :=
    runtime_pipeline_binding_checked_acceptance_eth
      validation
      artifact
      publicInput
      proof
      accepted
  have traceAccepted :=
    runtime_pipeline_binding_checked_acceptance_trace
      validation
      artifact
      publicInput
      proof
      accepted
  have queryPlanAccepted :=
    runtime_pipeline_binding_checked_acceptance_query_plan
      validation
      artifact
      publicInput
      proof
      accepted
  have ethSound :=
    runtime_eth_block_public_input_binding_checked_acceptance_sound
      assumptions
      validation.ethBindingValidation
      artifact
      publicInput
      proof
      ethAccepted
  have traceSound :=
    runtime_trace_constraint_artifact_binding_checked_acceptance_sound
      assumptions
      validation.traceBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      traceAccepted
  have queryPlanSound :=
    runtime_query_plan_binding_checked_acceptance_sound
      assumptions
      validation.queryPlanBindingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      queryPlanAccepted
  have ethEvidence := ethSound.left
  have artifactEvidence := ethSound.right.left
  have runtimeArtifactEvidence := ethSound.right.right.left
  have tracePreflightEvidence := traceSound.left
  have traceConstraintEvidence := traceSound.right.left
  have queryPlanEvidence := queryPlanSound.left
  have challengeEvidence := queryPlanSound.right.left
  have openingSegmentEvidence := queryPlanSound.right.right.left
  have openingEvidence := queryPlanSound.right.right.right.left
  have transcriptBound := queryPlanSound.right.right.right.right.left
  have pcsOpeningsValid := queryPlanSound.right.right.right.right.right.left
  have friQueriesValid := queryPlanSound.right.right.right.right.right.right.left
  have soundWitness := queryPlanSound.right.right.right.right.right.right.right
  have publicInputBound : system.publicInputBound publicInput proof := by
    rcases soundWitness with
      ⟨_witness,
        _trace,
        _constraints,
        _transcriptBound,
        publicInputBound,
        _pcsOpeningsValid,
        _friQueriesValid,
        _traceConsistent,
        _constraintsSatisfied,
        _witnessMatchesTrace⟩
    exact publicInputBound
  exact
    ⟨⟨ethEvidence,
        artifactEvidence,
        runtimeArtifactEvidence,
        tracePreflightEvidence,
        traceConstraintEvidence,
        queryPlanEvidence,
        challengeEvidence,
        openingSegmentEvidence,
        openingEvidence,
        transcriptBound,
        publicInputBound,
        pcsOpeningsValid,
        friQueriesValid⟩,
      soundWitness⟩

theorem runtime_pipeline_binding_checked_acceptance_core_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        system.transcriptBound publicInput proof
          /\ system.publicInputBound publicInput proof
          /\ system.pcsOpeningsValid publicInput proof
          /\ system.friQueriesValid publicInput proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  rcases sound.right with
    ⟨_witness,
      _trace,
      _constraints,
      transcriptBound,
      publicInputBound,
      pcsOpeningsValid,
      friQueriesValid,
      _traceConsistent,
      _constraintsSatisfied,
      _witnessMatchesTrace⟩
  exact
    And.intro transcriptBound
      (And.intro publicInputBound
        (And.intro pcsOpeningsValid friQueriesValid))

theorem runtime_pipeline_binding_checked_acceptance_soundness_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeArtifactSoundnessObligations
          system
          validation.ethBindingValidation.proofArtifactBindingValidation.runtimeValidation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  have runtimeArtifactEvidence := sound.left.right.right.left
  have verifierAccepts :=
    runtime_pipeline_binding_checked_acceptance_verifier_accepts
      validation
      artifact
      publicInput
      proof
      accepted
  have coreObligations :=
    runtime_pipeline_binding_checked_acceptance_core_obligations
      assumptions
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro runtimeArtifactEvidence
      (And.intro verifierAccepts
        (And.intro coreObligations.left
          (And.intro coreObligations.right.left
            (And.intro coreObligations.right.right.left coreObligations.right.right.right))))

theorem runtime_pipeline_binding_checked_acceptance_execution_obligations
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimePipelineBindingValidation system) :
    forall artifact publicInput proof,
      RuntimePipelineBindingCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        exists witness trace constraints,
          system.traceConsistent publicInput proof trace
            /\ system.constraintsSatisfied constraints trace
            /\ system.witnessMatchesTrace witness trace := by
  intro artifact publicInput proof accepted
  have sound :=
    runtime_pipeline_binding_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      False
      accepted
  rcases sound.right with
    ⟨witness,
      trace,
      constraints,
      _transcriptBound,
      _publicInputBound,
      _pcsOpeningsValid,
      _friQueriesValid,
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
