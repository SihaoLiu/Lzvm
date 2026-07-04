/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks.Timing

/-!
Aggregate runtime performance acceptance projections.
-/

namespace Lzvm

universe u

def RuntimePerformanceObservedAcceptance
    (system : VerifierModel)
    (summary : RuntimePerformanceObservationSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem runtime_performance_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projects_metadata
    {system : VerifierModel}
    {Metadata : Type u}
    (summary : RuntimePerformanceObservationSummary)
    (project : RuntimePerformanceObservationSummary -> Metadata) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        IgnoredMetadataObservedAcceptance
          system
          (project summary)
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_projected_metadata_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (summary : RuntimePerformanceObservationSummary)
    (project : RuntimePerformanceObservationSummary -> Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance
        system
        (project summary)
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_sound
      assumptions
      (project summary)
      publicInput
      proof
      observed

theorem runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (summary : RuntimePerformanceObservationSummary)
    (project : RuntimePerformanceObservationSummary -> Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance
        system
        (project summary)
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      (project summary)
      publicInput
      proof
      observed

theorem runtime_performance_observation_projected_metadata_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    {Metadata : Type u}
    (summary : RuntimePerformanceObservationSummary)
    (project : RuntimePerformanceObservationSummary -> Metadata) :
    forall publicInput proof,
      IgnoredMetadataObservedAcceptance
        system
        (project summary)
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_core_and_sound
      assumptions
      (project summary)
      publicInput
      proof
      observed

theorem runtime_performance_observation_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithPerformanceObservations
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithPerformanceObservations

theorem runtime_performance_observation_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem runtime_performance_observation_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_core_and_sound
      assumptions
      summary
      publicInput
      proof
      observed

theorem runtime_performance_timing_observations_metadata_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary)
    (observations : List TimingObservation) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance
        system
        { summary with timingObservations := observations }
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_acceptance_sound
      assumptions
      { summary with timingObservations := observations }
      publicInput
      proof
      observed

theorem runtime_performance_timing_observations_metadata_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary)
    (observations : List TimingObservation) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance
        system
        { summary with timingObservations := observations }
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_acceptance_verifier_core_contract
      assumptions
      { summary with timingObservations := observations }
      publicInput
      proof
      observed

theorem runtime_performance_timing_observations_metadata_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary)
    (observations : List TimingObservation) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance
        system
        { summary with timingObservations := observations }
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_acceptance_core_and_sound
      assumptions
      { summary with timingObservations := observations }
      publicInput
      proof
      observed

theorem runtime_performance_observation_projects_timing_observations
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        TimingObservedAcceptance
          system
          summary.timingObservations
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_timing_observations_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.timingObservations)
      publicInput
      proof
      (runtime_performance_observation_projects_timing_observations
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_timing_observations_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.timingObservations)
      publicInput
      proof
      (runtime_performance_observation_projects_timing_observations
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_timing_observations_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.timingObservations)
      publicInput
      proof
      (runtime_performance_observation_projects_timing_observations
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_guest_pc_trace_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        GuestPcTraceTimingObservedAcceptance
          system
          summary.guestPcTraceTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_guest_pc_trace_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.guestPcTraceTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_guest_pc_trace_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_guest_pc_trace_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.guestPcTraceTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_guest_pc_trace_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_guest_pc_trace_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.guestPcTraceTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_guest_pc_trace_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_witness_opening_row_value_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        WitnessOpeningRowValueTimingObservedAcceptance
          system
          summary.witnessOpeningRowValueTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_row_value_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.witnessOpeningRowValueTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_witness_opening_row_value_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_row_value_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.witnessOpeningRowValueTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_witness_opening_row_value_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_row_value_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.witnessOpeningRowValueTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_witness_opening_row_value_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_constant_material_validation_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ConstantMaterialValidationTimingObservedAcceptance
          system
          summary.constantMaterialValidationTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_constant_material_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.constantMaterialValidationTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_constant_material_validation_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_constant_material_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.constantMaterialValidationTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_constant_material_validation_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_constant_material_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.constantMaterialValidationTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_constant_material_validation_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_prover_gpu_mode
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ProverGpuModeObservedAcceptance
          system
          summary.proverGpuMode
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_prover_gpu_mode_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.proverGpuMode)
      publicInput
      proof
      (runtime_performance_observation_projects_prover_gpu_mode
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_prover_gpu_mode_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.proverGpuMode)
      publicInput
      proof
      (runtime_performance_observation_projects_prover_gpu_mode
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_prover_gpu_mode_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.proverGpuMode)
      publicInput
      proof
      (runtime_performance_observation_projects_prover_gpu_mode
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_gpu_run_options
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        GpuRunOptionsObservedAcceptance
          system
          summary.gpuRunOptions
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_gpu_run_options_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.gpuRunOptions)
      publicInput
      proof
      (runtime_performance_observation_projects_gpu_run_options
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_gpu_run_options_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.gpuRunOptions)
      publicInput
      proof
      (runtime_performance_observation_projects_gpu_run_options
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_gpu_run_options_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.gpuRunOptions)
      publicInput
      proof
      (runtime_performance_observation_projects_gpu_run_options
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_cuda_backend
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        CudaBackendObservedAcceptance
          system
          summary.cudaBackend
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_cuda_backend_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.cudaBackend)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_backend
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_cuda_backend_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.cudaBackend)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_backend
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_cuda_backend_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.cudaBackend)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_backend
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_cuda_allocator_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        CudaAllocatorTimingObservedAcceptance
          system
          summary.cudaAllocatorTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_cuda_allocator_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.cudaAllocatorTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_allocator_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_cuda_allocator_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.cudaAllocatorTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_allocator_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_cuda_allocator_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.cudaAllocatorTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_cuda_allocator_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_proof_artifact_finish_timing
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ProofArtifactFinishTimingObservedAcceptance
          system
          summary.proofArtifactFinishTiming
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_finish_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.proofArtifactFinishTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_artifact_finish_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_finish_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.proofArtifactFinishTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_artifact_finish_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_finish_timing_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.proofArtifactFinishTiming)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_artifact_finish_timing
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_projects_proof_timing_batch
    {system : VerifierModel}
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        ProofTimingBatchObservedAcceptance
          system
          summary.proofTimingBatch
          publicInput
          proof := by
  intro publicInput proof observed
  exact observed

theorem runtime_performance_observation_proof_timing_batch_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_sound
      assumptions
      summary
      (fun summary => summary.proofTimingBatch)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_timing_batch
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_proof_timing_batch_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      (fun summary => summary.proofTimingBatch)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_timing_batch
        summary
        publicInput
        proof
        observed)

theorem runtime_performance_observation_proof_timing_batch_acceptance_core_and_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof
          /\ SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    runtime_performance_observation_projected_metadata_acceptance_core_and_sound
      assumptions
      summary
      (fun summary => summary.proofTimingBatch)
      publicInput
      proof
      (runtime_performance_observation_projects_proof_timing_batch
        summary
        publicInput
        proof
        observed)

structure RuntimePerformanceObservationProjectedCoreContracts
    (system : VerifierModel)
    (publicInput : PublicInput)
    (proof : Proof) : Prop where
  timingObservations :
    RuntimeVerifierCoreContract system publicInput proof
  guestPcTraceTiming :
    RuntimeVerifierCoreContract system publicInput proof
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

theorem runtime_performance_observation_projected_core_contracts
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : RuntimePerformanceObservationSummary) :
    forall publicInput proof,
      RuntimePerformanceObservedAcceptance system summary publicInput proof ->
        RuntimePerformanceObservationProjectedCoreContracts system publicInput proof := by
  intro publicInput proof observed
  exact
    { timingObservations :=
        runtime_performance_observation_timing_observations_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      guestPcTraceTiming :=
        runtime_performance_observation_guest_pc_trace_timing_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      witnessOpeningRowValueTiming :=
        runtime_performance_observation_row_value_timing_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      constantMaterialValidationTiming :=
        runtime_performance_observation_constant_material_timing_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      proverGpuMode :=
        runtime_performance_observation_prover_gpu_mode_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      gpuRunOptions :=
        runtime_performance_observation_gpu_run_options_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      cudaBackend :=
        runtime_performance_observation_cuda_backend_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      cudaAllocatorTiming :=
        runtime_performance_observation_cuda_allocator_timing_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      proofArtifactFinishTiming :=
        runtime_performance_observation_finish_timing_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed
      proofTimingBatch :=
        runtime_performance_observation_proof_timing_batch_acceptance_verifier_core_contract
          assumptions
          summary
          publicInput
          proof
          observed }

end Lzvm
