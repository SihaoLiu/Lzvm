/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.OpeningValidation

/-!
Runtime trace extraction and regular constraint validation obligations.
-/

namespace Lzvm

structure RuntimeTraceConstraintValidation (system : VerifierModel) where
  openingValidation : RuntimeOpeningValidation system
  traceConstraintAccepted : RuntimeArtifact -> PublicInput -> Proof -> Prop
  traceExtracted : RuntimeArtifact -> PublicInput -> Proof -> Trace -> Prop
  constraintsEvaluated :
    RuntimeArtifact -> PublicInput -> Proof -> ConstraintSystem -> Trace -> Prop
  witnessExtractedFromTrace :
    RuntimeArtifact -> PublicInput -> Proof -> Witness -> Trace -> Prop
  constraintBackendConformant :
    RuntimeArtifact -> PublicInput -> Proof -> ConstraintSystem -> Trace -> Prop
  traceEvidenceMatchesWitnessCommitments :
    RuntimeArtifact -> PublicInput -> Proof -> Prop
  traceEvidenceMatchesConstraintCatalog :
    RuntimeArtifact -> PublicInput -> Proof -> Prop
  traceConstraintAcceptedImpliesOpeningAccepted :
    forall artifact publicInput proof,
      traceConstraintAccepted artifact publicInput proof ->
        openingValidation.openingAccepted artifact publicInput proof
  traceConstraintAcceptedImpliesTraceEvidenceMatchesWitnessCommitments :
    forall artifact publicInput proof,
      traceConstraintAccepted artifact publicInput proof ->
        traceEvidenceMatchesWitnessCommitments artifact publicInput proof
  traceConstraintAcceptedImpliesTraceEvidenceMatchesConstraintCatalog :
    forall artifact publicInput proof,
      traceConstraintAccepted artifact publicInput proof ->
        traceEvidenceMatchesConstraintCatalog artifact publicInput proof
  traceConstraintAcceptedImpliesTraceExtracted :
    forall artifact publicInput proof,
      traceConstraintAccepted artifact publicInput proof ->
        exists trace, traceExtracted artifact publicInput proof trace
  traceConstraintAcceptedImpliesConstraintsEvaluated :
    forall artifact publicInput proof trace,
      traceConstraintAccepted artifact publicInput proof ->
        traceExtracted artifact publicInput proof trace ->
          exists constraints,
            constraintsEvaluated artifact publicInput proof constraints trace
  traceConstraintAcceptedImpliesWitnessExtracted :
    forall artifact publicInput proof trace constraints,
      traceConstraintAccepted artifact publicInput proof ->
        traceExtracted artifact publicInput proof trace ->
          constraintsEvaluated artifact publicInput proof constraints trace ->
            exists witness, witnessExtractedFromTrace artifact publicInput proof witness trace
  constraintsEvaluatedImpliesBackendConformant :
    forall artifact publicInput proof constraints trace,
      constraintsEvaluated artifact publicInput proof constraints trace ->
        constraintBackendConformant artifact publicInput proof constraints trace
  traceExtractionImpliesTraceConsistent :
    forall artifact publicInput proof trace,
      traceExtracted artifact publicInput proof trace ->
        system.traceConsistent publicInput proof trace
  constraintEvaluationImpliesSatisfied :
    forall artifact publicInput proof constraints trace,
      constraintsEvaluated artifact publicInput proof constraints trace ->
        constraintBackendConformant artifact publicInput proof constraints trace ->
          system.constraintsSatisfied constraints trace
  witnessExtractionImpliesMatchesTrace :
    forall artifact publicInput proof witness trace,
      witnessExtractedFromTrace artifact publicInput proof witness trace ->
        system.witnessMatchesTrace witness trace

def RuntimeTraceConstraintCheckedAcceptance
    (_system : VerifierModel)
    (validation : RuntimeTraceConstraintValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.traceConstraintAccepted artifact publicInput proof

def RuntimeTraceConstraintArtifactBindingEvidence
    (_system : VerifierModel)
    (validation : RuntimeTraceConstraintValidation _system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  validation.traceEvidenceMatchesWitnessCommitments artifact publicInput proof
    /\ validation.traceEvidenceMatchesConstraintCatalog artifact publicInput proof

def RuntimeTraceConstraintEvidence
    (system : VerifierModel)
    (validation : RuntimeTraceConstraintValidation system)
    (artifact : RuntimeArtifact)
    (publicInput : PublicInput)
    (proof : Proof)
    (requiresExternalSource : Prop) : Prop :=
  RuntimeOpeningEvidence
      system
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
    /\ RuntimeTraceConstraintArtifactBindingEvidence
      system
      validation
      artifact
      publicInput
      proof
    /\ exists witness trace constraints,
      validation.traceExtracted artifact publicInput proof trace
        /\ validation.constraintsEvaluated artifact publicInput proof constraints trace
        /\ validation.witnessExtractedFromTrace artifact publicInput proof witness trace
        /\ validation.constraintBackendConformant
          artifact
          publicInput
          proof
          constraints
          trace
        /\ system.traceConsistent publicInput proof trace
        /\ system.constraintsSatisfied constraints trace
        /\ system.witnessMatchesTrace witness trace

theorem runtime_trace_constraint_checked_acceptance_artifact_binding_evidence
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintArtifactBindingEvidence
          system
          validation
          artifact
          publicInput
          proof := by
  intro artifact publicInput proof accepted
  exact
    And.intro
      (validation.traceConstraintAcceptedImpliesTraceEvidenceMatchesWitnessCommitments
        artifact
        publicInput
        proof
        accepted)
      (validation.traceConstraintAcceptedImpliesTraceEvidenceMatchesConstraintCatalog
        artifact
        publicInput
        proof
        accepted)

theorem runtime_trace_constraint_checked_acceptance_opening_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeOpeningEvidence
          system
          validation.openingValidation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingAccepted :=
    validation.traceConstraintAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted
  exact
    runtime_opening_checked_acceptance_evidence
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted

theorem runtime_trace_constraint_checked_acceptance_trace_witness_evidence
    {system : VerifierModel}
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        exists witness trace constraints,
          validation.traceExtracted artifact publicInput proof trace
            /\ validation.constraintsEvaluated artifact publicInput proof constraints trace
            /\ validation.witnessExtractedFromTrace artifact publicInput proof witness trace
            /\ validation.constraintBackendConformant
              artifact
              publicInput
              proof
              constraints
              trace
            /\ system.traceConsistent publicInput proof trace
            /\ system.constraintsSatisfied constraints trace
            /\ system.witnessMatchesTrace witness trace := by
  intro artifact publicInput proof accepted
  cases
      validation.traceConstraintAcceptedImpliesTraceExtracted
        artifact
        publicInput
        proof
        accepted
    with
  | intro trace traceExtracted =>
    cases
        validation.traceConstraintAcceptedImpliesConstraintsEvaluated
          artifact
          publicInput
          proof
          trace
          accepted
          traceExtracted
      with
    | intro constraints constraintsEvaluated =>
      cases
          validation.traceConstraintAcceptedImpliesWitnessExtracted
            artifact
            publicInput
            proof
            trace
            constraints
            accepted
            traceExtracted
            constraintsEvaluated
        with
      | intro witness witnessExtracted =>
        have backendConformant :=
          validation.constraintsEvaluatedImpliesBackendConformant
            artifact
            publicInput
            proof
            constraints
            trace
            constraintsEvaluated
        have traceConsistent :=
          validation.traceExtractionImpliesTraceConsistent
            artifact
            publicInput
            proof
            trace
            traceExtracted
        have constraintsSatisfied :=
          validation.constraintEvaluationImpliesSatisfied
            artifact
            publicInput
            proof
            constraints
            trace
            constraintsEvaluated
            backendConformant
        have witnessMatchesTrace :=
          validation.witnessExtractionImpliesMatchesTrace
            artifact
            publicInput
            proof
            witness
            trace
            witnessExtracted
        exact
          Exists.intro witness
            (Exists.intro trace
              (Exists.intro constraints
                (And.intro traceExtracted
                  (And.intro constraintsEvaluated
                    (And.intro witnessExtracted
                      (And.intro backendConformant
                        (And.intro traceConsistent
                          (And.intro constraintsSatisfied witnessMatchesTrace))))))))

theorem runtime_trace_constraint_checked_acceptance_evidence
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintEvidence
          system
          validation
          artifact
          publicInput
          proof
          requiresExternalSource := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingEvidence :=
    runtime_trace_constraint_checked_acceptance_opening_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have artifactBindingEvidence :=
    runtime_trace_constraint_checked_acceptance_artifact_binding_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  have traceWitnessEvidence :=
    runtime_trace_constraint_checked_acceptance_trace_witness_evidence
      validation
      artifact
      publicInput
      proof
      accepted
  exact
    And.intro openingEvidence
      (And.intro artifactBindingEvidence traceWitnessEvidence)

theorem runtime_trace_constraint_evidence_implies_sound_witness
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      system.accepts publicInput proof ->
        RuntimeTraceConstraintEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource ->
          SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource verifierAccepts evidence
  cases evidence with
  | intro openingEvidence remainingEvidence =>
    cases remainingEvidence with
    | intro _artifactBindingEvidence traceEvidence =>
      cases traceEvidence with
      | intro witness traceAndConstraints =>
        cases traceAndConstraints with
        | intro trace constraintEvidence =>
          cases constraintEvidence with
        | intro constraints checkedEvidence =>
          have transcriptBound := openingEvidence.left.right.right.left
          have publicInputBound :=
            assumptions.semantic.public_input_binding publicInput proof verifierAccepts
          have pcsOpenings := openingEvidence.right.right.right.right.left
          have friQueries := openingEvidence.right.right.right.right.right
          have traceConsistent := checkedEvidence.right.right.right.right.left
          have constraintsSatisfied := checkedEvidence.right.right.right.right.right.left
          have witnessMatchesTrace := checkedEvidence.right.right.right.right.right.right
          exact
            Exists.intro witness
              (Exists.intro trace
                (Exists.intro constraints
                  (And.intro transcriptBound
                    (And.intro publicInputBound
                      (And.intro pcsOpenings
                        (And.intro friQueries
                          (And.intro traceConsistent
                            (And.intro constraintsSatisfied witnessMatchesTrace))))))))

theorem runtime_trace_constraint_checked_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        RuntimeTraceConstraintEvidence
            system
            validation
            artifact
            publicInput
            proof
            requiresExternalSource
          /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted
  have openingAccepted :=
    validation.traceConstraintAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted
  have runtimeAccepted :=
    validation.openingValidation.openingAcceptedImpliesRuntimeSoundnessAccepted
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
  have transcriptAccepted := runtimeAccepted.left
  have artifactBindingAccepted :=
    validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
      |>.transcriptAcceptedImpliesArtifactBindingAccepted
      artifact
      publicInput
      proof
      transcriptAccepted
  have runtimeArtifactAccepted :=
    validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
      |>.artifactBindingValidation
      |>.bindingAcceptedImpliesRuntimeAccepted
      artifact
      publicInput
      proof
      artifactBindingAccepted
  have verifierAccepts :=
    runtime_artifact_checked_acceptance_implies_verifier_accepts
      (validation.openingValidation.runtimeSoundnessValidation.transcriptValidation
        |>.artifactBindingValidation
        |>.runtimeValidation)
      artifact
      publicInput
      proof
      runtimeArtifactAccepted
  have evidence :=
    runtime_trace_constraint_checked_acceptance_evidence
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have soundWitness :=
    runtime_trace_constraint_evidence_implies_sound_witness
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      verifierAccepts
      evidence
  exact And.intro evidence soundWitness

theorem runtime_trace_constraint_required_external_source_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (validation : RuntimeTraceConstraintValidation system) :
    forall artifact publicInput proof requiresExternalSource,
      RuntimeTraceConstraintCheckedAcceptance
          system
          validation
          artifact
          publicInput
          proof ->
        requiresExternalSource ->
          RuntimeTraceConstraintEvidence
              system
              validation
              artifact
              publicInput
              proof
              requiresExternalSource
            /\ ExternalSourceOpeningEvidence
              system
              validation.openingValidation.runtimeSoundnessValidation.sourceValidation
              publicInput
              proof
            /\ SoundWitness system publicInput proof := by
  intro artifact publicInput proof requiresExternalSource accepted required
  have openingAccepted :=
    validation.traceConstraintAcceptedImpliesOpeningAccepted
      artifact
      publicInput
      proof
      accepted
  have traceSound :=
    runtime_trace_constraint_checked_acceptance_sound
      assumptions
      validation
      artifact
      publicInput
      proof
      requiresExternalSource
      accepted
  have openingSound :=
    runtime_opening_required_external_source_sound
      assumptions
      validation.openingValidation
      artifact
      publicInput
      proof
      requiresExternalSource
      openingAccepted
      required
  exact
    And.intro traceSound.left
      (And.intro openingSound.right.left traceSound.right)

end Lzvm
