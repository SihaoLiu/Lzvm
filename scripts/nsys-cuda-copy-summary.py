#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

RUNTIME_TABLE = "CUPTI_ACTIVITY_KIND_RUNTIME"
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


def summarize(conn: sqlite3.Connection, label: str, limit: int) -> None:
    require_tables(conn)
    conn.row_factory = sqlite3.Row
    print(f"profile={label}")
    print_runtime(runtime_memcpy_summary(conn))
    print_gpu(gpu_memcpy_summary(conn))
    print_correlated(correlated_memcpy_summary(conn, limit))


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
            correlationId integer
        );
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [(1, "cudaMemcpy_v3020"), (2, "cudaMemcpyAsync_v3020")],
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
    conn.executemany(
        f"""
        insert into {MEMCPY_TABLE} (start, end, bytes, copyKind, correlationId)
        values (?, ?, ?, ?, ?)
        """,
        [
            (10_000, 10_500, 32, 2, 10),
            (20_000, 20_700, 1152, 2, 11),
            (30_000, 35_000, 4096, 1, 12),
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
