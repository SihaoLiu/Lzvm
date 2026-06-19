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
MAX_APPLICATION_CALLCHAIN_FRAMES = 8


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


def hot_libc_nearest_application_callers(
    conn: sqlite3.Connection, limit: int
) -> list[sqlite3.Row]:
    return conn.execute(
        f"""
        with frames as (
            select
                e.id as event_id,
                c.stackDepth as stack_depth,
                coalesce(sym.value, 'symbol=' || c.symbol) as symbol,
                coalesce(mod.value, 'module=' || c.module) as module
            from {COMPOSITE_EVENTS_TABLE} e
            join {SAMPLING_CALLCHAINS_TABLE} c
              on c.id = e.id
            left join {STRING_TABLE} sym on sym.id = c.symbol
            left join {STRING_TABLE} mod on mod.id = c.module
        ),
        libc_events as (
            select event_id, symbol as libc_symbol
            from frames
            where stack_depth = 0
              and (
                    symbol like '%memcpy%'
                 or symbol like '%memmove%'
                 or symbol like '%memset%'
              )
              and not (
                    module = 'lzvm'
                 or module like '%/target/release/lzvm'
              )
        ),
        nearest_app_depth as (
            select
                f.event_id,
                min(f.stack_depth) as stack_depth
            from frames f
            join libc_events e on e.event_id = f.event_id
            where f.stack_depth > 0
              and (
                    f.module = 'lzvm'
                 or f.module like '%/target/release/lzvm'
              )
            group by f.event_id
        )
        select
            e.libc_symbol as libc_symbol,
            f.symbol as nearest_app_symbol,
            f.module as nearest_app_module,
            count(*) as samples
        from nearest_app_depth d
        join libc_events e on e.event_id = d.event_id
        join frames f
          on f.event_id = d.event_id
         and f.stack_depth = d.stack_depth
        group by e.libc_symbol, f.symbol, f.module
        order by samples desc, e.libc_symbol asc, f.symbol asc, f.module asc
        limit ?
        """,
        (limit,),
    ).fetchall()


def hot_libc_application_callchains(
    conn: sqlite3.Connection, limit: int
) -> list[dict[str, object]]:
    rows = conn.execute(
        f"""
        with frames as (
            select
                e.id as event_id,
                c.stackDepth as stack_depth,
                coalesce(sym.value, 'symbol=' || c.symbol) as symbol,
                coalesce(mod.value, 'module=' || c.module) as module
            from {COMPOSITE_EVENTS_TABLE} e
            join {SAMPLING_CALLCHAINS_TABLE} c
              on c.id = e.id
            left join {STRING_TABLE} sym on sym.id = c.symbol
            left join {STRING_TABLE} mod on mod.id = c.module
        ),
        libc_events as (
            select event_id, symbol as libc_symbol
            from frames
            where stack_depth = 0
              and (
                    symbol like '%memcpy%'
                 or symbol like '%memmove%'
                 or symbol like '%memset%'
              )
              and not (
                    module = 'lzvm'
                 or module like '%/target/release/lzvm'
              )
        )
        select
            e.libc_symbol as libc_symbol,
            f.event_id as event_id,
            f.stack_depth as stack_depth,
            f.symbol as symbol,
            f.module as module
        from libc_events e
        join frames f on f.event_id = e.event_id
        where f.stack_depth > 0
          and (
                f.module = 'lzvm'
             or f.module like '%/target/release/lzvm'
          )
        order by e.libc_symbol asc, f.event_id asc, f.stack_depth asc
        """
    ).fetchall()

    event_frames: dict[tuple[str, int], list[str]] = {}
    for row in rows:
        key = (str(row["libc_symbol"]), int(row["event_id"]))
        frames = event_frames.setdefault(key, [])
        if len(frames) < MAX_APPLICATION_CALLCHAIN_FRAMES:
            frames.append(str(row["symbol"]))

    chain_counts: dict[tuple[str, str], int] = {}
    for (libc_symbol, _event_id), frames in event_frames.items():
        if not frames:
            continue
        chain = " <= ".join(frames)
        key = (libc_symbol, chain)
        chain_counts[key] = chain_counts.get(key, 0) + 1

    sorted_counts = sorted(
        chain_counts.items(),
        key=lambda item: (-item[1], item[0][0], item[0][1]),
    )
    return [
        {
            "libc_symbol": libc_symbol,
            "application_callchain": application_callchain,
            "samples": samples,
        }
        for (libc_symbol, application_callchain), samples in sorted_counts[:limit]
    ]


def cpu_trace_memcpy_action_hint(nearest_app_symbol: str) -> str:
    if "run_guest_pc_trace_segment_slice" in nearest_app_symbol:
        return "trace_report_storage_structural_candidate"
    if (
        "GuestMachineMemory::read_range_into" in nearest_app_symbol
        or "read_guest_load" in nearest_app_symbol
    ):
        return "guest_memory_read_candidate"
    if (
        "GuestMachineMemorySegment::write_range" in nearest_app_symbol
        or "write_guest_store" in nearest_app_symbol
    ):
        return "guest_memory_write_candidate"
    return "none"


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

    libc_caller_rows = hot_libc_nearest_application_callers(conn, limit)
    libc_caller_total = sum(int(row["samples"] or 0) for row in libc_caller_rows)
    print("hot_libc_nearest_application_callers")
    print("libc_symbol,nearest_app_symbol,nearest_app_module,samples,libc_sample_pct")
    for row in libc_caller_rows:
        samples = int(row["samples"] or 0)
        print(
            ",".join(
                [
                    csv_cell(row["libc_symbol"]),
                    csv_cell(row["nearest_app_symbol"]),
                    csv_cell(row["nearest_app_module"]),
                    str(samples),
                    pct(samples, libc_caller_total),
                ]
            )
        )

    libc_callchain_rows = hot_libc_application_callchains(conn, limit)
    libc_callchain_total = sum(int(row["samples"] or 0) for row in libc_callchain_rows)
    print("hot_libc_application_callchains")
    print("libc_symbol,application_callchain,samples,libc_sample_pct")
    for row in libc_callchain_rows:
        samples = int(row["samples"] or 0)
        print(
            ",".join(
                [
                    csv_cell(row["libc_symbol"]),
                    csv_cell(row["application_callchain"]),
                    str(samples),
                    pct(samples, libc_callchain_total),
                ]
            )
        )

    print("cpu_trace_memcpy_action_hints")
    print("nearest_app_symbol,samples,libc_sample_pct,action_hint")
    for row in libc_caller_rows:
        action_hint = cpu_trace_memcpy_action_hint(str(row["nearest_app_symbol"]))
        if action_hint == "none":
            continue
        samples = int(row["samples"] or 0)
        print(
            ",".join(
                [
                    csv_cell(row["nearest_app_symbol"]),
                    str(samples),
                    pct(samples, libc_caller_total),
                    action_hint,
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
            (6, "run_guest_pc_trace_segment_slice"),
            (7, "produce_guest_pc_trace_pending_slices"),
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
        values (?, ?, ?, 0, 0, 0, 0, 0, 0, ?)
        """,
        [
            (1, 1, 4, 0),
            (2, 1, 4, 0),
            (3, 1, 4, 0),
            (4, 2, 4, 0),
            (5, 2, 4, 0),
            (6, 3, 5, 0),
            (7, 3, 5, 0),
            (6, 6, 4, 1),
            (7, 6, 4, 1),
            (6, 7, 4, 2),
            (7, 7, 4, 2),
        ],
    )
    emit_summary(conn, 10)
    conn.close()


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize Nsight Systems CPU sampling self frames from SQLite exports."
    )
    parser.add_argument("sqlite", nargs="*", help="Nsight Systems SQLite export paths")
    parser.add_argument("--limit", "--top", dest="limit", type=int, default=25)
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
