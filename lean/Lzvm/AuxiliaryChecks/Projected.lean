/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.ProofTimingProjected
import Lzvm.AuxiliaryChecks.RuntimePerformance
import Lzvm.AuxiliaryChecks.TimingProjected

/-!
Top-level batching for auxiliary timing and runtime-performance projections.
-/

namespace Lzvm

structure AuxiliaryProjectedCoreContracts
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop where
  timing :
    TimingProjectedCoreContracts system publicInput proof
  proofTiming :
    ProofTimingProjectedCoreContracts system publicInput proof
  runtimePerformance :
    RuntimePerformanceObservationProjectedCoreContracts system publicInput proof

theorem runtime_performance_observation_auxiliary_projected_core_contracts
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        AuxiliaryProjectedCoreContracts system publicInput proof := by
  intro publicInput proof observed
  exact
    { timing :=
        timing_projected_core_contracts
          assumptions
          summary.timingObservations
          summary.guestPcTraceTiming
          publicInput
          proof
          (runtime_performance_observation_projects_timing_observations
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_guest_pc_trace_timing
            summary
            publicInput
            proof
            observed)
      proofTiming :=
        proof_timing_projected_core_contracts
          assumptions
          summary.witnessOpeningRowValueTiming
          summary.constantMaterialValidationTiming
          summary.proverGpuMode
          summary.gpuRunOptions
          summary.cudaBackend
          summary.cudaAllocatorTiming
          summary.proofArtifactFinishTiming
          publicInput
          proof
          (runtime_performance_observation_projects_witness_opening_row_value_timing
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_constant_material_validation_timing
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_prover_gpu_mode
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_gpu_run_options
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_cuda_backend
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_cuda_allocator_timing
            summary
            publicInput
            proof
            observed)
          (runtime_performance_observation_projects_proof_artifact_finish_timing
            summary
            publicInput
            proof
            observed)
      runtimePerformance :=
        runtime_performance_observation_projected_core_contracts
          assumptions
          summary
          publicInput
          proof
          observed }

end Lzvm
