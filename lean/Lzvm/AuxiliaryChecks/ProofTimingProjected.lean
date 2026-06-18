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

theorem proof_timing_projected_core_contracts
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (rowValueTiming : Option WitnessOpeningRowValueTimingSummary)
    (constantMaterialTiming : Option ConstantMaterialValidationTimingSummary)
    (gpuMode : Option ProverGpuModeSummary)
    (gpuRunOptions : Option GpuRunOptionsSummary)
    (cudaBackend : Option CudaBackendSummary)
    (cudaAllocatorTiming : Option CudaAllocatorTimingSummary)
    (finishTiming : Option ProofArtifactFinishTimingSummary) :
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
        ProofTimingProjectedCoreContracts system publicInput proof := by
  intro publicInput proof rowValueObserved constantMaterialObserved
    gpuModeObserved gpuRunOptionsObserved cudaBackendObserved
    cudaAllocatorObserved finishObserved
  exact
    { witnessOpeningRowValueTiming :=
        witness_opening_row_value_timing_acceptance_verifier_core_contract
          assumptions
          rowValueTiming
          publicInput
          proof
          rowValueObserved
      constantMaterialValidationTiming :=
        constant_material_validation_timing_acceptance_verifier_core_contract
          assumptions
          constantMaterialTiming
          publicInput
          proof
          constantMaterialObserved
      proverGpuMode :=
        prover_gpu_mode_acceptance_verifier_core_contract
          assumptions
          gpuMode
          publicInput
          proof
          gpuModeObserved
      gpuRunOptions :=
        gpu_run_options_acceptance_verifier_core_contract
          assumptions
          gpuRunOptions
          publicInput
          proof
          gpuRunOptionsObserved
      cudaBackend :=
        cuda_backend_acceptance_verifier_core_contract
          assumptions
          cudaBackend
          publicInput
          proof
          cudaBackendObserved
      cudaAllocatorTiming :=
        cuda_allocator_timing_acceptance_verifier_core_contract
          assumptions
          cudaAllocatorTiming
          publicInput
          proof
          cudaAllocatorObserved
      proofArtifactFinishTiming :=
        proof_artifact_finish_timing_acceptance_verifier_core_contract
          assumptions
          finishTiming
          publicInput
          proof
          finishObserved }

end Lzvm
