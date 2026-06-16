#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

RUNTIME_TABLE = "CUPTI_ACTIVITY_KIND_RUNTIME"
KERNEL_TABLE = "CUPTI_ACTIVITY_KIND_KERNEL"
MEMCPY_TABLE = "CUPTI_ACTIVITY_KIND_MEMCPY"
MEMCPY_KIND_TABLE = "ENUM_CUDA_MEMCPY_OPER"
STRING_TABLE = "StringIds"


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
        for table in [RUNTIME_TABLE, KERNEL_TABLE, STRING_TABLE]
        if not table_exists(conn, table)
    ]
    if missing:
        raise SystemExit(f"missing nsys SQLite tables: {', '.join(missing)}")


def kernel_name_expr() -> str:
    return "coalesce(short.value, demangled.value, 'nameId=' || k.shortName)"


def kernel_gpu_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            {kernel_name_expr()} as kernel,
            count(*) as calls,
            sum(k.end - k.start) as gpu_ns,
            max(k.end - k.start) as max_gpu_ns,
            min(k.gridX) as min_grid_x,
            max(k.gridX) as max_grid_x,
            min(k.blockX) as min_block_x,
            max(k.blockX) as max_block_x
        from {KERNEL_TABLE} k
        left join {STRING_TABLE} short on short.id = k.shortName
        left join {STRING_TABLE} demangled on demangled.id = k.demangledName
        group by kernel
        order by gpu_ns desc, calls desc, kernel asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def launch_api_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            count(*) as calls,
            sum(r.end - r.start) as launch_ns
        from {RUNTIME_TABLE} r
        left join {STRING_TABLE} s on s.id = r.nameId
        where s.value like 'cudaLaunchKernel%'
           or s.value like 'cudaGraphLaunch%'
        group by api
        order by launch_ns desc, calls desc, api asc
        """
    ).fetchall()


def sync_api_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            coalesce(r.returnValue, -1) as return_code,
            count(*) as calls,
            sum(r.end - r.start) as sync_ns,
            max(r.end - r.start) as max_sync_ns
        from {RUNTIME_TABLE} r
        left join {STRING_TABLE} s on s.id = r.nameId
        where s.value like 'cudaDeviceSynchronize%'
           or s.value like 'cudaStreamSynchronize%'
           or s.value like 'cudaEventSynchronize%'
        group by api, return_code
        order by sync_ns desc, calls desc, api asc, return_code asc
        """
    ).fetchall()


def correlated_launch_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            {kernel_name_expr()} as kernel,
            count(*) as calls,
            sum(r.end - r.start) as launch_ns,
            sum(k.end - k.start) as gpu_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} api on api.id = r.nameId
        join {KERNEL_TABLE} k on k.correlationId = r.correlationId
        left join {STRING_TABLE} short on short.id = k.shortName
        left join {STRING_TABLE} demangled on demangled.id = k.demangledName
        where api.value like 'cudaLaunchKernel%'
           or api.value like 'cudaGraphLaunch%'
        group by kernel
        order by launch_ns desc, calls desc, kernel asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def stream_kernel_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            k.streamId as stream_id,
            count(*) as calls,
            sum(k.end - k.start) as gpu_ns,
            min(k.start) as first_ns,
            max(k.end) as last_ns
        from {KERNEL_TABLE} k
        group by k.streamId
        order by gpu_ns desc, calls desc, k.streamId asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def fusion_candidate_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            {kernel_name_expr()} as kernel,
            count(*) as calls,
            sum(r.end - r.start) as launch_ns,
            sum(k.end - k.start) as gpu_ns,
            max(k.end - k.start) as max_gpu_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} api on api.id = r.nameId
        join {KERNEL_TABLE} k on k.correlationId = r.correlationId
        left join {STRING_TABLE} short on short.id = k.shortName
        left join {STRING_TABLE} demangled on demangled.id = k.demangledName
        where api.value like 'cudaLaunchKernel%'
           or api.value like 'cudaGraphLaunch%'
        group by kernel
        having calls >= 2
        order by launch_ns desc, calls desc, gpu_ns desc, kernel asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def graph_shape_candidate_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            {kernel_name_expr()} as kernel,
            k.gridX as grid_x,
            k.blockX as block_x,
            count(distinct k.streamId) as streams,
            count(*) as calls,
            sum(r.end - r.start) as launch_ns,
            sum(k.end - k.start) as gpu_ns
        from {RUNTIME_TABLE} r
        join {STRING_TABLE} api on api.id = r.nameId
        join {KERNEL_TABLE} k on k.correlationId = r.correlationId
        left join {STRING_TABLE} short on short.id = k.shortName
        left join {STRING_TABLE} demangled on demangled.id = k.demangledName
        where api.value like 'cudaLaunchKernel%'
           or api.value like 'cudaGraphLaunch%'
        group by kernel, k.gridX, k.blockX
        having calls >= 2
        order by launch_ns desc, calls desc, gpu_ns desc, kernel asc, grid_x desc, block_x desc
        limit ?
        """,
        (limit,),
    ).fetchall()


def memcpy_tables_available(conn: sqlite3.Connection) -> bool:
    return table_exists(conn, MEMCPY_TABLE) and table_exists(conn, MEMCPY_KIND_TABLE)


def memcpy_direction_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    if not memcpy_tables_available(conn):
        return []
    return conn.execute(
        f"""
        select
            coalesce(e.label, 'copyKind=' || m.copyKind) as direction,
            count(*) as calls,
            sum(m.bytes) as bytes,
            sum(m.end - m.start) as gpu_ns,
            sum(coalesce(r.end - r.start, 0)) as host_ns
        from {MEMCPY_TABLE} m
        left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
        left join {RUNTIME_TABLE} r on r.correlationId = m.correlationId
        group by direction
        order by host_ns desc, gpu_ns desc, bytes desc, calls desc, direction asc
        """
    ).fetchall()


def d2h_preceding_kernel_summary(conn: sqlite3.Connection) -> sqlite3.Row | None:
    if not memcpy_tables_available(conn):
        return None
    conn.executescript(
        f"""
        drop table if exists temp._lzvm_kernel_summary_d2h_memcpy;
        drop table if exists temp._lzvm_kernel_summary_stream_end;
        create temp table _lzvm_kernel_summary_d2h_memcpy as
            select
                coalesce(r.start, m.start) as runtime_start,
                coalesce(r.end, m.end) as runtime_end,
                m.start as memcpy_start,
                m.end as memcpy_end,
                m.bytes as bytes,
                m.streamId as stream_id
            from {MEMCPY_TABLE} m
            left join {RUNTIME_TABLE} r on r.correlationId = m.correlationId
            left join {MEMCPY_KIND_TABLE} e on e.id = m.copyKind
            where coalesce(e.label, 'copyKind=' || m.copyKind) = 'Device-to-Host';
        create temp table _lzvm_kernel_summary_stream_end as
            select
                streamId as stream_id,
                end as kernel_end,
                shortName as kernel_name_id
            from {KERNEL_TABLE};
        create index _lzvm_kernel_summary_stream_end_idx
            on _lzvm_kernel_summary_stream_end(stream_id, kernel_end);
        """
    )
    return conn.execute(
        f"""
        with d2h_with_kernel as (
            select
                d2h.*,
                (
                    select k.kernel_name_id
                    from _lzvm_kernel_summary_stream_end k
                    where k.stream_id = d2h.stream_id
                      and k.kernel_end <= d2h.memcpy_start
                    order by k.kernel_end desc
                    limit 1
                ) as previous_kernel_name_id
            from _lzvm_kernel_summary_d2h_memcpy d2h
        )
        select
            bytes,
            coalesce(kernel.value, 'unresolved') as previous_kernel,
            count(*) as calls,
            sum(runtime_end - runtime_start) as host_ns,
            sum(memcpy_end - memcpy_start) as gpu_ns
        from d2h_with_kernel
        left join {STRING_TABLE} kernel on kernel.id = previous_kernel_name_id
        group by bytes, previous_kernel
        order by host_ns desc, calls desc, bytes asc, previous_kernel asc
        limit 1
        """
    ).fetchone()


def kernel_launch_timeline(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            k.streamId as stream_id,
            k.start as kernel_start_ns,
            {kernel_name_expr()} as kernel,
            coalesce(r.end - r.start, 0) as launch_ns,
            k.end - k.start as gpu_ns
        from {KERNEL_TABLE} k
        left join {RUNTIME_TABLE} r on r.correlationId = k.correlationId
        left join {STRING_TABLE} api on api.id = r.nameId
        left join {STRING_TABLE} short on short.id = k.shortName
        left join {STRING_TABLE} demangled on demangled.id = k.demangledName
        where api.value is null
           or api.value like 'cudaLaunchKernel%'
           or api.value like 'cudaGraphLaunch%'
        order by k.streamId asc, k.start asc, k.end asc
        """
    ).fetchall()


def kernel_adjacency_summary(conn: sqlite3.Connection, limit: int) -> list[dict[str, object]]:
    pairs: dict[tuple[str, str], dict[str, object]] = {}
    previous_by_stream: dict[int, sqlite3.Row] = {}
    for row in kernel_launch_timeline(conn):
        stream_id = int(row["stream_id"] or 0)
        previous = previous_by_stream.get(stream_id)
        if previous is not None:
            key = (str(previous["kernel"]), str(row["kernel"]))
            entry = pairs.setdefault(
                key,
                {
                    "previous_kernel": key[0],
                    "next_kernel": key[1],
                    "calls": 0,
                    "launch_ns": 0,
                    "gpu_ns": 0,
                },
            )
            entry["calls"] = int(entry["calls"]) + 1
            entry["launch_ns"] = int(entry["launch_ns"]) + int(previous["launch_ns"] or 0) + int(
                row["launch_ns"] or 0
            )
            entry["gpu_ns"] = int(entry["gpu_ns"]) + int(previous["gpu_ns"] or 0) + int(
                row["gpu_ns"] or 0
            )
        previous_by_stream[stream_id] = row
    rows = list(pairs.values())
    rows.sort(
        key=lambda row: (
            -int(row["launch_ns"]),
            -int(row["calls"]),
            -int(row["gpu_ns"]),
            str(row["previous_kernel"]),
            str(row["next_kernel"]),
        )
    )
    return rows[:limit]


def print_kernel_gpu(rows: list[sqlite3.Row]) -> None:
    print("kernel_gpu_activity")
    print("kernel,calls,kernel_gpu_ms,avg_kernel_us,max_kernel_us,grid_x_range,block_x_range")
    for row in rows:
        calls = int(row["calls"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        avg = gpu_ns / calls if calls else 0
        print(
            f"{row['kernel']},{calls},{ms(gpu_ns):.3f},{us(avg):.3f},"
            f"{us(row['max_gpu_ns']):.3f},{int(row['min_grid_x'] or 0)}.."
            f"{int(row['max_grid_x'] or 0)},{int(row['min_block_x'] or 0)}.."
            f"{int(row['max_block_x'] or 0)}"
        )
    if not rows:
        print("none,0,0.000,0.000,0.000,0..0,0..0")


def print_launch_api(rows: list[sqlite3.Row]) -> None:
    print()
    print("runtime_cuda_kernel_launch_api")
    print("api,calls,launch_api_ms,avg_launch_api_us")
    for row in rows:
        calls = int(row["calls"] or 0)
        launch_ns = int(row["launch_ns"] or 0)
        avg = launch_ns / calls if calls else 0
        print(f"{row['api']},{calls},{ms(launch_ns):.3f},{us(avg):.3f}")
    if not rows:
        print("none,0,0.000,0.000")


def print_sync_api(rows: list[sqlite3.Row]) -> None:
    print()
    print("runtime_cuda_sync_api")
    print("api,return_code,calls,sync_api_ms,avg_sync_api_us,max_sync_api_ms")
    for row in rows:
        calls = int(row["calls"] or 0)
        sync_ns = int(row["sync_ns"] or 0)
        avg = sync_ns / calls if calls else 0
        print(
            f"{row['api']},{int(row['return_code'] or 0)},{calls},"
            f"{ms(sync_ns):.3f},{us(avg):.3f},{ms(row['max_sync_ns']):.3f}"
        )
    if not rows:
        print("none,0,0,0.000,0.000,0.000")


def print_correlated_launch(rows: list[sqlite3.Row]) -> None:
    print()
    print("correlated_kernel_launch_waits")
    print("kernel,calls,launch_api_ms,kernel_gpu_ms,launch_to_kernel_ratio")
    for row in rows:
        launch_ns = int(row["launch_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        print(
            f"{row['kernel']},{int(row['calls'] or 0)},{ms(launch_ns):.3f},"
            f"{ms(gpu_ns):.3f},{ratio(launch_ns, gpu_ns)}"
        )
    if not rows:
        print("none,0,0.000,0.000,0.000")


def print_streams(rows: list[sqlite3.Row]) -> None:
    print()
    print("stream_kernel_activity")
    print("stream_id,calls,kernel_gpu_ms,active_window_ms,occupancy_ratio")
    for row in rows:
        gpu_ns = int(row["gpu_ns"] or 0)
        first_ns = int(row["first_ns"] or 0)
        last_ns = int(row["last_ns"] or first_ns)
        window_ns = max(0, last_ns - first_ns)
        print(
            f"{int(row['stream_id'] or 0)},{int(row['calls'] or 0)},"
            f"{ms(gpu_ns):.3f},{ms(window_ns):.3f},{ratio(gpu_ns, window_ns)}"
        )
    if not rows:
        print("0,0,0.000,0.000,0.000")


def print_fusion_candidates(rows: list[sqlite3.Row]) -> None:
    print()
    print("fusion_candidates")
    print("kernel,calls,launch_api_ms,kernel_gpu_ms,avg_kernel_us,launch_to_kernel_ratio")
    for row in rows:
        calls = int(row["calls"] or 0)
        launch_ns = int(row["launch_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        avg = gpu_ns / calls if calls else 0
        print(
            f"{row['kernel']},{calls},{ms(launch_ns):.3f},{ms(gpu_ns):.3f},"
            f"{us(avg):.3f},{ratio(launch_ns, gpu_ns)}"
        )
    if not rows:
        print("none,0,0.000,0.000,0.000,0.000")


def print_graph_shape_candidates(rows: list[sqlite3.Row]) -> None:
    print()
    print("graph_shape_candidates")
    print(
        "kernel,grid_x,block_x,streams,calls,launch_api_ms,kernel_gpu_ms,"
        "avg_kernel_us,launch_to_kernel_ratio"
    )
    for row in rows:
        calls = int(row["calls"] or 0)
        launch_ns = int(row["launch_ns"] or 0)
        gpu_ns = int(row["gpu_ns"] or 0)
        avg = gpu_ns / calls if calls else 0
        print(
            f"{row['kernel']},{int(row['grid_x'] or 0)},{int(row['block_x'] or 0)},"
            f"{int(row['streams'] or 0)},{calls},{ms(launch_ns):.3f},"
            f"{ms(gpu_ns):.3f},{us(avg):.3f},{ratio(launch_ns, gpu_ns)}"
        )
    if not rows:
        print("none,0,0,0,0,0.000,0.000,0.000,0.000")


def print_kernel_adjacency(rows: list[dict[str, object]]) -> None:
    print()
    print("kernel_adjacency_candidates")
    print(
        "previous_kernel,next_kernel,calls,pair_launch_api_ms,"
        "pair_kernel_gpu_ms,launch_to_kernel_ratio"
    )
    for row in rows:
        launch_ns = int(row["launch_ns"])
        gpu_ns = int(row["gpu_ns"])
        print(
            f"{row['previous_kernel']},{row['next_kernel']},{int(row['calls'])},"
            f"{ms(launch_ns):.3f},{ms(gpu_ns):.3f},{ratio(launch_ns, gpu_ns)}"
        )
    if not rows:
        print("none,none,0,0.000,0.000,0.000")


def sum_rows_ns(rows: list[sqlite3.Row], column: str) -> int:
    return sum(int(row[column] or 0) for row in rows)


def top_stream_stats(stream_rows: list[sqlite3.Row]) -> tuple[int, int, int, int]:
    if not stream_rows:
        return (0, 0, 0, 0)
    row = stream_rows[0]
    gpu_ns = int(row["gpu_ns"] or 0)
    first_ns = int(row["first_ns"] or 0)
    last_ns = int(row["last_ns"] or first_ns)
    window_ns = max(0, last_ns - first_ns)
    idle_ns = max(0, window_ns - gpu_ns)
    return (int(row["stream_id"] or 0), gpu_ns, window_ns, idle_ns)


def direction_hint(launch_ns: int, sync_ns: int) -> str:
    if sync_ns > launch_ns * 5 // 4:
        return "sync_boundary_before_graph_or_fusion"
    if launch_ns > sync_ns * 5 // 4:
        return "graph_or_kernel_fusion"
    return "mixed_launch_and_sync"


def next_action_hint(
    launch_ns: int,
    sync_ns: int,
    stream_count: int,
    top_stream_window_ns: int,
    top_stream_idle_ns: int,
) -> tuple[str, str]:
    if top_stream_window_ns > 0 and top_stream_idle_ns * 4 > top_stream_window_ns:
        return (
            "inspect_stream_idle_or_cpu_producer",
            "top kernel stream is idle for more than a quarter of its active window",
        )
    if sync_ns > launch_ns * 5 // 4:
        if stream_count <= 1:
            return (
                "remove_sync_boundaries_or_keep_roots_on_device",
                "sync dominates and only one kernel stream is active",
            )
        return (
            "inspect_cross_stream_sync_or_data_residency",
            "sync dominates despite multiple kernel streams",
        )
    if launch_ns > sync_ns * 5 // 4:
        return (
            "graph_or_fuse_repeated_launch_shapes",
            "launch API time dominates synchronization time",
        )
    return (
        "measure_launch_and_sync_candidates_together",
        "launch and synchronization costs are both material",
    )


def print_direction_triage(
    launch_rows: list[sqlite3.Row],
    sync_rows: list[sqlite3.Row],
    stream_rows: list[sqlite3.Row],
    fusion_rows: list[sqlite3.Row],
    graph_shape_rows: list[sqlite3.Row],
    adjacency_rows: list[dict[str, object]],
    memcpy_rows: list[sqlite3.Row],
    top_d2h: sqlite3.Row | None,
) -> None:
    launch_ns = sum_rows_ns(launch_rows, "launch_ns")
    sync_ns = sum_rows_ns(sync_rows, "sync_ns")
    stream_count = len(stream_rows)
    top_stream_id, top_stream_gpu_ns, top_stream_window_ns, top_stream_idle_ns = top_stream_stats(
        stream_rows
    )
    top_fusion = fusion_rows[0] if fusion_rows else None
    top_shape = graph_shape_rows[0] if graph_shape_rows else None
    top_pair = adjacency_rows[0] if adjacency_rows else None
    next_action, next_action_detail = next_action_hint(
        launch_ns,
        sync_ns,
        stream_count,
        top_stream_window_ns,
        top_stream_idle_ns,
    )
    print()
    print("cuda_graph_fusion_separation_triage")
    print("metric,value,detail")
    print(
        f"launch_api_ms,{ms(launch_ns):.3f},"
        "host launch time available to CUDA Graph or kernel fusion"
    )
    print(
        f"graph_or_fusion_upper_bound_ms,{ms(launch_ns):.3f},"
        "launch API time before any synchronization or transfer costs"
    )
    print(
        f"sync_api_ms,{ms(sync_ns):.3f},"
        "host synchronization time that Graph or fusion may not remove"
    )
    print(
        f"sync_to_launch_ratio,{ratio(sync_ns, launch_ns)},"
        "values above 1 mean synchronization dominates launch overhead"
    )
    dominant = "sync_api" if sync_ns > launch_ns else "launch_api"
    print(f"dominant_wait,{dominant},{direction_hint(launch_ns, sync_ns)}")
    print(f"stream_count,{stream_count},kernel streams observed in nsys export")
    print(f"top_stream_id,{top_stream_id},kernel stream with largest GPU work")
    print(
        f"top_stream_occupancy_ratio,{ratio(top_stream_gpu_ns, top_stream_window_ns)},"
        "top stream kernel GPU time divided by its active window"
    )
    print(
        f"top_stream_idle_ms,{ms(top_stream_idle_ns):.3f},"
        "active-window time not covered by kernels on the top stream"
    )
    print(
        f"launch_to_top_stream_idle_ratio,{ratio(launch_ns, top_stream_idle_ns)},"
        "CUDA launch API time relative to top-stream idle time"
    )
    print(f"next_action_hint,{next_action},{next_action_detail}")
    if top_fusion is None:
        print("top_fusion_kernel,none,no repeated launch candidate")
    else:
        print(
            f"top_fusion_kernel,{top_fusion['kernel']},"
            f"calls={int(top_fusion['calls'] or 0)} "
            f"launch_api_ms={ms(top_fusion['launch_ns']):.3f}"
        )
    if top_shape is None:
        print("top_graph_shape,none,no repeated same-shape launch candidate")
    else:
        print(
            f"top_graph_shape,{top_shape['kernel']},"
            f"grid_x={int(top_shape['grid_x'] or 0)} "
            f"block_x={int(top_shape['block_x'] or 0)} "
            f"calls={int(top_shape['calls'] or 0)} "
            f"launch_api_ms={ms(top_shape['launch_ns']):.3f}"
        )
    if top_pair is None:
        print("top_same_stream_pair,none,no same-stream adjacency candidate")
    else:
        print(
            "top_same_stream_pair,"
            f"{top_pair['previous_kernel']}->{top_pair['next_kernel']},"
            f"calls={int(top_pair['calls'])} "
            f"pair_launch_api_ms={ms(int(top_pair['launch_ns'])):.3f}"
        )
    print(
        "kernel_separation_hint,use_ncu_occupancy_before_splitting,"
        "nsys identifies launch and sync shape but not resource limits"
    )
    host_memcpy_ns = sum_rows_ns(memcpy_rows, "host_ns")
    gpu_memcpy_ns = sum_rows_ns(memcpy_rows, "gpu_ns")
    print(
        f"host_memcpy_api_ms,{ms(host_memcpy_ns):.3f},"
        "host time spent inside cudaMemcpy APIs"
    )
    print(f"gpu_memcpy_ms,{ms(gpu_memcpy_ns):.3f},GPU copy engine activity")
    if top_d2h is None:
        if memcpy_rows:
            detail = "no Device-to-Host copy hotspot in this profile"
        else:
            detail = "nsys export lacks CUDA memcpy tables"
        print(f"top_d2h_wait,none,{detail}")
        print("transfer_residency_hint,none,no D2H hotspot visible to this summary")
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
    elif host_memcpy_ns > gpu_memcpy_ns * 5:
        hint = "reduce_host_round_trips_before_launch_fusion"
    else:
        hint = "inspect_transfer_callchains_before_copy_refactor"
    print(
        f"transfer_residency_hint,{hint},"
        "prioritize data residency before relying on Graph or fusion speedups"
    )


def summarize(conn: sqlite3.Connection, label: str, limit: int) -> None:
    require_tables(conn)
    conn.row_factory = sqlite3.Row
    launch_rows = launch_api_summary(conn)
    sync_rows = sync_api_summary(conn)
    stream_rows = stream_kernel_summary(conn, limit)
    fusion_rows = fusion_candidate_summary(conn, limit)
    graph_shape_rows = graph_shape_candidate_summary(conn, limit)
    adjacency_rows = kernel_adjacency_summary(conn, limit)
    memcpy_rows = memcpy_direction_summary(conn)
    top_d2h = d2h_preceding_kernel_summary(conn)
    print(f"profile={label}")
    print_kernel_gpu(kernel_gpu_summary(conn, limit))
    print_launch_api(launch_rows)
    print_sync_api(sync_rows)
    print_correlated_launch(correlated_launch_summary(conn, limit))
    print_streams(stream_rows)
    print_fusion_candidates(fusion_rows)
    print_graph_shape_candidates(graph_shape_rows)
    print_kernel_adjacency(adjacency_rows)
    print_direction_triage(
        launch_rows,
        sync_rows,
        stream_rows,
        fusion_rows,
        graph_shape_rows,
        adjacency_rows,
        memcpy_rows,
        top_d2h,
    )


def build_self_test_db() -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    conn.executescript(
        f"""
        create table {STRING_TABLE} (id integer primary key, value text);
        create table {MEMCPY_KIND_TABLE} (id integer primary key, label text);
        create table {RUNTIME_TABLE} (
            start integer,
            end integer,
            eventClass integer,
            globalTid integer,
            correlationId integer,
            nameId integer,
            returnValue integer,
            callchainId integer
        );
        create table {KERNEL_TABLE} (
            start integer not null,
            end integer not null,
            deviceId integer not null,
            contextId integer not null,
            streamId integer not null,
            correlationId integer,
            demangledName integer not null,
            shortName integer not null,
            registersPerThread integer not null,
            gridX integer not null,
            gridY integer not null,
            gridZ integer not null,
            blockX integer not null,
            blockY integer not null,
            blockZ integer not null,
            staticSharedMemory integer not null,
            dynamicSharedMemory integer not null,
            localMemoryPerThread integer not null,
            localMemoryTotal integer not null,
            gridId integer not null
        );
        create table {MEMCPY_TABLE} (
            start integer,
            end integer,
            bytes integer,
            copyKind integer,
            correlationId integer,
            streamId integer
        );
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [
            (1, "cudaLaunchKernel_v7000"),
            (2, "ntt_stage_kernel"),
            (3, "poseidon2_width16_merkle_parent_kernel"),
            (4, "cudaDeviceSynchronize_v3020"),
            (5, "cudaMemcpyAsync_v3020"),
        ],
    )
    conn.executemany(
        f"insert into {MEMCPY_KIND_TABLE} (id, label) values (?, ?)",
        [
            (1, "Host-to-Device"),
            (2, "Device-to-Host"),
        ],
    )
    conn.executemany(
        f"""
        insert into {RUNTIME_TABLE}
            (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
        values (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        [
            (0, 20_000, 0, 0, 10, 1, 0, 0),
            (50_000, 90_000, 0, 0, 11, 1, 0, 0),
            (100_000, 150_000, 0, 0, 12, 1, 0, 0),
            (160_000, 230_000, 0, 0, 0, 4, 0, 0),
            (240_000, 260_000, 0, 0, 13, 1, 0, 0),
            (262_000, 1_862_000, 0, 0, 20, 5, 0, 0),
        ],
    )
    conn.executemany(
        f"""
        insert into {KERNEL_TABLE}
            (start, end, deviceId, contextId, streamId, correlationId, demangledName, shortName,
             registersPerThread, gridX, gridY, gridZ, blockX, blockY, blockZ,
             staticSharedMemory, dynamicSharedMemory, localMemoryPerThread,
             localMemoryTotal, gridId)
        values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        [
            (1_000, 11_000, 0, 0, 7, 10, 2, 2, 64, 32, 1, 1, 256, 1, 1, 0, 0, 0, 0, 1),
            (51_000, 71_000, 0, 0, 7, 11, 2, 2, 64, 64, 1, 1, 256, 1, 1, 0, 0, 0, 0, 2),
            (101_000, 151_000, 0, 0, 9, 12, 3, 3, 96, 8, 1, 1, 128, 1, 1, 0, 0, 0, 0, 3),
            (241_000, 261_000, 0, 0, 7, 13, 2, 2, 64, 64, 1, 1, 256, 1, 1, 0, 0, 0, 0, 4),
        ],
    )
    conn.executemany(
        f"""
        insert into {MEMCPY_TABLE}
            (start, end, bytes, copyKind, correlationId, streamId)
        values (?, ?, ?, ?, ?, ?)
        """,
        [
            (262_100, 262_600, 1152, 2, 20, 7),
        ],
    )
    return conn


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize CUDA kernel launch and stream shape from an Nsight Systems SQLite export."
    )
    parser.add_argument("sqlite", nargs="?", help="path to an nsys SQLite export")
    parser.add_argument("--top", type=int, default=16, help="rows to print per summary")
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
