/-
Copyright (c) 2026 Sihao Liu. All rights reserved.
Released under MIT OR Apache-2.0 license.
Authors: Sihao Liu
-/

import Lzvm.AuxiliaryChecks

/-!
Runtime proof-finish timing observation contracts.
-/

namespace Lzvm

def ProofTimingBatchObservedAcceptance
    (system : VerifierModel)
    (summary : Option ProofTimingBatchSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem proof_timing_batch_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ProofTimingBatchSummary) :
    forall publicInput proof,
      ProofTimingBatchObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem proof_timing_batch_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofTimingBatchSummary) :
    forall publicInput proof,
      ProofTimingBatchObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      observed

theorem proof_timing_batch_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofTimingBatchSummary) :
    forall publicInput proof,
      ProofTimingBatchObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

def WitnessOpeningRowValueTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option WitnessOpeningRowValueTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem witness_opening_row_value_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem witness_opening_row_value_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithRowValueTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithRowValueTimings

theorem witness_opening_row_value_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option WitnessOpeningRowValueTimingSummary) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem witness_opening_row_value_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : WitnessOpeningRowValueTimingSummary)
    (sourceExtendMilliseconds sourceDownloadMilliseconds deviceDownloadMilliseconds
      deviceRows deviceDownloadBatches deviceSingleDownloads sourceRows words bytes : Nat) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance
        system
        (some
          { summary with
            rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
            rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
            rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
            deviceRowCount := deviceRows
            deviceDownloadBatchCount := deviceDownloadBatches
            deviceSingleDownloadCount := deviceSingleDownloads
            sourceRowCount := sourceRows
            wordCount := words
            byteCount := bytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    witness_opening_row_value_timing_acceptance_sound
      assumptions
      (some
        { summary with
          rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
          rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
          rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
          deviceRowCount := deviceRows
          deviceDownloadBatchCount := deviceDownloadBatches
          deviceSingleDownloadCount := deviceSingleDownloads
          sourceRowCount := sourceRows
          wordCount := words
          byteCount := bytes })
      publicInput
      proof
      observed

theorem witness_opening_row_value_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : WitnessOpeningRowValueTimingSummary)
    (sourceExtendMilliseconds sourceDownloadMilliseconds deviceDownloadMilliseconds
      deviceRows deviceDownloadBatches deviceSingleDownloads sourceRows words bytes : Nat) :
    forall publicInput proof,
      WitnessOpeningRowValueTimingObservedAcceptance
        system
        (some
          { summary with
            rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
            rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
            rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
            deviceRowCount := deviceRows
            deviceDownloadBatchCount := deviceDownloadBatches
            deviceSingleDownloadCount := deviceSingleDownloads
            sourceRowCount := sourceRows
            wordCount := words
            byteCount := bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    witness_opening_row_value_timing_acceptance_verifier_core_contract
      assumptions
      (some
        { summary with
          rowValueSourceExtendMilliseconds := sourceExtendMilliseconds
          rowValueSourceDownloadMilliseconds := sourceDownloadMilliseconds
          rowValueDeviceDownloadMilliseconds := deviceDownloadMilliseconds
          deviceRowCount := deviceRows
          deviceDownloadBatchCount := deviceDownloadBatches
          deviceSingleDownloadCount := deviceSingleDownloads
          sourceRowCount := sourceRows
          wordCount := words
          byteCount := bytes })
      publicInput
      proof
      observed

def ConstantMaterialValidationTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option ConstantMaterialValidationTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem constant_material_validation_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem constant_material_validation_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithConstantMaterialTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithConstantMaterialTimings

theorem constant_material_validation_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ConstantMaterialValidationTimingSummary) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem constant_material_validation_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ConstantMaterialValidationTimingSummary)
    (elapsedMilliseconds joinWaitMilliseconds unitCount byteCount : Nat) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance
        system
        (some
          { summary with
            constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
            constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
            constantMaterialValidationUnitCount := unitCount
            constantMaterialValidationByteCount := byteCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    constant_material_validation_timing_acceptance_sound
      assumptions
      (some
        { summary with
          constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
          constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
          constantMaterialValidationUnitCount := unitCount
          constantMaterialValidationByteCount := byteCount })
      publicInput
      proof
      observed

theorem constant_material_validation_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ConstantMaterialValidationTimingSummary)
    (elapsedMilliseconds joinWaitMilliseconds unitCount byteCount : Nat) :
    forall publicInput proof,
      ConstantMaterialValidationTimingObservedAcceptance
        system
        (some
          { summary with
            constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
            constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
            constantMaterialValidationUnitCount := unitCount
            constantMaterialValidationByteCount := byteCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    constant_material_validation_timing_acceptance_verifier_core_contract
      assumptions
      (some
        { summary with
          constantMaterialValidationElapsedMilliseconds := elapsedMilliseconds
          constantMaterialValidationJoinWaitMilliseconds := joinWaitMilliseconds
          constantMaterialValidationUnitCount := unitCount
          constantMaterialValidationByteCount := byteCount })
      publicInput
      proof
      observed

def ProverGpuModeObservedAcceptance
    (system : VerifierModel)
    (summary : Option ProverGpuModeSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem prover_gpu_mode_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem prover_gpu_mode_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuMode
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithGpuMode

theorem prover_gpu_mode_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProverGpuModeSummary) :
    forall publicInput proof,
      ProverGpuModeObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

def GpuRunOptionsObservedAcceptance
    (system : VerifierModel)
    (summary : Option GpuRunOptionsSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem gpu_run_options_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem gpu_run_options_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithGpuRunOptions
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithGpuRunOptions

theorem gpu_run_options_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option GpuRunOptionsSummary) :
    forall publicInput proof,
      GpuRunOptionsObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

def CudaBackendObservedAcceptance
    (system : VerifierModel)
    (summary : Option CudaBackendSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem cuda_backend_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem cuda_backend_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithCudaBackend
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithCudaBackend

theorem cuda_backend_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaBackendSummary) :
    forall publicInput proof,
      CudaBackendObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

def CudaAllocatorTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option CudaAllocatorTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem cuda_allocator_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem cuda_allocator_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithAllocatorTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithAllocatorTimings

theorem cuda_allocator_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option CudaAllocatorTimingSummary) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem cuda_allocator_aggregate_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (mallocCalls mallocBytes mallocWaitNanoseconds mallocMaxWaitNanoseconds
      hostRegisterCalls hostRegisterBytes hostRegisterWaitNanoseconds
      hostRegisterMaxWaitNanoseconds hostUnregisterCalls
      hostUnregisterWaitNanoseconds hostUnregisterMaxWaitNanoseconds
      deviceSynchronizeCalls deviceSynchronizeWaitNanoseconds
      deviceSynchronizeMaxWaitNanoseconds cachedBlocks cachedBytes eventQueryCalls
      eventQueryReadyCount eventQueryNotReadyCount eventSynchronizeCalls
      eventSynchronizeBytes eventSynchronizeMaxBytes eventSynchronizeWaitNanoseconds
      eventSynchronizeMaxWaitNanoseconds eventSynchronizeHotBytes
      eventSynchronizeHotCount eventSynchronizeHotWaitNanoseconds cachedReuseCount
      pendingReuseCount noWaitBypassCount noWaitBypassBytes : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorMallocCallCount := mallocCalls
            cudaAllocatorMallocByteCount := mallocBytes
            cudaAllocatorMallocWaitNanoseconds := mallocWaitNanoseconds
            cudaAllocatorMallocMaxWaitNanoseconds := mallocMaxWaitNanoseconds
            cudaAllocatorHostRegisterCallCount := hostRegisterCalls
            cudaAllocatorHostRegisterByteCount := hostRegisterBytes
            cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
            cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
            cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
            cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
            cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
              hostUnregisterMaxWaitNanoseconds
            cudaAllocatorDeviceSynchronizeCallCount := deviceSynchronizeCalls
            cudaAllocatorDeviceSynchronizeWaitNanoseconds :=
              deviceSynchronizeWaitNanoseconds
            cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds :=
              deviceSynchronizeMaxWaitNanoseconds
            cudaAllocatorCachedBlockCount := cachedBlocks
            cudaAllocatorCachedByteCount := cachedBytes
            cudaAllocatorEventQueryCallCount := eventQueryCalls
            cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
            cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
            cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
            cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
            cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
            cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
            cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
              eventSynchronizeMaxWaitNanoseconds
            cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
            cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
            cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
              eventSynchronizeHotWaitNanoseconds
            cudaAllocatorCachedReuseCount := cachedReuseCount
            cudaAllocatorPendingReuseCount := pendingReuseCount
            cudaAllocatorNoWaitBypassCount := noWaitBypassCount
            cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    cuda_allocator_timing_acceptance_sound
      assumptions
      (some
        { summary with
          cudaAllocatorMallocCallCount := mallocCalls
          cudaAllocatorMallocByteCount := mallocBytes
          cudaAllocatorMallocWaitNanoseconds := mallocWaitNanoseconds
          cudaAllocatorMallocMaxWaitNanoseconds := mallocMaxWaitNanoseconds
          cudaAllocatorHostRegisterCallCount := hostRegisterCalls
          cudaAllocatorHostRegisterByteCount := hostRegisterBytes
          cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
          cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
          cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
          cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
          cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
            hostUnregisterMaxWaitNanoseconds
          cudaAllocatorDeviceSynchronizeCallCount := deviceSynchronizeCalls
          cudaAllocatorDeviceSynchronizeWaitNanoseconds :=
            deviceSynchronizeWaitNanoseconds
          cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds :=
            deviceSynchronizeMaxWaitNanoseconds
          cudaAllocatorCachedBlockCount := cachedBlocks
          cudaAllocatorCachedByteCount := cachedBytes
          cudaAllocatorEventQueryCallCount := eventQueryCalls
          cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
          cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
          cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
          cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
          cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
          cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
          cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
            eventSynchronizeMaxWaitNanoseconds
          cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
          cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
          cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
            eventSynchronizeHotWaitNanoseconds
          cudaAllocatorCachedReuseCount := cachedReuseCount
          cudaAllocatorPendingReuseCount := pendingReuseCount
          cudaAllocatorNoWaitBypassCount := noWaitBypassCount
          cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
      publicInput
      proof
      observed

theorem cuda_allocator_aggregate_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (mallocCalls mallocBytes mallocWaitNanoseconds mallocMaxWaitNanoseconds
      hostRegisterCalls hostRegisterBytes hostRegisterWaitNanoseconds
      hostRegisterMaxWaitNanoseconds hostUnregisterCalls
      hostUnregisterWaitNanoseconds hostUnregisterMaxWaitNanoseconds
      deviceSynchronizeCalls deviceSynchronizeWaitNanoseconds
      deviceSynchronizeMaxWaitNanoseconds cachedBlocks cachedBytes eventQueryCalls
      eventQueryReadyCount eventQueryNotReadyCount eventSynchronizeCalls
      eventSynchronizeBytes eventSynchronizeMaxBytes eventSynchronizeWaitNanoseconds
      eventSynchronizeMaxWaitNanoseconds eventSynchronizeHotBytes
      eventSynchronizeHotCount eventSynchronizeHotWaitNanoseconds cachedReuseCount
      pendingReuseCount noWaitBypassCount noWaitBypassBytes : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorMallocCallCount := mallocCalls
            cudaAllocatorMallocByteCount := mallocBytes
            cudaAllocatorMallocWaitNanoseconds := mallocWaitNanoseconds
            cudaAllocatorMallocMaxWaitNanoseconds := mallocMaxWaitNanoseconds
            cudaAllocatorHostRegisterCallCount := hostRegisterCalls
            cudaAllocatorHostRegisterByteCount := hostRegisterBytes
            cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
            cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
            cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
            cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
            cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
              hostUnregisterMaxWaitNanoseconds
            cudaAllocatorDeviceSynchronizeCallCount := deviceSynchronizeCalls
            cudaAllocatorDeviceSynchronizeWaitNanoseconds :=
              deviceSynchronizeWaitNanoseconds
            cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds :=
              deviceSynchronizeMaxWaitNanoseconds
            cudaAllocatorCachedBlockCount := cachedBlocks
            cudaAllocatorCachedByteCount := cachedBytes
            cudaAllocatorEventQueryCallCount := eventQueryCalls
            cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
            cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
            cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
            cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
            cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
            cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
            cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
              eventSynchronizeMaxWaitNanoseconds
            cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
            cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
            cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
              eventSynchronizeHotWaitNanoseconds
            cudaAllocatorCachedReuseCount := cachedReuseCount
            cudaAllocatorPendingReuseCount := pendingReuseCount
            cudaAllocatorNoWaitBypassCount := noWaitBypassCount
            cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    cuda_allocator_timing_acceptance_verifier_core_contract
      assumptions
      (some
        { summary with
          cudaAllocatorMallocCallCount := mallocCalls
          cudaAllocatorMallocByteCount := mallocBytes
          cudaAllocatorMallocWaitNanoseconds := mallocWaitNanoseconds
          cudaAllocatorMallocMaxWaitNanoseconds := mallocMaxWaitNanoseconds
          cudaAllocatorHostRegisterCallCount := hostRegisterCalls
          cudaAllocatorHostRegisterByteCount := hostRegisterBytes
          cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
          cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
          cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
          cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
          cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
            hostUnregisterMaxWaitNanoseconds
          cudaAllocatorDeviceSynchronizeCallCount := deviceSynchronizeCalls
          cudaAllocatorDeviceSynchronizeWaitNanoseconds :=
            deviceSynchronizeWaitNanoseconds
          cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds :=
            deviceSynchronizeMaxWaitNanoseconds
          cudaAllocatorCachedBlockCount := cachedBlocks
          cudaAllocatorCachedByteCount := cachedBytes
          cudaAllocatorEventQueryCallCount := eventQueryCalls
          cudaAllocatorEventQueryReadyCount := eventQueryReadyCount
          cudaAllocatorEventQueryNotReadyCount := eventQueryNotReadyCount
          cudaAllocatorEventSynchronizeCallCount := eventSynchronizeCalls
          cudaAllocatorEventSynchronizeByteCount := eventSynchronizeBytes
          cudaAllocatorEventSynchronizeMaxByteCount := eventSynchronizeMaxBytes
          cudaAllocatorEventSynchronizeWaitNanoseconds := eventSynchronizeWaitNanoseconds
          cudaAllocatorEventSynchronizeMaxWaitNanoseconds :=
            eventSynchronizeMaxWaitNanoseconds
          cudaAllocatorEventSynchronizeHotByteCount := eventSynchronizeHotBytes
          cudaAllocatorEventSynchronizeHotCount := eventSynchronizeHotCount
          cudaAllocatorEventSynchronizeHotWaitNanoseconds :=
            eventSynchronizeHotWaitNanoseconds
          cudaAllocatorCachedReuseCount := cachedReuseCount
          cudaAllocatorPendingReuseCount := pendingReuseCount
          cudaAllocatorNoWaitBypassCount := noWaitBypassCount
          cudaAllocatorNoWaitBypassByteCount := noWaitBypassBytes })
      publicInput
      proof
      observed

theorem cuda_allocator_host_registration_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (hostRegisterCalls hostRegisterBytes hostRegisterWaitNanoseconds
      hostRegisterMaxWaitNanoseconds hostUnregisterCalls
      hostUnregisterWaitNanoseconds hostUnregisterMaxWaitNanoseconds : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorHostRegisterCallCount := hostRegisterCalls
            cudaAllocatorHostRegisterByteCount := hostRegisterBytes
            cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
            cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
            cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
            cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
            cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
              hostUnregisterMaxWaitNanoseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    cuda_allocator_timing_acceptance_sound
      assumptions
      (some
        { summary with
          cudaAllocatorHostRegisterCallCount := hostRegisterCalls
          cudaAllocatorHostRegisterByteCount := hostRegisterBytes
          cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
          cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
          cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
          cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
          cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
            hostUnregisterMaxWaitNanoseconds })
      publicInput
      proof
      observed

theorem cuda_allocator_host_registration_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : CudaAllocatorTimingSummary)
    (hostRegisterCalls hostRegisterBytes hostRegisterWaitNanoseconds
      hostRegisterMaxWaitNanoseconds hostUnregisterCalls
      hostUnregisterWaitNanoseconds hostUnregisterMaxWaitNanoseconds : Nat) :
    forall publicInput proof,
      CudaAllocatorTimingObservedAcceptance
        system
        (some
          { summary with
            cudaAllocatorHostRegisterCallCount := hostRegisterCalls
            cudaAllocatorHostRegisterByteCount := hostRegisterBytes
            cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
            cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
            cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
            cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
            cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
              hostUnregisterMaxWaitNanoseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    cuda_allocator_timing_acceptance_verifier_core_contract
      assumptions
      (some
        { summary with
          cudaAllocatorHostRegisterCallCount := hostRegisterCalls
          cudaAllocatorHostRegisterByteCount := hostRegisterBytes
          cudaAllocatorHostRegisterWaitNanoseconds := hostRegisterWaitNanoseconds
          cudaAllocatorHostRegisterMaxWaitNanoseconds := hostRegisterMaxWaitNanoseconds
          cudaAllocatorHostUnregisterCallCount := hostUnregisterCalls
          cudaAllocatorHostUnregisterWaitNanoseconds := hostUnregisterWaitNanoseconds
          cudaAllocatorHostUnregisterMaxWaitNanoseconds :=
            hostUnregisterMaxWaitNanoseconds })
      publicInput
      proof
      observed

def ProofArtifactFinishTimingObservedAcceptance
    (system : VerifierModel)
    (summary : Option ProofArtifactFinishTimingSummary)
    (publicInput : PublicInput)
    (proof : Proof) : Prop :=
  IgnoredMetadataObservedAcceptance system summary publicInput proof

theorem proof_artifact_finish_timing_observed_acceptance_projects_verifier_acceptance
    {system : VerifierModel}
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        system.accepts publicInput proof := by
  intro publicInput proof observed
  exact observed

theorem proof_artifact_finish_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof acceptedWithProofFinishTimings
  exact
    ignored_metadata_acceptance_sound
      assumptions
      summary
      publicInput
      proof
      acceptedWithProofFinishTimings

theorem proof_artifact_finish_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : Option ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system summary publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    ignored_metadata_acceptance_verifier_core_contract
      assumptions
      summary
      publicInput
      proof
      observed

theorem proof_artifact_finish_timing_some_summary_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        SoundWitness system publicInput proof :=
  proof_artifact_finish_timing_acceptance_sound assumptions (some summary)

theorem proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance system (some summary) publicInput proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_acceptance_verifier_core_contract
      assumptions
      (some summary)
      publicInput
      proof
      observed

theorem proof_artifact_finish_top_level_timing_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
      friOpeningMilliseconds friTranscriptUnitBuildMilliseconds
      friTranscriptLayerTreeMilliseconds friTranscriptFoldMilliseconds proofEncodeMilliseconds
      contributionSegmentMilliseconds contributionVerifyMilliseconds
      contributionChallengeMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishQueryPlanMilliseconds := queryPlanMilliseconds
            finishConstantOpeningMilliseconds := constantOpeningMilliseconds
            finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
            finishFriOpeningMilliseconds := friOpeningMilliseconds
            finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
            finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
            finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
            finishProofEncodeMilliseconds := proofEncodeMilliseconds
            finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
            finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
            finishContributionChallengeMilliseconds := contributionChallengeMilliseconds })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishQueryPlanMilliseconds := queryPlanMilliseconds
        finishConstantOpeningMilliseconds := constantOpeningMilliseconds
        finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
        finishFriOpeningMilliseconds := friOpeningMilliseconds
        finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
        finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
        finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
        finishProofEncodeMilliseconds := proofEncodeMilliseconds
        finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
        finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
        finishContributionChallengeMilliseconds := contributionChallengeMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_top_level_timing_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryPlanMilliseconds constantOpeningMilliseconds witnessOpeningMilliseconds
      friOpeningMilliseconds friTranscriptUnitBuildMilliseconds
      friTranscriptLayerTreeMilliseconds friTranscriptFoldMilliseconds proofEncodeMilliseconds
      contributionSegmentMilliseconds contributionVerifyMilliseconds
      contributionChallengeMilliseconds : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishQueryPlanMilliseconds := queryPlanMilliseconds
            finishConstantOpeningMilliseconds := constantOpeningMilliseconds
            finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
            finishFriOpeningMilliseconds := friOpeningMilliseconds
            finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
            finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
            finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
            finishProofEncodeMilliseconds := proofEncodeMilliseconds
            finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
            finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
            finishContributionChallengeMilliseconds := contributionChallengeMilliseconds })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishQueryPlanMilliseconds := queryPlanMilliseconds
        finishConstantOpeningMilliseconds := constantOpeningMilliseconds
        finishWitnessOpeningMilliseconds := witnessOpeningMilliseconds
        finishFriOpeningMilliseconds := friOpeningMilliseconds
        finishFriTranscriptUnitBuildMilliseconds := friTranscriptUnitBuildMilliseconds
        finishFriTranscriptLayerTreeMilliseconds := friTranscriptLayerTreeMilliseconds
        finishFriTranscriptFoldMilliseconds := friTranscriptFoldMilliseconds
        finishProofEncodeMilliseconds := proofEncodeMilliseconds
        finishContributionSegmentMilliseconds := contributionSegmentMilliseconds
        finishContributionVerifyMilliseconds := contributionVerifyMilliseconds
        finishContributionChallengeMilliseconds := contributionChallengeMilliseconds }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryCount queryUnitCount singleQueryUnitCount maxQueriesPerUnit stageCount
      retainedSourceCount externalSourceCount embeddedSourceCount missingSourceCount
      retainedLeafDigestOpeningCount retainedLeafDigestOpeningRowCount
      retainedParentCheckpointOpeningCount retainedParentCheckpointOpeningRowCount
      rowDedupInputRowCount rowDedupUniqueRowCount rowDedupElidedRowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningQueryCount := queryCount
            finishWitnessOpeningQueryUnitCount := queryUnitCount
            finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
            finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
            finishWitnessOpeningStageCount := stageCount
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := externalSourceCount
            finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
            finishWitnessOpeningMissingSourceCount := missingSourceCount
            finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
            finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
              retainedLeafDigestOpeningRowCount
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
            finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
            finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningQueryCount := queryCount
        finishWitnessOpeningQueryUnitCount := queryUnitCount
        finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
        finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
        finishWitnessOpeningStageCount := stageCount
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := externalSourceCount
        finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
        finishWitnessOpeningMissingSourceCount := missingSourceCount
        finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
        finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
          retainedLeafDigestOpeningRowCount
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
        finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
        finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_witness_opening_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (queryCount queryUnitCount singleQueryUnitCount maxQueriesPerUnit stageCount
      retainedSourceCount externalSourceCount embeddedSourceCount missingSourceCount
      retainedLeafDigestOpeningCount retainedLeafDigestOpeningRowCount
      retainedParentCheckpointOpeningCount retainedParentCheckpointOpeningRowCount
      rowDedupInputRowCount rowDedupUniqueRowCount rowDedupElidedRowCount : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningQueryCount := queryCount
            finishWitnessOpeningQueryUnitCount := queryUnitCount
            finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
            finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
            finishWitnessOpeningStageCount := stageCount
            finishWitnessOpeningRetainedSourceCount := retainedSourceCount
            finishWitnessOpeningExternalSourceCount := externalSourceCount
            finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
            finishWitnessOpeningMissingSourceCount := missingSourceCount
            finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
            finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
              retainedLeafDigestOpeningRowCount
            finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
              retainedParentCheckpointOpeningCount
            finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
              retainedParentCheckpointOpeningRowCount
            finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
            finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
            finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningQueryCount := queryCount
        finishWitnessOpeningQueryUnitCount := queryUnitCount
        finishWitnessOpeningSingleQueryUnitCount := singleQueryUnitCount
        finishWitnessOpeningMaxQueriesPerUnit := maxQueriesPerUnit
        finishWitnessOpeningStageCount := stageCount
        finishWitnessOpeningRetainedSourceCount := retainedSourceCount
        finishWitnessOpeningExternalSourceCount := externalSourceCount
        finishWitnessOpeningEmbeddedSourceCount := embeddedSourceCount
        finishWitnessOpeningMissingSourceCount := missingSourceCount
        finishWitnessOpeningRetainedLeafDigestOpeningCount := retainedLeafDigestOpeningCount
        finishWitnessOpeningRetainedLeafDigestOpeningRowCount :=
          retainedLeafDigestOpeningRowCount
        finishWitnessOpeningRetainedParentCheckpointOpeningCount :=
          retainedParentCheckpointOpeningCount
        finishWitnessOpeningRetainedParentCheckpointOpeningRowCount :=
          retainedParentCheckpointOpeningRowCount
        finishWitnessOpeningRowDedupInputRowCount := rowDedupInputRowCount
        finishWitnessOpeningRowDedupUniqueRowCount := rowDedupUniqueRowCount
        finishWitnessOpeningRowDedupElidedRowCount := rowDedupElidedRowCount }
      publicInput
      proof
      observed

theorem proof_artifact_finish_leaf_work_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (leafHashRows leafHashBytes leafHashArity2Rows leafHashArity2Bytes
      leafHashArity4Rows leafHashArity4Bytes leafCosetCalls leafCosetOutputBytes
      leafCosetColumns leafCosetMaxColumns leafCosetNttLaunches
      leafCosetBitReverseLaunches leafCosetNttStageLaunches
      leafCosetNttBlockTwiddleLaunches leafCosetNormalizeLaunches
      leafCosetPackLaunches leafCosetUnpackLaunches : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningLeafHashRowCount := leafHashRows
            finishWitnessOpeningLeafHashByteCount := leafHashBytes
            finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
            finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
            finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
            finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
            finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
            finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
            finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
            finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
            finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
            finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
              leafCosetBitReverseLaunches
            finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
              leafCosetNttStageLaunches
            finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
              leafCosetNttBlockTwiddleLaunches
            finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount :=
              leafCosetNormalizeLaunches
            finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
            finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningLeafHashRowCount := leafHashRows
        finishWitnessOpeningLeafHashByteCount := leafHashBytes
        finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
        finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
        finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
        finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
        finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
        finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
        finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
        finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
        finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
        finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
          leafCosetBitReverseLaunches
        finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
          leafCosetNttStageLaunches
        finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
          leafCosetNttBlockTwiddleLaunches
        finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount := leafCosetNormalizeLaunches
        finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
        finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_leaf_work_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (leafHashRows leafHashBytes leafHashArity2Rows leafHashArity2Bytes
      leafHashArity4Rows leafHashArity4Bytes leafCosetCalls leafCosetOutputBytes
      leafCosetColumns leafCosetMaxColumns leafCosetNttLaunches
      leafCosetBitReverseLaunches leafCosetNttStageLaunches
      leafCosetNttBlockTwiddleLaunches leafCosetNormalizeLaunches
      leafCosetPackLaunches leafCosetUnpackLaunches : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningLeafHashRowCount := leafHashRows
            finishWitnessOpeningLeafHashByteCount := leafHashBytes
            finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
            finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
            finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
            finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
            finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
            finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
            finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
            finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
            finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
            finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
              leafCosetBitReverseLaunches
            finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
              leafCosetNttStageLaunches
            finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
              leafCosetNttBlockTwiddleLaunches
            finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount :=
              leafCosetNormalizeLaunches
            finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
            finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningLeafHashRowCount := leafHashRows
        finishWitnessOpeningLeafHashByteCount := leafHashBytes
        finishWitnessOpeningLeafHashArity2RowCount := leafHashArity2Rows
        finishWitnessOpeningLeafHashArity2ByteCount := leafHashArity2Bytes
        finishWitnessOpeningLeafHashArity4RowCount := leafHashArity4Rows
        finishWitnessOpeningLeafHashArity4ByteCount := leafHashArity4Bytes
        finishWitnessOpeningLeafCosetExtendCallCount := leafCosetCalls
        finishWitnessOpeningLeafCosetExtendOutputByteCount := leafCosetOutputBytes
        finishWitnessOpeningLeafCosetExtendColumnCount := leafCosetColumns
        finishWitnessOpeningLeafCosetExtendMaxColumnCount := leafCosetMaxColumns
        finishWitnessOpeningLeafCosetExtendNttLaunchCount := leafCosetNttLaunches
        finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount :=
          leafCosetBitReverseLaunches
        finishWitnessOpeningLeafCosetExtendNttStageLaunchCount :=
          leafCosetNttStageLaunches
        finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount :=
          leafCosetNttBlockTwiddleLaunches
        finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount := leafCosetNormalizeLaunches
        finishWitnessOpeningLeafCosetExtendPackLaunchCount := leafCosetPackLaunches
        finishWitnessOpeningLeafCosetExtendUnpackLaunchCount := leafCosetUnpackLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (parentHashRows parentHashBytes parentHashLaunches
      recomputedRows recomputedBytes recomputedLaunches
      retainedLeafDigestRows retainedLeafDigestBytes retainedLeafDigestLaunches
      retainedCheckpointPrefixRows retainedCheckpointPrefixBytes retainedCheckpointPrefixLaunches
      retainedCheckpointSuffixRows retainedCheckpointSuffixBytes retainedCheckpointSuffixLaunches
      : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowCount := parentHashRows
            finishWitnessOpeningPathParentHashByteCount := parentHashBytes
            finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
            finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
            finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
            finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
            finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
              retainedLeafDigestRows
            finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
              retainedLeafDigestBytes
            finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
              retainedLeafDigestLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
              retainedCheckpointPrefixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
              retainedCheckpointPrefixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
              retainedCheckpointPrefixLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
              retainedCheckpointSuffixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
              retainedCheckpointSuffixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
              retainedCheckpointSuffixLaunches })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowCount := parentHashRows
        finishWitnessOpeningPathParentHashByteCount := parentHashBytes
        finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
        finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
        finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
        finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
        finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
          retainedLeafDigestRows
        finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
          retainedLeafDigestBytes
        finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
          retainedLeafDigestLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
          retainedCheckpointPrefixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
          retainedCheckpointPrefixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
          retainedCheckpointPrefixLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
          retainedCheckpointSuffixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
          retainedCheckpointSuffixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
          retainedCheckpointSuffixLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (parentHashRows parentHashBytes parentHashLaunches
      recomputedRows recomputedBytes recomputedLaunches
      retainedLeafDigestRows retainedLeafDigestBytes retainedLeafDigestLaunches
      retainedCheckpointPrefixRows retainedCheckpointPrefixBytes retainedCheckpointPrefixLaunches
      retainedCheckpointSuffixRows retainedCheckpointSuffixBytes retainedCheckpointSuffixLaunches
      : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowCount := parentHashRows
            finishWitnessOpeningPathParentHashByteCount := parentHashBytes
            finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
            finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
            finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
            finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
            finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
              retainedLeafDigestRows
            finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
              retainedLeafDigestBytes
            finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
              retainedLeafDigestLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
              retainedCheckpointPrefixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
              retainedCheckpointPrefixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
              retainedCheckpointPrefixLaunches
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
              retainedCheckpointSuffixRows
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
              retainedCheckpointSuffixBytes
            finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
              retainedCheckpointSuffixLaunches })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowCount := parentHashRows
        finishWitnessOpeningPathParentHashByteCount := parentHashBytes
        finishWitnessOpeningPathParentHashLaunchCount := parentHashLaunches
        finishWitnessOpeningPathParentHashRecomputedRowCount := recomputedRows
        finishWitnessOpeningPathParentHashRecomputedByteCount := recomputedBytes
        finishWitnessOpeningPathParentHashRecomputedLaunchCount := recomputedLaunches
        finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount :=
          retainedLeafDigestRows
        finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount :=
          retainedLeafDigestBytes
        finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount :=
          retainedLeafDigestLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount :=
          retainedCheckpointPrefixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount :=
          retainedCheckpointPrefixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount :=
          retainedCheckpointPrefixLaunches
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount :=
          retainedCheckpointSuffixRows
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount :=
          retainedCheckpointSuffixBytes
        finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount :=
          retainedCheckpointSuffixLaunches }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowsPerQuery rowsPerStage launchesPerStage : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
            finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
            finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowsPerQuery rowsPerStage launchesPerStage : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
            finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
            finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningPathParentHashRowsPerQuery := rowsPerQuery
        finishWitnessOpeningPathParentHashRowsPerStage := rowsPerStage
        finishWitnessOpeningPathParentHashLaunchesPerStage := launchesPerStage }
      publicInput
      proof
      observed

theorem proof_artifact_finish_row_values_shape_acceptance_sound
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowValuesMilliseconds sourceExtendMilliseconds sourceDownloadMilliseconds
      deviceDownloadMilliseconds deviceRows deviceDownloadBatches deviceSingleDownloads
      sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
              deviceDownloadMilliseconds
            finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
              deviceDownloadBatches
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
              deviceSingleDownloads
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        SoundWitness system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_sound
      assumptions
      { summary with
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
          deviceDownloadMilliseconds
        finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
          deviceDownloadBatches
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
          deviceSingleDownloads
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed

theorem proof_artifact_finish_row_values_shape_acceptance_verifier_core_contract
    {system : VerifierModel}
    (assumptions : AssumptionBundle system)
    (summary : ProofArtifactFinishTimingSummary)
    (rowValuesMilliseconds sourceExtendMilliseconds sourceDownloadMilliseconds
      deviceDownloadMilliseconds deviceRows deviceDownloadBatches deviceSingleDownloads
      sourceRows words bytes : Nat) :
    forall publicInput proof,
      ProofArtifactFinishTimingObservedAcceptance
        system
        (some
          { summary with
            finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
            finishWitnessOpeningRowValueSourceExtendMilliseconds :=
              sourceExtendMilliseconds
            finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
              sourceDownloadMilliseconds
            finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
              deviceDownloadMilliseconds
            finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
            finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
              deviceDownloadBatches
            finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
              deviceSingleDownloads
            finishWitnessOpeningRowValuesSourceRowCount := sourceRows
            finishWitnessOpeningRowValuesWordCount := words
            finishWitnessOpeningRowValuesByteCount := bytes })
        publicInput
        proof ->
        RuntimeVerifierCoreContract system publicInput proof := by
  intro publicInput proof observed
  exact
    proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract
      assumptions
      { summary with
        finishWitnessOpeningRowValuesMilliseconds := rowValuesMilliseconds
        finishWitnessOpeningRowValueSourceExtendMilliseconds :=
          sourceExtendMilliseconds
        finishWitnessOpeningRowValueSourceDownloadMilliseconds :=
          sourceDownloadMilliseconds
        finishWitnessOpeningRowValueDeviceDownloadMilliseconds :=
          deviceDownloadMilliseconds
        finishWitnessOpeningRowValuesDeviceRowCount := deviceRows
        finishWitnessOpeningRowValuesDeviceDownloadBatchCount :=
          deviceDownloadBatches
        finishWitnessOpeningRowValuesDeviceSingleDownloadCount :=
          deviceSingleDownloads
        finishWitnessOpeningRowValuesSourceRowCount := sourceRows
        finishWitnessOpeningRowValuesWordCount := words
        finishWitnessOpeningRowValuesByteCount := bytes }
      publicInput
      proof
      observed


end Lzvm
