/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTiming

/-!
Batched proof-timing core-contract projections.
-/

namespace Lzvm

universe u

theorem proof_timing_projected_metadata_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      metadata
      publicInput
      proof
      observed

theorem proof_timing_projected_metadata_acceptance_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (metadata : Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance system metadata publicInput proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_audited_core_contract
      assumptions
      metadata
      publicInput
      proof
      observed

theorem
  proof_timing_projected_finish_summary_required_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some summary)
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem
  proof_timing_projected_finish_summary_required_audited_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some summary)
        publicInput
        proof ->
        RequiredCryptographicAssumptionStatements assumptions.crypto
          /\ RequiredSemanticAssumptionStatements assumptions.semantic
          /\ RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_audited_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

structure ProofTimingProjectedCoreContracts
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop where
  witnessOpeningRowValueTiming :
    RuntimeVerifierCoreContract system publicInput proof
  constantMaterialValidationTiming :
    RuntimeVerifierCoreContract system publicInput proof
  proverGpuMode :
    RuntimeVerifierCoreContract system publicInput proof
  gpuRunOptions :
    RuntimeVerifierCoreContract system publicInput proof
  cudaBackend :
    RuntimeVerifierCoreContract system publicInput proof
  cudaAllocatorTiming :
    RuntimeVerifierCoreContract system publicInput proof
  proofArtifactFinishTiming :
    RuntimeVerifierCoreContract system publicInput proof
  proofTimingBatch :
    RuntimeVerifierCoreContract system publicInput proof

theorem proof_timing_projected_core_contracts
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (rowValueTiming : Option WitnessOpeningRowValueTimingSummary)
    (constantMaterialTiming : Option ConstantMaterialValidationTimingSummary)
    (gpuMode : Option ProverGpuModeSummary)
    (gpuRunOptions : Option GpuRunOptionsSummary)
    (cudaBackend : Option CudaBackendSummary)
    (cudaAllocatorTiming : Option CudaAllocatorTimingSummary)
    (finishTiming : Option ProofArtifactFinishTimingSummary)
    (batchTiming : Option ProofTimingBatchSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance
        system
        rowValueTiming
        publicInput
        proof ->
      ConstantMaterialValidationTimingObservedAcceptance
        system
        constantMaterialTiming
        publicInput
        proof ->
      ProverGpuModeObservedAcceptance system gpuMode publicInput proof ->
      GpuRunOptionsObservedAcceptance system gpuRunOptions publicInput proof ->
      CudaBackendObservedAcceptance system cudaBackend publicInput proof ->
      CudaAllocatorTimingObservedAcceptance
        system
        cudaAllocatorTiming
        publicInput
        proof ->
      ProofArtifactFinishTimingObservedAcceptance
        system
        finishTiming
        publicInput
        proof ->
      ProofTimingBatchObservedAcceptance
        system
        batchTiming
        publicInput
        proof ->
        ProofTimingProjectedCoreContracts system publicInput proof := by
  intro publicInput proof rowValueObserved constantMaterialObserved
    gpuModeObserved gpuRunOptionsObserved cudaBackendObserved
    cudaAllocatorObserved finishObserved batchObserved
  exact
    { witnessOpeningRowValueTiming :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          rowValueTiming
          publicInput
          proof
          rowValueObserved
      constantMaterialValidationTiming :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          constantMaterialTiming
          publicInput
          proof
          constantMaterialObserved
      proverGpuMode :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          gpuMode
          publicInput
          proof
          gpuModeObserved
      gpuRunOptions :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          gpuRunOptions
          publicInput
          proof
          gpuRunOptionsObserved
      cudaBackend :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          cudaBackend
          publicInput
          proof
          cudaBackendObserved
      cudaAllocatorTiming :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          cudaAllocatorTiming
          publicInput
          proof
          cudaAllocatorObserved
      proofArtifactFinishTiming :=
        proof_timing_projected_metadata_acceptance_verifier_core_contract
          assumptions
          finishTiming
          publicInput
          proof
          finishObserved
      proofTimingBatch :=
        proof_timing_batch_acceptance_verifier_core_contract
          assumptions
          batchTiming
          publicInput
          proof
          batchObserved }

end Lzvm
