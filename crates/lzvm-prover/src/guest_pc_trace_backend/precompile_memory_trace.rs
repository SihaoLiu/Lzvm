use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrecompileMemoryTraceColumns {
    main_step: TraceColumnTarget,
    is_write: TraceColumnTarget,
    address: TraceColumnTarget,
    value: TraceColumnTarget,
    byte_len: Option<TraceColumnTarget>,
    selector: TraceColumnTarget,
}

pub(super) fn write_layout_precompile_memory_trace(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    output: &mut [u8],
) -> Result<Option<usize>, GuestPcTraceBackendError> {
    let Some(columns) = precompile_memory_trace_columns(layout)? else {
        return Ok(None);
    };
    let access_count = reports.iter().try_fold(0_usize, |count, report| {
        count.checked_add(report.precompile_memory_accesses.len())
    });
    let Some(access_count) = access_count else {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: usize::MAX,
            output_len: output.len(),
        });
    };
    if access_count > layout.row_count() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len: layout_trace_byte_len(access_count, layout.column_count()),
            output_len: output.len(),
        });
    }

    let mut builder = layout
        .trace_builder()
        .map_err(GuestPcTraceBackendError::TraceBuild)?;
    let mut state = ZiskMainTraceState::new();
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: true,
        previous_c: 0,
    };
    let mut output_row = 0_usize;
    for (main_step, report) in reports.iter().enumerate() {
        let next_instruction = reports.get(main_step + 1).map(|next| next.instruction);
        validate_and_apply_zisk_main_report(
            main_step,
            report,
            next_instruction,
            &mut state,
            None,
            reports.len(),
            segment,
        )?;
        for access in &report.precompile_memory_accesses {
            write_precompile_memory_access(
                &mut builder,
                output_row,
                &columns,
                main_step as u64,
                access,
            )?;
            output_row += 1;
        }
    }

    let trace = builder.build();
    let produced_len =
        trace
            .values()
            .len()
            .checked_mul(8)
            .ok_or(GuestPcTraceBackendError::OutputOverflow {
                produced_len: usize::MAX,
                output_len: output.len(),
            })?;
    if produced_len > output.len() {
        return Err(GuestPcTraceBackendError::OutputOverflow {
            produced_len,
            output_len: output.len(),
        });
    }
    for (index, value) in trace.values().iter().copied().enumerate() {
        let offset = index * 8;
        output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(Some(produced_len))
}

fn write_precompile_memory_access(
    builder: &mut crate::witness_layout::WitnessTraceBuilder<'_>,
    row: usize,
    columns: &PrecompileMemoryTraceColumns,
    main_step: u64,
    access: &GuestMemoryAccess,
) -> Result<(), GuestPcTraceBackendError> {
    write_column(builder, row, &columns.main_step, main_step)?;
    write_column(
        builder,
        row,
        &columns.is_write,
        u64::from(matches!(access.kind, GuestMemoryAccessKind::Write)),
    )?;
    write_column(builder, row, &columns.address, access.address)?;
    write_wide_column(builder, row, &columns.value, access.value)?;
    write_optional_column(builder, row, &columns.byte_len, access.byte_len as u64)?;
    write_column(builder, row, &columns.selector, 1)
}

pub(super) fn precompile_memory_trace_columns(
    layout: &WitnessTraceLayout,
) -> Result<Option<PrecompileMemoryTraceColumns>, GuestPcTraceBackendError> {
    let main_step = trace_column_target(layout, "precompile_mem_main_step")?;
    let is_write = trace_column_target(layout, "precompile_mem_is_write")?;
    let address = trace_column_target(layout, "precompile_mem_address")?;
    let value = vector_trace_column_target(layout, "precompile_mem_value", 2)?;
    let byte_len = trace_column_target(layout, "precompile_mem_byte_len")?;
    let selector = trace_column_target(layout, "precompile_mem_selector")?;
    if main_step.is_none()
        && is_write.is_none()
        && address.is_none()
        && value.is_none()
        && byte_len.is_none()
        && selector.is_none()
    {
        return Ok(None);
    }
    let main_step = main_step.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "missing precompile_mem_main_step column".to_owned(),
    })?;
    let is_write = is_write.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "missing precompile_mem_is_write column".to_owned(),
    })?;
    let address = address.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "missing precompile_mem_address column".to_owned(),
    })?;
    let value = value.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "missing precompile_mem_value column".to_owned(),
    })?;
    let selector = selector.ok_or_else(|| GuestPcTraceBackendError::InvalidPcTraceLayout {
        message: "missing precompile_mem_selector column".to_owned(),
    })?;
    Ok(Some(PrecompileMemoryTraceColumns {
        main_step,
        is_write,
        address,
        value,
        byte_len,
        selector,
    }))
}
