use super::*;

#[test]
fn guest_pc_trace_backend_writes_zisk_main_load_reserved_rows() {
    let dir = temp_dir("load-reserved");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let code_words = [
        addi(1, 0, data_address as i16),
        lr_w(2, 1),
        addi(1, 1, 8),
        lr_d_aqrl(3, 1),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&[0x01, 0x00, 0x00, 0x80, 0xaa, 0xbb, 0xcc, 0xdd]);
    data.extend_from_slice(&0x0123_4567_89ab_cdef_u64.to_le_bytes());
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write load-reserved rows");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 4);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 1, 0, data_address);
    assert_wide(&trace, 1, 2, 0x8000_0001);
    assert_wide(&trace, 1, 4, 0xffff_ffff_8000_0001);
    assert_cell(&trace, 1, 12, 1);
    assert_cell(&trace, 1, 15, 0x29);
    assert_cell(&trace, 1, 21, 1);
    assert_cell(&trace, 1, 22, 4);
    assert_cell(&trace, 1, 24, 2);
    assert_eq!(trace.value(1, 26), Some(Felt::ZERO));

    assert_wide(&trace, 3, 0, data_address + 8);
    assert_wide(&trace, 3, 2, 0x0123_4567_89ab_cdef);
    assert_wide(&trace, 3, 4, 0x0123_4567_89ab_cdef);
    assert_cell(&trace, 3, 12, 1);
    assert_cell(&trace, 3, 15, 0x01);
    assert_cell(&trace, 3, 21, 1);
    assert_cell(&trace, 3, 22, 8);
    assert_cell(&trace, 3, 24, 3);
    assert_eq!(trace.value(3, 26), Some(Felt::ZERO));
}

#[test]
fn guest_pc_trace_backend_writes_zisk_main_add256_precompile_row() {
    let dir = temp_dir("add256-precompile");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let out_address = b_address + 32;
    let code_words = [
        addi(1, 0, params_address as i16),
        csrrs(2, 0x0811, 1),
        addi(3, 2, 2),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&a_address.to_le_bytes());
    data.extend_from_slice(&b_address.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&out_address.to_le_bytes());
    for _ in 0..4 {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    for _ in 0..4 {
        data.extend_from_slice(&0_u64.to_le_bytes());
    }
    data.extend_from_slice(&[0; 32]);
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(3);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write Add256 precompile row");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 3);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 1, 0, 0);
    assert_wide(&trace, 1, 2, params_address);
    assert_wide(&trace, 1, 4, 1);
    assert_cell(&trace, 1, 7, ENTRY + 4);
    assert_cell(&trace, 1, 8, 1);
    assert_cell(&trace, 1, 11, 1);
    assert_cell(&trace, 1, 12, 1);
    assert_cell(&trace, 1, 15, 0xf0);
    assert_cell(&trace, 1, 19, 1);
    assert_cell(&trace, 1, 20, 1);
    assert_cell(&trace, 1, 24, 2);

    assert_wide(&trace, 2, 0, 1);
    assert_wide(&trace, 2, 2, 2);
    assert_wide(&trace, 2, 4, 3);
    assert_cell(&trace, 2, 10, 1);
    assert_cell(&trace, 2, 12, 1);
    assert_cell(&trace, 2, 15, 0x0a);
    assert_cell(&trace, 2, 24, 3);
}

#[test]
fn guest_pc_trace_backend_writes_discarded_add256_carry_result() {
    let dir = temp_dir("discarded-add256-precompile");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_address = 64_u64;
    let params_address = data_address;
    let a_address = data_address + 32;
    let b_address = a_address + 32;
    let out_address = b_address + 32;
    let code_words = [
        addi(1, 0, params_address as i16),
        csrrs(0, 0x0811, 1),
        0x0000_0073,
    ];
    let mut code = Vec::with_capacity(code_words.len() * 4);
    for word in code_words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let data_offset = 176_u64 + code.len() as u64;
    let mut data = Vec::new();
    data.extend_from_slice(&a_address.to_le_bytes());
    data.extend_from_slice(&b_address.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.extend_from_slice(&out_address.to_le_bytes());
    for _ in 0..4 {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    for _ in 0..4 {
        data.extend_from_slice(&0_u64.to_le_bytes());
    }
    data.extend_from_slice(&[0; 32]);
    let headers = [
        program_header_at(176, ENTRY, code.len() as u64),
        program_header_at(data_offset, data_address, data.len() as u64),
    ];
    let mut guest_image_bytes = sample_guest_image_with_program_headers(&headers);
    guest_image_bytes.resize(176, 0);
    guest_image_bytes.extend_from_slice(&code);
    guest_image_bytes.resize(data_offset as usize, 0);
    guest_image_bytes.extend_from_slice(&data);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");

    let trace = run_witness_trace_with_context(
        &GuestPcTraceBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        layout.request(Vec::new()),
    )
    .expect("Zisk Main layout should write discarded Add256 carry");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 2);
    assert_eq!(trace.column_count(), 27);

    assert_wide(&trace, 1, 0, 0);
    assert_wide(&trace, 1, 2, params_address);
    assert_wide(&trace, 1, 4, 1);
    assert_cell(&trace, 1, 12, 0);
    assert_cell(&trace, 1, 15, 0xf0);
    assert_cell(&trace, 1, 20, 1);
}
