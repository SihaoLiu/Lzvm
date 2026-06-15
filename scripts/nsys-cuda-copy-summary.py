#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

RUNTIME_TABLE = "CUPTI_ACTIVITY_KIND_RUNTIME"
MEMCPY_TABLE = "CUPTI_ACTIVITY_KIND_MEMCPY"
KERNEL_TABLE = "CUPTI_ACTIVITY_KIND_KERNEL"
MEMCPY_KIND_TABLE = "ENUM_CUDA_MEMCPY_OPER"
STRING_TABLE = "StringIds"
CALLCHAIN_TABLE = "OSRT_CALLCHAINS"


def ms(ns: int | float | None) -> float:
    return float(ns or 0) / 1_000_000.0


def us(ns: int | float | None) -> float:
    return float(ns or 0) / 1_000.0


def ratio(host_ns: int | float | None, gpu_ns: int | float | None) -> str:
    host = float(host_ns or 0)
    gpu = float(gpu_ns or 0)
    if gpu == 0:
        return "inf" if host > 0 else "0.000"
    return f"{host / gpu:.3f}"


def csv_cell(value: object) -> str:
    return str(value).replace(",", " ").replace("\n", " ").replace("\r", " ")


def table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "select 1 from sqlite_master where type = 'table' and name = ?",
        (name,),
    ).fetchone()
    return row is not None


def require_tables(conn: sqlite3.Connection) -> None:
    missing = [
        table
        for table in [RUNTIME_TABLE, MEMCPY_TABLE, MEMCPY_KIND_TABLE, STRING_TABLE]
        if not table_exists(conn, table)
    ]
    if missing:
        raise SystemExit(f"missing nsys SQLite tables: {', '.join(missing)}")


def runtime_memcpy_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            count(*) as calls,
            sum(r.end - r.start) as host_ns
        from {RUNTIME_TABLE} r
        left join {STRING_TABLE} s on s.id = r.nameId
        where s.value like 'cudaMemcpy%'
        group by api
        order by host_ns desc, calls desc, api asc
        """
    ).fetchall()


def gpu_memcpy_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(e.label, 'copyKind=' || m.copyKind) as direction,
            count(*) as calls,
            sum(m.bytes) as bytes,
            sum(m.end - m.start) as gpu_ns
        from {MEMCPY_TABLE} m
        left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
        group by direction
        order by gpu_ns desc, bytes desc, calls desc
        """
    ).fetchall()


def correlated_memcpy_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(e.label, 'copyKind=' || m.copyKind) as direction,
            m.bytes as bytes,
            count(*) as calls,
            sum(r.end - r.start) as host_ns,
            sum(m.end - m.start) as gpu_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} s on s.id = r.nameId
        join {MEMCPY_TABLE} m on m.correlationId = r.correlationId
        left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
        where s.value like 'cudaMemcpy%'
        group by direction, m.bytes
        order by host_ns desc, calls desc, direction asc, m.bytes asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def memcpy_callchain_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    if not table_exists(conn, CALLCHAIN_TABLE):
        return []
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            coalesce(e.label, 'copyKind=' || m.copyKind) as direction,
            m.bytes as bytes,
            r.callchainId as callchain_id,
            count(*) as calls,
            sum(r.end - r.start) as host_ns,
            sum(m.end - m.start) as gpu_ns,
            max(r.end - r.start) as max_host_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} s on s.id = r.nameId
        join {MEMCPY_TABLE} m on m.correlationId = r.correlationId
        left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
        where s.value like 'cudaMemcpy%'
          and r.callchainId is not null
        group by api, direction, m.bytes, r.callchainId
        order by host_ns desc, calls desc, api asc, direction asc, m.bytes asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def memcpy_missing_callchain_summary(conn: sqlite3.Connection) -> sqlite3.Row:
    return conn.execute(
        f"""
        select
            count(*) as calls,
            sum(r.end - r.start) as host_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} s on s.id = r.nameId
        join {MEMCPY_TABLE} m on m.correlationId = r.correlationId
        where s.value like 'cudaMemcpy%'
          and r.callchainId is null
        """
    ).fetchone()


def d2h_preceding_kernel_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    if not table_exists(conn, KERNEL_TABLE):
        return []
    conn.executescript(
        f"""
        drop table if exists temp._lzvm_d2h_memcpy;
        drop table if exists temp._lzvm_kernel_stream_end;
        create temp table _lzvm_d2h_memcpy as
            select
                r.start as runtime_start,
                r.end as runtime_end,
                m.start as memcpy_start,
                m.end as memcpy_end,
                m.bytes as bytes,
                m.streamId as stream_id
            from {RUNTIME_TABLE} r
            join {STRING_TABLE} api on api.id = r.nameId
            join {MEMCPY_TABLE} m on m.correlationId = r.correlationId
            left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
            where api.value like 'cudaMemcpy%'
              and coalesce(e.label, 'copyKind=' || m.copyKind) = 'Device-to-Host';
        create temp table _lzvm_kernel_stream_end as
            select
                streamId as stream_id,
                end as kernel_end,
                shortName as kernel_name_id
            from {KERNEL_TABLE};
        create index _lzvm_kernel_stream_end_idx
            on _lzvm_kernel_stream_end(stream_id, kernel_end);
        """
    )
    return conn.execute(
        f"""
        with d2h_with_kernel as (
            select
                d2h.rowid as d2h_rowid,
                d2h.*,
                (
                    select k.kernel_name_id
                    from _lzvm_kernel_stream_end k
                    where k.stream_id = d2h.stream_id
                      and k.kernel_end <= d2h.memcpy_start
                    order by k.kernel_end desc
                    limit 1
                ) as previous_kernel_name_id,
                (
                    select k.kernel_end
                    from _lzvm_kernel_stream_end k
                    where k.stream_id = d2h.stream_id
                      and k.kernel_end <= d2h.memcpy_start
                    order by k.kernel_end desc
                    limit 1
                ) as previous_kernel_end
            from _lzvm_d2h_memcpy d2h
        )
        select
            bytes,
            coalesce(kernel.value, 'unresolved') as previous_kernel,
            count(*) as calls,
            sum(runtime_end - runtime_start) as host_ns,
            sum(memcpy_end - memcpy_start) as gpu_ns,
            max(runtime_end - runtime_start) as max_host_ns,
            avg(memcpy_start - previous_kernel_end) as avg_gap_ns
        from d2h_with_kernel
        left join {STRING_TABLE} kernel on kernel.id = previous_kernel_name_id
        group by bytes, previous_kernel
        order by host_ns desc, calls desc, bytes asc, previous_kernel asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def callchain_frame_rows(
    conn: sqlite3.Connection,
    callchain_id: int,
) -> list[sqlite3.Row]:
    if not table_exists(conn, CALLCHAIN_TABLE):
        return []
    return conn.execute(
        f"""
        select
            c.stackDepth as stackDepth,
            coalesce(sym.value, c.symbol, 'unknown') as symbol,
            coalesce(mod.value, c.module, 'unknown') as module
        from {CALLCHAIN_TABLE} c
        left join {STRING_TABLE} sym on sym.id = c.symbol
        left join {STRING_TABLE} mod on mod.id = c.module
        where c.id = ?
        order by c.stackDepth asc
        """,
        (callchain_id,),
    ).fetchall()


def callchain_frames(
    conn: sqlite3.Connection,
    callchain_id: int,
    max_frames: int = 6,
) -> str:
    rows = callchain_frame_rows(conn, callchain_id)[:max_frames]
    frames = []
    for row in rows:
        symbol = row["symbol"] or "unknown"
        module = row["module"] or "unknown"
        frames.append(f"{symbol}@{module}")
    return " | ".join(frames) if frames else "unresolved"


def first_application_frame(conn: sqlite3.Connection, callchain_id: int) -> str:
    for row in callchain_frame_rows(conn, callchain_id):
        symbol = str(row["symbol"] or "unknown")
        module = str(row["module"] or "unknown")
        lowered_module = module.lower()
        if symbol.startswith("0x") or symbol == "__GI___ioctl":
            continue
        if any(
            skipped in lowered_module
            for skipped in [
                "libcuda",
                "libcudart",
                "libc.so",
                "libpthread",
                "ld-linux",
            ]
        ):
            continue
        return f"{symbol}@{module}"
    return "unresolved"


def print_runtime(rows: list[sqlite3.Row]) -> None:
    print("runtime_cuda_memcpy_api")
    print("api,calls,host_api_ms,avg_host_api_us")
    for row in rows:
        calls = int(row["calls"] or 0)
        host_ns = int(row["host_ns"] or 0)
        avg = host_ns / calls if calls else 0
        print(f"{row['api']},{calls},{ms(host_ns):.3f},{us(avg):.3f}")
    if not rows:
        print("none,0,0.000,0.000")


def print_gpu(rows: list[sqlite3.Row]) -> None:
    print()
    print("gpu_cuda_memcpy_activity")
    print("direction,calls,bytes,gpu_memcpy_ms,avg_gpu_us")
    for row in rows:
        calls = int(row["calls"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        avg = gpu_ns / calls if calls else 0
        print(
            f"{row['direction']},{calls},{int(row['bytes'] or 0)},"
            f"{ms(gpu_ns):.3f},{us(avg):.3f}"
        )
    if not rows:
        print("none,0,0,0.000,0.000")


def print_correlated(rows: list[sqlite3.Row]) -> None:
    print()
    print("correlated_cuda_memcpy_waits_by_direction_and_size")
    print("direction,bytes,calls,host_api_ms,gpu_memcpy_ms,wait_ratio")
    for row in rows:
        host_ns = int(row["host_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        print(
            f"{row['direction']},{int(row['bytes'] or 0)},{int(row['calls'] or 0)},"
            f"{ms(host_ns):.3f},{ms(gpu_ns):.3f},{ratio(host_ns, gpu_ns)}"
        )
    if not rows:
        print("none,0,0,0.000,0.000,0.000")


def print_callchains(conn: sqlite3.Connection, rows: list[sqlite3.Row]) -> None:
    print()
    print("cuda_memcpy_callchain_hotspots")
    print(
        "api,direction,bytes,calls,host_api_ms,gpu_memcpy_ms,max_host_api_ms,"
        "callchain_id,app_frame,frames"
    )
    for row in rows:
        callchain_id = int(row["callchain_id"])
        host_ns = int(row["host_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        print(
            f"{row['api']},{row['direction']},{int(row['bytes'] or 0)},"
            f"{int(row['calls'] or 0)},{ms(host_ns):.3f},{ms(gpu_ns):.3f},"
            f"{ms(row['max_host_ns']):.3f},{callchain_id},"
            f"{csv_cell(first_application_frame(conn, callchain_id))},"
            f"{csv_cell(callchain_frames(conn, callchain_id))}"
        )
    if not rows:
        print("none,none,0,0,0.000,0.000,0.000,0,unavailable,unavailable")


def print_d2h_preceding_kernels(rows: list[sqlite3.Row]) -> None:
    print()
    print("d2h_wait_preceding_kernel_hotspots")
    print(
        "bytes,previous_kernel,calls,host_api_ms,gpu_memcpy_ms,"
        "max_host_api_ms,avg_kernel_to_copy_gap_us"
    )
    for row in rows:
        host_ns = int(row["host_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        avg_gap_ns = row["avg_gap_ns"]
        print(
            f"{int(row['bytes'] or 0)},{csv_cell(row['previous_kernel'])},"
            f"{int(row['calls'] or 0)},{ms(host_ns):.3f},{ms(gpu_ns):.3f},"
            f"{ms(row['max_host_ns']):.3f},{us(avg_gap_ns):.3f}"
        )
    if not rows:
        print("0,unavailable,0,0.000,0.000,0.000,0.000")


def sum_rows_ns(rows: list[sqlite3.Row], column: str) -> int:
    return sum(int(row[column] or 0) for row in rows)


def direction_host_waits(rows: list[sqlite3.Row]) -> dict[str, int]:
    waits: dict[str, int] = {}
    for row in rows:
        direction = str(row["direction"] or "unknown")
        waits[direction] = waits.get(direction, 0) + int(row["host_ns"] or 0)
    return waits


def direction_gpu_waits(rows: list[sqlite3.Row]) -> dict[str, int]:
    waits: dict[str, int] = {}
    for row in rows:
        direction = str(row["direction"] or "unknown")
        waits[direction] = waits.get(direction, 0) + int(row["gpu_ns"] or 0)
    return waits


def top_direction(waits: dict[str, int]) -> tuple[str, int]:
    if not waits:
        return ("none", 0)
    return max(waits.items(), key=lambda item: (item[1], item[0]))


def print_transfer_triage(
    runtime_rows: list[sqlite3.Row],
    gpu_rows: list[sqlite3.Row],
    correlated_rows: list[sqlite3.Row],
    d2h_rows: list[sqlite3.Row],
) -> None:
    host_ns = sum_rows_ns(runtime_rows, "host_ns")
    gpu_ns = sum_rows_ns(gpu_rows, "gpu_ns")
    host_direction, host_direction_ns = top_direction(direction_host_waits(correlated_rows))
    gpu_direction, gpu_direction_ns = top_direction(direction_gpu_waits(gpu_rows))
    top_d2h = d2h_rows[0] if d2h_rows else None
    print()
    print("cuda_transfer_triage")
    print("metric,value,detail")
    print(
        f"host_memcpy_api_ms,{ms(host_ns):.3f},"
        "host time spent inside cudaMemcpy APIs"
    )
    print(f"gpu_memcpy_ms,{ms(gpu_ns):.3f},GPU copy engine activity")
    print(
        f"dominant_transfer_wait,{csv_cell(host_direction)},"
        f"host_api_ms={ms(host_direction_ns):.3f}"
    )
    print(
        f"dominant_gpu_copy,{csv_cell(gpu_direction)},"
        f"gpu_memcpy_ms={ms(gpu_direction_ns):.3f}"
    )
    if top_d2h is None:
        print("top_d2h_wait,none,no D2H memcpy activity")
        print("gpu_residency_hint,none,no D2H hotspot")
        return
    top_host_ns = int(top_d2h["host_ns"] or 0)
    top_gpu_ns = int(top_d2h["gpu_ns"] or 0)
    top_bytes = int(top_d2h["bytes"] or 0)
    print(
        f"top_d2h_wait,{top_bytes},"
        f"previous_kernel={csv_cell(top_d2h['previous_kernel'])} "
        f"calls={int(top_d2h['calls'] or 0)} "
        f"host_api_ms={ms(top_host_ns):.3f} "
        f"gpu_memcpy_ms={ms(top_gpu_ns):.3f} "
        f"wait_ratio={ratio(top_host_ns, top_gpu_ns)}"
    )
    if top_bytes <= 4096 and top_host_ns > top_gpu_ns * 100:
        hint = "batch_or_keep_small_d2h_on_device"
    elif host_direction == "Host-to-Device" and host_direction_ns > gpu_direction_ns:
        hint = "prefer_reused_device_residency_for_h2d_inputs"
    else:
        hint = "inspect_callchains_before_copy_refactor"
    print(
        f"gpu_residency_hint,{hint},"
        "prioritize changes that remove host round trips without changing verifier outputs"
    )


def print_callchain_hint(conn: sqlite3.Connection) -> None:
    missing = memcpy_missing_callchain_summary(conn)
    missing_calls = int(missing["calls"] or 0)
    missing_host_ns = int(missing["host_ns"] or 0)
    print()
    print("cuda_api_backtrace_hint")
    print("missing_callchain_calls,missing_host_api_ms,recommended_nsys_options")
    if missing_calls:
        print(
            f"{missing_calls},{ms(missing_host_ns):.3f},"
            "--trace=cuda,nvtx,osrt --sample=process-tree "
            "--cudabacktrace=memory:80000"
        )
    else:
        print("0,0.000,none")


def summarize(conn: sqlite3.Connection, label: str, limit: int) -> None:
    require_tables(conn)
    conn.row_factory = sqlite3.Row
    runtime_rows = runtime_memcpy_summary(conn)
    gpu_rows = gpu_memcpy_summary(conn)
    correlated_rows = correlated_memcpy_summary(conn, limit)
    callchain_rows = memcpy_callchain_summary(conn, limit)
    d2h_rows = d2h_preceding_kernel_summary(conn, limit)
    print(f"profile={label}")
    print_runtime(runtime_rows)
    print_gpu(gpu_rows)
    print_correlated(correlated_rows)
    print_callchains(conn, callchain_rows)
    print_d2h_preceding_kernels(d2h_rows)
    print_transfer_triage(runtime_rows, gpu_rows, correlated_rows, d2h_rows)
    print_callchain_hint(conn)


def build_self_test_db() -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    conn.executescript(
        f"""
        create table {STRING_TABLE} (id integer primary key, value text);
        create table {MEMCPY_KIND_TABLE} (id integer primary key, label text);
        create table {RUNTIME_TABLE} (
            start integer,
            end integer,
            nameId integer,
            correlationId integer
        );
        create table {MEMCPY_TABLE} (
            start integer,
            end integer,
            bytes integer,
            copyKind integer,
            correlationId integer,
            streamId integer
        );
        create table {KERNEL_TABLE} (
            start integer,
            end integer,
            streamId integer,
            shortName integer
        );
        create table {CALLCHAIN_TABLE} (
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
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [
            (1, "cudaMemcpy_v3020"),
            (2, "cudaMemcpyAsync_v3020"),
            (3, "poseidon2_merkle_digest_parent_kernel"),
            (4, "pack_row_major_columns_strided_kernel"),
        ],
    )
    conn.executemany(
        f"insert into {MEMCPY_KIND_TABLE} (id, label) values (?, ?)",
        [(1, "Host-to-Device"), (2, "Device-to-Host")],
    )
    conn.executemany(
        f"insert into {RUNTIME_TABLE} (start, end, nameId, correlationId) values (?, ?, ?, ?)",
        [
            (0, 2_000_000, 1, 10),
            (3_000_000, 6_500_000, 1, 11),
            (7_000_000, 8_000_000, 2, 12),
        ],
    )
    conn.execute(f"alter table {RUNTIME_TABLE} add column callchainId integer")
    conn.execute(
        f"update {RUNTIME_TABLE} set callchainId = 100 where correlationId = 10"
    )
    conn.execute(
        f"update {RUNTIME_TABLE} set callchainId = 101 where correlationId = 11"
    )
    conn.executemany(
        f"""
        insert into {MEMCPY_TABLE} (start, end, bytes, copyKind, correlationId, streamId)
        values (?, ?, ?, ?, ?, ?)
        """,
        [
            (10_000, 10_500, 32, 2, 10, 7),
            (20_000, 20_700, 1152, 2, 11, 7),
            (30_000, 35_000, 4096, 1, 12, 7),
        ],
    )
    conn.executemany(
        f"""
        insert into {KERNEL_TABLE} (start, end, streamId, shortName)
        values (?, ?, ?, ?)
        """,
        [
            (1_000, 8_000, 7, 3),
            (12_000, 18_000, 7, 4),
        ],
    )
    conn.executemany(
        f"""
        insert into {CALLCHAIN_TABLE}
            (id, symbol, module, kernelMode, thumbCode, unresolved, specialEntry,
             originalIP, unwindMethod, stackDepth)
        values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        [
            (100, "copy_root_to_host", "lzvm", 0, 0, 0, 0, 0, 0, 0),
            (100, "commit_stage_root", "lzvm", 0, 0, 0, 0, 0, 0, 1),
            (101, "extract_opening_rows", "lzvm", 0, 0, 0, 0, 0, 0, 0),
            (101, "finish_witness_opening", "lzvm", 0, 0, 0, 0, 0, 0, 1),
        ],
    )
    return conn


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize CUDA memcpy wait shape from an Nsight Systems SQLite export."
    )
    parser.add_argument("sqlite", nargs="?", help="path to an nsys SQLite export")
    parser.add_argument("--top", type=int, default=12, help="correlated rows to print")
    parser.add_argument("--self-test", action="store_true", help="run against an in-memory sample")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        with closing(build_self_test_db()) as conn:
            summarize(conn, "self-test", max(args.top, 1))
        return 0

    if not args.sqlite:
        raise SystemExit("sqlite path is required unless --self-test is used")
    path = Path(args.sqlite)
    if not path.exists():
        raise SystemExit(f"SQLite export does not exist: {path}")

    with closing(sqlite3.connect(path)) as conn:
        summarize(conn, str(path), max(args.top, 1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
