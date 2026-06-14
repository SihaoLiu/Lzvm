#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

RUNTIME_TABLE = "CUPTI_ACTIVITY_KIND_RUNTIME"
STRING_TABLE = "StringIds"

SYNC_APIS = [
    "cudaDeviceSynchronize",
    "cudaStreamSynchronize",
    "cudaEventSynchronize",
    "cudaThreadSynchronize",
    "cudaStreamWaitEvent",
]


def ms(ns: int | float | None) -> float:
    return float(ns or 0) / 1_000_000.0


def us(ns: int | float | None) -> float:
    return float(ns or 0) / 1_000.0


def table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "select 1 from sqlite_master where type = 'table' and name = ?",
        (name,),
    ).fetchone()
    return row is not None


def require_tables(conn: sqlite3.Connection) -> None:
    missing = [
        table for table in [RUNTIME_TABLE, STRING_TABLE] if not table_exists(conn, table)
    ]
    if missing:
        raise SystemExit(f"missing nsys SQLite tables: {', '.join(missing)}")


def sync_api_predicate(alias: str = "s") -> str:
    return " or ".join(f"{alias}.value like '{api}%'" for api in SYNC_APIS)


def runtime_sync_summary(conn: sqlite3.Connection) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            coalesce(r.returnValue, 0) as return_code,
            count(*) as calls,
            sum(r.end - r.start) as host_ns,
            max(r.end - r.start) as max_host_ns
        from {RUNTIME_TABLE} r
        left join {STRING_TABLE} s on s.id = r.nameId
        where {sync_api_predicate()}
        group by api, return_code
        order by host_ns desc, calls desc, api asc, return_code asc
        """
    ).fetchall()


def sync_wait_candidate_summary(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(s.value, 'nameId=' || r.nameId) as api,
            coalesce(r.returnValue, 0) as return_code,
            coalesce(r.globalTid, 0) as global_tid,
            count(*) as calls,
            sum(r.end - r.start) as host_ns,
            max(r.end - r.start) as max_host_ns,
            min(r.start) as first_start_ns,
            max(r.end) as last_end_ns
        from {RUNTIME_TABLE} r
        left join {STRING_TABLE} s on s.id = r.nameId
        where {sync_api_predicate()}
        group by api, return_code, global_tid
        order by host_ns desc, max_host_ns desc, calls desc, api asc, global_tid asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def print_runtime_sync(rows: list[sqlite3.Row]) -> None:
    print("runtime_cuda_sync_api")
    print("api,return_code,calls,host_api_ms,avg_host_api_us,max_host_api_ms")
    for row in rows:
        calls = int(row["calls"] or 0)
        host_ns = int(row["host_ns"] or 0)
        avg = host_ns / calls if calls else 0
        print(
            f"{row['api']},{int(row['return_code'] or 0)},{calls},"
            f"{ms(host_ns):.3f},{us(avg):.3f},{ms(row['max_host_ns']):.3f}"
        )
    if not rows:
        print("none,0,0,0.000,0.000,0.000")


def print_sync_wait_candidates(rows: list[sqlite3.Row]) -> None:
    print()
    print("sync_wait_candidates")
    print(
        "api,return_code,global_tid,calls,host_api_ms,max_host_api_ms,"
        "active_window_ms"
    )
    for row in rows:
        first_ns = int(row["first_start_ns"] or 0)
        last_ns = int(row["last_end_ns"] or first_ns)
        window_ns = max(0, last_ns - first_ns)
        print(
            f"{row['api']},{int(row['return_code'] or 0)},"
            f"{int(row['global_tid'] or 0)},{int(row['calls'] or 0)},"
            f"{ms(row['host_ns']):.3f},{ms(row['max_host_ns']):.3f},"
            f"{ms(window_ns):.3f}"
        )
    if not rows:
        print("none,0,0,0,0.000,0.000,0.000")


def summarize(conn: sqlite3.Connection, label: str, limit: int) -> None:
    require_tables(conn)
    conn.row_factory = sqlite3.Row
    print(f"profile={label}")
    print_runtime_sync(runtime_sync_summary(conn))
    print_sync_wait_candidates(sync_wait_candidate_summary(conn, limit))


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
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [
            (1, "cudaDeviceSynchronize_v3020"),
            (2, "cudaStreamSynchronize_v3020"),
            (3, "cudaEventSynchronize_v3020"),
            (4, "cudaLaunchKernel_v7000"),
        ],
    )
    conn.executemany(
        f"""
        insert into {RUNTIME_TABLE}
            (start, end, eventClass, globalTid, correlationId, nameId, returnValue, callchainId)
        values (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        [
            (0, 2_500_000, 0, 100, 10, 1, 0, 0),
            (3_000_000, 4_500_000, 0, 100, 11, 2, 0, 0),
            (5_000_000, 7_000_000, 0, 101, 12, 3, 0, 0),
            (8_000_000, 8_030_000, 0, 100, 13, 4, 0, 0),
        ],
    )
    return conn


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize explicit CUDA synchronization waits from an Nsight Systems SQLite export."
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
