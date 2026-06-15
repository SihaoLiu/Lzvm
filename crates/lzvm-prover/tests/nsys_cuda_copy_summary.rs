use std::path::Path;
use std::process::Command;

#[test]
fn nsys_cuda_copy_summary_deduplicates_cuda_memcpy_api_aliases() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-copy-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("temp directory should be creatable");
    let sqlite_path = temp_dir.join(format!(
        "nsys-copy-summary-alias-test-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sqlite_path);

    let setup = r#"
import sqlite3
import sys

path = sys.argv[1]
conn = sqlite3.connect(path)
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table ENUM_CUDA_MEMCPY_OPER (id integer primary key, label text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    eventClass integer,
    globalTid integer,
    correlationId integer,
    nameId integer,
    returnValue integer,
    callchainId integer
);
create table CUPTI_ACTIVITY_KIND_MEMCPY (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    bytes integer,
    copyKind integer,
    deprecatedSrcId integer,
    srcKind integer,
    dstKind integer,
    srcDeviceId integer,
    srcContextId integer,
    dstDeviceId integer,
    dstContextId integer,
    migrationCause integer,
    graphNodeId integer,
    virtualAddress integer,
    copyCount integer
);
create table OSRT_CALLCHAINS (
    id integer,
    symbol text,
    module text,
    kernelMode integer,
    thumbCode integer,
    unresolved integer,
    specialEntry integer,
    originalIP integer,
    unwindMethod integer,
    stackDepth integer
);
""")
conn.executemany("insert into StringIds (id, value) values (?, ?)", [
    (1, "cudaMemcpy_v3020"),
    (2, "cudaMemcpy"),
])
conn.executemany("insert into ENUM_CUDA_MEMCPY_OPER (id, label) values (?, ?)", [
    (1, "Host-to-Device"),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_RUNTIME
    (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
values (?, ?, 0, 7, ?, ?, 0, ?)
""", [
    (0, 10_000_000, 99, 1, None),
    (100, 9_999_900, 99, 2, 42),
])
conn.execute("""
insert into CUPTI_ACTIVITY_KIND_MEMCPY
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, bytes, copyKind, deprecatedSrcId, srcKind, dstKind, srcDeviceId,
     srcContextId, dstDeviceId, dstContextId, migrationCause, graphNodeId,
     virtualAddress, copyCount)
values (1000, 9000, 0, 0, null, 3, 99, 1, 2097152, 1, null, null, null,
        null, null, null, null, null, null, null, 1)
""")
conn.executemany("""
insert into OSRT_CALLCHAINS
    (id, symbol, module, kernelMode, thumbCode, unresolved, specialEntry,
     originalIP, unwindMethod, stackDepth)
values (?, ?, ?, 0, 0, 0, 0, 0, 0, ?)
""", [
    (42, "upload_trace_source_to_device", "lzvm", 0),
])
conn.commit()
conn.close()
"#;

    let setup_output = Command::new("python3")
        .arg("-c")
        .arg(setup)
        .arg(&sqlite_path)
        .output()
        .expect("alias test database should be created");
    assert!(
        setup_output.status.success(),
        "alias test database creation should succeed: stderr={}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&sqlite_path)
        .output()
        .expect("nsys CUDA copy summary should run on alias test database");
    let _ = std::fs::remove_file(&sqlite_path);

    assert!(
        output.status.success(),
        "nsys CUDA copy summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Host-to-Device,2097152,1,"),
        "correlated memcpy summary should count the H2D activity once: {stdout}"
    );
    assert!(
        stdout.contains("top_h2d_bulk_upload,2097152,calls=1 "),
        "bulk H2D summary should count the H2D activity once: {stdout}"
    );
    assert!(
        stdout.contains("cudaMemcpy,Host-to-Device,2097152,1,"),
        "callchain summary should keep the runtime alias that carries the callchain: {stdout}"
    );
    assert!(
        !stdout.contains("Host-to-Device,2097152,2,"),
        "runtime API aliases must not double count one CUDA memcpy activity: {stdout}"
    );
}

#[test]
fn nsys_cuda_copy_summary_groups_h2d_bulk_by_application_frame() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-copy-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("temp directory should be creatable");
    let sqlite_path = temp_dir.join(format!(
        "nsys-copy-summary-app-frame-test-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sqlite_path);

    let setup = r#"
import sqlite3
import sys

path = sys.argv[1]
conn = sqlite3.connect(path)
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table ENUM_CUDA_MEMCPY_OPER (id integer primary key, label text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    eventClass integer,
    globalTid integer,
    correlationId integer,
    nameId integer,
    returnValue integer,
    callchainId integer
);
create table CUPTI_ACTIVITY_KIND_MEMCPY (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    bytes integer,
    copyKind integer,
    deprecatedSrcId integer,
    srcKind integer,
    dstKind integer,
    srcDeviceId integer,
    srcContextId integer,
    dstDeviceId integer,
    dstContextId integer,
    migrationCause integer,
    graphNodeId integer,
    virtualAddress integer,
    copyCount integer
);
create table OSRT_CALLCHAINS (
    id integer,
    symbol text,
    module text,
    kernelMode integer,
    thumbCode integer,
    unresolved integer,
    specialEntry integer,
    originalIP integer,
    unwindMethod integer,
    stackDepth integer
);
""")
conn.executemany("insert into StringIds (id, value) values (?, ?)", [
    (1, "cudaMemcpy_v3020"),
])
conn.executemany("insert into ENUM_CUDA_MEMCPY_OPER (id, label) values (?, ?)", [
    (1, "Host-to-Device"),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_RUNTIME
    (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
values (?, ?, 0, 7, ?, 1, 0, ?)
""", [
    (0, 10_000_000, 10, 100),
    (20_000_000, 35_000_000, 11, 101),
    (40_000_000, 46_000_000, 12, 102),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_MEMCPY
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, bytes, copyKind, deprecatedSrcId, srcKind, dstKind, srcDeviceId,
     srcContextId, dstDeviceId, dstContextId, migrationCause, graphNodeId,
     virtualAddress, copyCount)
values (?, ?, 0, 0, null, 3, ?, 1, ?, 1, null, null, null,
        null, null, null, null, null, null, null, 1)
""", [
    (1000, 9000, 10, 2097152),
    (11000, 19000, 11, 2097152),
    (21000, 25000, 12, 1048576),
])
conn.executemany("""
insert into OSRT_CALLCHAINS
    (id, symbol, module, kernelMode, thumbCode, unresolved, specialEntry,
     originalIP, unwindMethod, stackDepth)
values (?, ?, ?, 0, 0, 0, 0, 0, 0, ?)
""", [
    (100, "upload_trace_source_to_device", "lzvm", 0),
    (100, "commit_stage_inputs", "lzvm", 1),
    (101, "upload_trace_source_to_device", "lzvm", 0),
    (101, "finish_witness_opening", "lzvm", 1),
    (102, "upload_auxiliary_inputs", "lzvm", 0),
])
conn.commit()
conn.close()
"#;

    let setup_output = Command::new("python3")
        .arg("-c")
        .arg(setup)
        .arg(&sqlite_path)
        .output()
        .expect("app-frame test database should be created");
    assert!(
        setup_output.status.success(),
        "app-frame test database creation should succeed: stderr={}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&sqlite_path)
        .output()
        .expect("nsys CUDA copy summary should run on app-frame test database");
    let _ = std::fs::remove_file(&sqlite_path);

    assert!(
        output.status.success(),
        "nsys CUDA copy summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("h2d_bulk_app_frame_hotspots"),
        "bulk H2D app-frame summary should be printed: {stdout}"
    );
    assert!(
        stdout.contains("2097152,2,25.000,0.016,15.000,upload_trace_source_to_device@lzvm"),
        "bulk H2D app-frame summary should merge same application frame across callchains: {stdout}"
    );
    assert!(
        stdout.contains("1048576,1,6.000,0.004,6.000,upload_auxiliary_inputs@lzvm"),
        "bulk H2D app-frame summary should keep distinct application frames separate: {stdout}"
    );
}

#[test]
fn nsys_cuda_copy_summary_flags_meminfo_polluted_h2d_bulk_callchains() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-copy-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("temp directory should be creatable");
    let sqlite_path = temp_dir.join(format!(
        "nsys-copy-summary-callchain-quality-test-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sqlite_path);

    let setup = r#"
import sqlite3
import sys

path = sys.argv[1]
conn = sqlite3.connect(path)
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table ENUM_CUDA_MEMCPY_OPER (id integer primary key, label text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    eventClass integer,
    globalTid integer,
    correlationId integer,
    nameId integer,
    returnValue integer,
    callchainId integer
);
create table CUPTI_ACTIVITY_KIND_MEMCPY (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    bytes integer,
    copyKind integer,
    deprecatedSrcId integer,
    srcKind integer,
    dstKind integer,
    srcDeviceId integer,
    srcContextId integer,
    dstDeviceId integer,
    dstContextId integer,
    migrationCause integer,
    graphNodeId integer,
    virtualAddress integer,
    copyCount integer
);
create table OSRT_CALLCHAINS (
    id integer,
    symbol text,
    module text,
    kernelMode integer,
    thumbCode integer,
    unresolved integer,
    specialEntry integer,
    originalIP integer,
    unwindMethod integer,
    stackDepth integer
);
""")
conn.executemany("insert into StringIds (id, value) values (?, ?)", [
    (1, "cudaMemcpy_v3020"),
])
conn.executemany("insert into ENUM_CUDA_MEMCPY_OPER (id, label) values (?, ?)", [
    (1, "Host-to-Device"),
])
conn.execute("""
insert into CUPTI_ACTIVITY_KIND_RUNTIME
    (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
values (0, 18_000_000, 0, 7, 30, 1, 0, 200)
""")
conn.execute("""
insert into CUPTI_ACTIVITY_KIND_MEMCPY
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, bytes, copyKind, deprecatedSrcId, srcKind, dstKind, srcDeviceId,
     srcContextId, dstDeviceId, dstContextId, migrationCause, graphNodeId,
     virtualAddress, copyCount)
values (1000, 17_000_000, 0, 0, null, 3, 30, 1, 2097152, 1, null, null, null,
        null, null, null, null, null, null, null, 1)
""")
conn.executemany("""
insert into OSRT_CALLCHAINS
    (id, symbol, module, kernelMode, thumbCode, unresolved, specialEntry,
     originalIP, unwindMethod, stackDepth)
values (?, ?, ?, 0, 0, 0, 0, 0, 0, ?)
""", [
    (200, "__GI___ioctl", "libc.so.6", 0),
    (200, "cudaMemGetInfo", "libcudart.so", 1),
    (200, "lzvm_cuda_memory_info", "lzvm", 2),
])
conn.commit()
conn.close()
"#;

    let setup_output = Command::new("python3")
        .arg("-c")
        .arg(setup)
        .arg(&sqlite_path)
        .output()
        .expect("callchain quality test database should be created");
    assert!(
        setup_output.status.success(),
        "callchain quality test database creation should succeed: stderr={}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&sqlite_path)
        .output()
        .expect("nsys CUDA copy summary should run on callchain quality test database");
    let _ = std::fs::remove_file(&sqlite_path);

    assert!(
        output.status.success(),
        "nsys CUDA copy summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cuda_memcpy_callchain_quality"),
        "callchain quality summary should be printed: {stdout}"
    );
    assert!(
        stdout.contains("h2d_bulk_meminfo_frame_calls,1,"),
        "meminfo-polluted H2D bulk callchains should be counted: {stdout}"
    );
    assert!(
        stdout.contains("example_callchain_id=200")
            && stdout.contains("app_frame=lzvm_cuda_memory_info@lzvm"),
        "quality summary should point at the suspect callchain and apparent app frame: {stdout}"
    );
}

#[test]
fn nsys_cuda_copy_summary_groups_small_d2h_batching_candidates() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-copy-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("temp directory should be creatable");
    let sqlite_path = temp_dir.join(format!(
        "nsys-copy-summary-small-d2h-test-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sqlite_path);

    let setup = r#"
import sqlite3
import sys

path = sys.argv[1]
conn = sqlite3.connect(path)
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table ENUM_CUDA_MEMCPY_OPER (id integer primary key, label text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    eventClass integer,
    globalTid integer,
    correlationId integer,
    nameId integer,
    returnValue integer,
    callchainId integer
);
create table CUPTI_ACTIVITY_KIND_MEMCPY (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    bytes integer,
    copyKind integer,
    deprecatedSrcId integer,
    srcKind integer,
    dstKind integer,
    srcDeviceId integer,
    srcContextId integer,
    dstDeviceId integer,
    dstContextId integer,
    migrationCause integer,
    graphNodeId integer,
    virtualAddress integer,
    copyCount integer
);
create table CUPTI_ACTIVITY_KIND_KERNEL (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    demangledName integer,
    shortName integer,
    mangledName integer
);
""")
conn.executemany("insert into StringIds (id, value) values (?, ?)", [
    (1, "cudaMemcpy_v3020"),
    (2, "poseidon2_merkle_digest_parent_kernel"),
])
conn.executemany("insert into ENUM_CUDA_MEMCPY_OPER (id, label) values (?, ?)", [
    (1, "Device-to-Host"),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_RUNTIME
    (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
values (?, ?, 0, 7, ?, 1, 0, null)
""", [
    (0, 2_000_000, 10),
    (3_000_000, 5_000_000, 11),
    (6_000_000, 8_000_000, 12),
    (9_000_000, 11_000_000, 13),
    (12_000_000, 14_000_000, 14),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_MEMCPY
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, bytes, copyKind, deprecatedSrcId, srcKind, dstKind, srcDeviceId,
     srcContextId, dstDeviceId, dstContextId, migrationCause, graphNodeId,
     virtualAddress, copyCount)
values (?, ?, 0, 0, null, 3, ?, 1, ?, 1, null, null, null,
        null, null, null, null, null, null, null, 1)
""", [
    (1000, 1500, 10, 1152),
    (2000, 2500, 11, 1152),
    (3000, 3500, 12, 1152),
    (4000, 4500, 13, 936),
    (5000, 5500, 14, 936),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_KERNEL
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, demangledName, shortName, mangledName)
values (?, ?, 0, 0, null, 3, ?, 1, 2, 2, 2)
""", [
    (100, 900, 100),
    (1600, 1900, 101),
    (2600, 2900, 102),
    (3600, 3900, 103),
    (4600, 4900, 104),
])
conn.commit()
conn.close()
"#;

    let setup_output = Command::new("python3")
        .arg("-c")
        .arg(setup)
        .arg(&sqlite_path)
        .output()
        .expect("small D2H test database should be created");
    assert!(
        setup_output.status.success(),
        "small D2H test database creation should succeed: stderr={}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&sqlite_path)
        .output()
        .expect("nsys CUDA copy summary should run on small D2H test database");
    let _ = std::fs::remove_file(&sqlite_path);

    assert!(
        output.status.success(),
        "nsys CUDA copy summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("small_d2h_batching_candidates"),
        "small D2H batching candidate summary should be printed: {stdout}"
    );
    assert!(
        stdout.contains("1152,3,6.000,0.002,2.000,poseidon2_merkle_digest_parent_kernel"),
        "same-size small D2H copies should be aggregated by size and preceding kernel: {stdout}"
    );
    assert!(
        stdout.contains("936,2,4.000,0.001,2.000,poseidon2_merkle_digest_parent_kernel"),
        "secondary small D2H sizes should stay visible: {stdout}"
    );
    assert!(
        stdout.contains("small_d2h_batching_hint,batch_small_d2h_by_size"),
        "transfer triage should flag repeated small D2H copies as a batching candidate: {stdout}"
    );
}

#[test]
fn nsys_cuda_copy_summary_reports_host_registration_overhead() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-copy-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("temp directory should be creatable");
    let sqlite_path = temp_dir.join(format!(
        "nsys-copy-summary-host-register-test-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sqlite_path);

    let setup = r#"
import sqlite3
import sys

path = sys.argv[1]
conn = sqlite3.connect(path)
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table ENUM_CUDA_MEMCPY_OPER (id integer primary key, label text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    eventClass integer,
    globalTid integer,
    correlationId integer,
    nameId integer,
    returnValue integer,
    callchainId integer
);
create table CUPTI_ACTIVITY_KIND_MEMCPY (
    start integer,
    end integer,
    deviceId integer,
    contextId integer,
    greenContextId integer,
    streamId integer,
    correlationId integer,
    globalPid integer,
    bytes integer,
    copyKind integer,
    deprecatedSrcId integer,
    srcKind integer,
    dstKind integer,
    srcDeviceId integer,
    srcContextId integer,
    dstDeviceId integer,
    dstContextId integer,
    migrationCause integer,
    graphNodeId integer,
    virtualAddress integer,
    copyCount integer
);
""")
conn.executemany("insert into StringIds (id, value) values (?, ?)", [
    (1, "cudaMemcpy_v3020"),
    (2, "cudaHostRegister_v3020"),
    (3, "cudaHostUnregister_v3020"),
])
conn.executemany("insert into ENUM_CUDA_MEMCPY_OPER (id, label) values (?, ?)", [
    (1, "Host-to-Device"),
])
conn.executemany("""
insert into CUPTI_ACTIVITY_KIND_RUNTIME
    (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
values (?, ?, 0, 7, ?, ?, 0, null)
""", [
    (0, 10_000_000, 10, 1),
    (20_000_000, 24_000_000, None, 2),
    (30_000_000, 37_000_000, None, 2),
    (40_000_000, 43_000_000, None, 3),
])
conn.execute("""
insert into CUPTI_ACTIVITY_KIND_MEMCPY
    (start, end, deviceId, contextId, greenContextId, streamId, correlationId,
     globalPid, bytes, copyKind, deprecatedSrcId, srcKind, dstKind, srcDeviceId,
     srcContextId, dstDeviceId, dstContextId, migrationCause, graphNodeId,
     virtualAddress, copyCount)
values (1000, 9000, 0, 0, null, 3, 10, 1, 2097152, 1, null, null, null,
        null, null, null, null, null, null, null, 1)
""")
conn.commit()
conn.close()
"#;

    let setup_output = Command::new("python3")
        .arg("-c")
        .arg(setup)
        .arg(&sqlite_path)
        .output()
        .expect("host-register test database should be created");
    assert!(
        setup_output.status.success(),
        "host-register test database creation should succeed: stderr={}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&sqlite_path)
        .output()
        .expect("nsys CUDA copy summary should run on host-register test database");
    let _ = std::fs::remove_file(&sqlite_path);

    assert!(
        output.status.success(),
        "nsys CUDA copy summary should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("runtime_cuda_host_registration_api"),
        "host registration API summary should be printed: {stdout}"
    );
    assert!(
        stdout.contains("cudaHostRegister_v3020,2,11.000,5.500,7.000"),
        "host register calls should report total, average, and max host API time: {stdout}"
    );
    assert!(
        stdout.contains("cudaHostUnregister_v3020,1,3.000,3.000,3.000"),
        "host unregister calls should report total, average, and max host API time: {stdout}"
    );
    assert!(
        stdout.contains("host_registration_hint,cache_or_reuse_pinned_host_memory"),
        "transfer triage should flag large host registration overhead: {stdout}"
    );
}

#[test]
fn nsys_cuda_copy_summary_reports_host_and_gpu_memcpy_waits() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/nsys-cuda-copy-summary.py");
    let script_source =
        std::fs::read_to_string(&script_path).expect("nsys CUDA copy summary source should read");

    for required in [
        "CUPTI_ACTIVITY_KIND_RUNTIME",
        "CUPTI_ACTIVITY_KIND_MEMCPY",
        "ENUM_CUDA_MEMCPY_OPER",
        "StringIds",
        "cudaMemcpy",
        "host_api_ms",
        "gpu_memcpy_ms",
        "wait_ratio",
        "OSRT_CALLCHAINS",
        "cuda_memcpy_callchain_hotspots",
        "d2h_cuda_memcpy_callchain_hotspots",
        "top_h2d_bulk_callchain_hotspots",
        "h2d_bulk_app_frame_hotspots",
        "cuda_memcpy_callchain_quality",
        "h2d_bulk_meminfo_frame_calls",
        "CUPTI_ACTIVITY_KIND_KERNEL",
        "d2h_wait_preceding_kernel_hotspots",
        "previous_kernel",
        "cuda_transfer_triage",
        "dominant_transfer_wait",
        "top_d2h_wait",
        "top_h2d_bulk_upload",
        "small_d2h_batching_candidates",
        "small_d2h_batching_hint",
        "batch_small_d2h_by_size",
        "gpu_residency_hint",
        "h2d_residency_hint",
        "--cudabacktrace=memory:80000",
        "app_frame",
    ] {
        assert!(
            script_source.contains(required),
            "nsys CUDA copy summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("nsys CUDA copy summary self-test should run");

    assert!(
        output.status.success(),
        "nsys CUDA copy summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Device-to-Host")
            && stdout.contains("host_api_ms")
            && stdout.contains("gpu_memcpy_ms")
            && stdout.contains("wait_ratio")
            && stdout.contains("cuda_memcpy_callchain_hotspots")
            && stdout.contains("api,direction,bytes")
            && stdout.contains("d2h_cuda_memcpy_callchain_hotspots")
            && stdout.contains("api,bytes,calls")
            && stdout.contains("top_h2d_bulk_callchain_hotspots")
            && stdout.contains("h2d_bulk_app_frame_hotspots")
            && stdout.contains("cuda_memcpy_callchain_quality")
            && stdout.contains("h2d_bulk_meminfo_frame_calls")
            && stdout.contains("cudaMemcpy_v3020,2097152")
            && stdout.contains("upload_trace_source_to_device")
            && stdout.contains("cudaMemcpy_v3020,Device-to-Host")
            && stdout.contains("copy_root_to_host")
            && stdout.contains("extract_opening_rows")
            && stdout.contains("app_frame")
            && stdout.contains("d2h_wait_preceding_kernel_hotspots")
            && stdout.contains("previous_kernel")
            && stdout.contains("poseidon2_merkle_digest_parent_kernel")
            && stdout.contains("pack_row_major_columns_strided_kernel")
            && stdout.contains("cuda_transfer_triage")
            && stdout.contains("dominant_transfer_wait")
            && stdout.contains("top_d2h_wait")
            && stdout.contains("top_h2d_bulk_upload")
            && stdout.contains("small_d2h_batching_candidates")
            && stdout.contains("small_d2h_batching_hint")
            && stdout.contains("gpu_residency_hint")
            && stdout.contains("h2d_residency_hint")
            && stdout.contains("batch_or_keep_small_d2h_on_device")
            && stdout.contains("reduce_bulk_h2d_source_uploads")
            && stdout.contains("cuda_api_backtrace_hint"),
        "nsys CUDA copy summary should print D2H host/GPU wait correlation"
    );
}
