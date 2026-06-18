#!/usr/bin/env python3
import argparse
import sqlite3
import sys
from contextlib import closing
from pathlib import Path

COMPOSITE_EVENTS_TABLE = "COMPOSITE_EVENTS"
SAMPLING_CALLCHAINS_TABLE = "SAMPLING_CALLCHAINS"
SAMPLING_THREAD_STATE_TABLE = "ENUM_SAMPLING_THREAD_STATE"
THREAD_NAMES_TABLE = "ThreadNames"
STRING_TABLE = "StringIds"


def csv_cell(value: object) -> str:
    return str(value).replace(",", " ").replace("\n", " ").replace("\r", " ")


def pct(part: int | float, total: int | float) -> str:
    total = float(total or 0)
    if total == 0:
        return "0.000"
    return f"{float(part or 0) * 100.0 / total:.3f}"


def table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "select 1 from sqlite_master where type = 'table' and name = ?",
        (name,),
    ).fetchone()
    return row is not None


def require_tables(conn: sqlite3.Connection) -> None:
    missing = [
        table
        for table in [COMPOSITE_EVENTS_TABLE, SAMPLING_CALLCHAINS_TABLE, STRING_TABLE]
        if not table_exists(conn, table)
    ]
    if missing:
        raise SystemExit(f"missing nsys SQLite tables: {', '.join(missing)}")


def sampling_rows(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        select
            coalesce(sym.value, 'symbol=' || c.symbol) as symbol,
            coalesce(mod.value, 'module=' || c.module) as module,
            count(*) as samples
        from {COMPOSITE_EVENTS_TABLE} e
        join {SAMPLING_CALLCHAINS_TABLE} c
          on c.id = e.id
         and c.stackDepth = 0
        left join {STRING_TABLE} sym on sym.id = c.symbol
        left join {STRING_TABLE} mod on mod.id = c.module
        group by symbol, module
        order by samples desc, symbol asc, module asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def sample_count(conn: sqlite3.Connection) -> int:
    return int(
        conn.execute(
            f"""
            select count(*)
            from {COMPOSITE_EVENTS_TABLE} e
            join {SAMPLING_CALLCHAINS_TABLE} c
              on c.id = e.id
             and c.stackDepth = 0
            """
        ).fetchone()[0]
        or 0
    )


def is_application_module(module: str) -> bool:
    module = module.strip()
    if module == "lzvm":
        return True
    path = Path(module)
    return path.name == "lzvm" and (
        "target" in path.parts or module.endswith("/target/release/lzvm")
    )


def application_rows(conn: sqlite3.Connection, limit: int) -> list[sqlite3.Row]:
    all_rows = conn.execute(
        f"""
        select
            coalesce(sym.value, 'symbol=' || c.symbol) as symbol,
            coalesce(mod.value, 'module=' || c.module) as module,
            count(*) as samples
        from {COMPOSITE_EVENTS_TABLE} e
        join {SAMPLING_CALLCHAINS_TABLE} c
          on c.id = e.id
         and c.stackDepth = 0
        left join {STRING_TABLE} sym on sym.id = c.symbol
        left join {STRING_TABLE} mod on mod.id = c.module
        group by symbol, module
        order by samples desc, symbol asc, module asc
        """
    ).fetchall()
    return [row for row in all_rows if is_application_module(str(row["module"]))][:limit]


def emit_summary(conn: sqlite3.Connection, limit: int) -> None:
    require_tables(conn)
    total_samples = sample_count(conn)
    rows = sampling_rows(conn, limit)

    print("top_cpu_self_samples")
    print("symbol,module,samples,cpu_sample_pct")
    for row in rows:
        samples = int(row["samples"] or 0)
        print(
            ",".join(
                [
                    csv_cell(row["symbol"]),
                    csv_cell(row["module"]),
                    str(samples),
                    pct(samples, total_samples),
                ]
            )
        )

    app_rows = application_rows(conn, limit)
    app_total = sum(int(row["samples"] or 0) for row in app_rows)
    print("application_cpu_hotspots")
    print("symbol,module,samples,application_sample_pct")
    for row in app_rows:
        samples = int(row["samples"] or 0)
        print(
            ",".join(
                [
                    csv_cell(row["symbol"]),
                    csv_cell(row["module"]),
                    str(samples),
                    pct(samples, app_total),
                ]
            )
        )


def run_self_test() -> None:
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    conn.executescript(
        f"""
        create table {STRING_TABLE} (id integer primary key, value text);
        create table {COMPOSITE_EVENTS_TABLE} (
            id integer,
            start integer,
            cpu integer,
            threadState integer,
            globalTid integer,
            cpuCycles integer
        );
        create table {SAMPLING_CALLCHAINS_TABLE} (
            id integer,
            symbol integer,
            module integer,
            kernelMode integer,
            thumbCode integer,
            unresolved integer,
            specialEntry integer,
            originalIP integer,
            unwindMethod integer,
            stackDepth integer
        );
        create table {SAMPLING_THREAD_STATE_TABLE} (id integer primary key, label text);
        create table {THREAD_NAMES_TABLE} (nameId integer, priority integer, globalTid integer);
        """
    )
    conn.executemany(
        f"insert into {STRING_TABLE} (id, value) values (?, ?)",
        [
            (1, "apply_zisk_main_lowered_report_row"),
            (2, "advance_guest_machine_prepared_inner"),
            (3, "__memcpy_avx512_unaligned_erms"),
            (4, "lzvm"),
            (5, "/usr/lib64/libc.so.6"),
        ],
    )
    conn.executemany(
        f"""
        insert into {COMPOSITE_EVENTS_TABLE}
            (id, start, cpu, threadState, globalTid, cpuCycles)
        values (?, ?, 0, 0, 7, 1)
        """,
        [(sample_id, sample_id * 100) for sample_id in range(1, 8)],
    )
    conn.executemany(
        f"""
        insert into {SAMPLING_CALLCHAINS_TABLE}
            (id, symbol, module, kernelMode, thumbCode, unresolved, specialEntry,
             originalIP, unwindMethod, stackDepth)
        values (?, ?, ?, 0, 0, 0, 0, 0, 0, 0)
        """,
        [
            (1, 1, 4),
            (2, 1, 4),
            (3, 1, 4),
            (4, 2, 4),
            (5, 2, 4),
            (6, 3, 5),
            (7, 3, 5),
        ],
    )
    emit_summary(conn, 10)
    conn.close()


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize Nsight Systems CPU sampling self frames from SQLite exports."
    )
    parser.add_argument("sqlite", nargs="*", help="Nsight Systems SQLite export paths")
    parser.add_argument("--limit", type=int, default=25)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        run_self_test()
        return 0
    if not args.sqlite:
        raise SystemExit("usage: nsys-cpu-sampling-summary.py <nsys.sqlite> [...]")
    for index, path in enumerate(args.sqlite):
        if len(args.sqlite) > 1:
            if index:
                print()
            print(f"profile,{csv_cell(path)}")
        with closing(sqlite3.connect(path)) as conn:
            conn.row_factory = sqlite3.Row
            emit_summary(conn, args.limit)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
