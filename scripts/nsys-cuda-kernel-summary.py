#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

RUNTIME_TABLE = "CUPTI_ACTIVITY_KIND_RUNTIME"
KERNEL_TABLE = "CUPTI_ACTIVITY_KIND_KERNEL"
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


def summarize(conn: sqlite3.Connection, label: str, limit: int) -> None:
    require_tables(conn)
    conn.row_factory = sqlite3.Row
    print(f"profile={label}")
    print_kernel_gpu(kernel_gpu_summary(conn, limit))
    print_launch_api(launch_api_summary(conn))
    print_correlated_launch(correlated_launch_summary(conn, limit))
    print_streams(stream_kernel_summary(conn, limit))
    print_fusion_candidates(fusion_candidate_summary(conn, limit))


def build_self_test_db() -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    conn.executescript(
        f"""
        create table {STRING_TABLE} (id integer primary key, value text);
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
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [
            (1, "cudaLaunchKernel_v7000"),
            (2, "ntt_stage_kernel"),
            (3, "poseidon2_width16_merkle_parent_kernel"),
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
