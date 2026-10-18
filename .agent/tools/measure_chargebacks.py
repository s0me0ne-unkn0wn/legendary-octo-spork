import argparse
from datetime import date
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from measure_throughput import decimal, measure


CLIENT_COUNT = 1024


def create_input(directory, pattern, rows):
    close_count = CLIENT_COUNT // 2 if pattern == "half" else CLIENT_COUNT
    groups, remainder = divmod(rows - close_count * 2, 4)
    assert remainder == 0 and groups >= CLIENT_COUNT
    path = directory / f"{pattern}-{rows}.csv"
    counts = [0] * (CLIENT_COUNT + 1)
    last_ids = [0] * (CLIENT_COUNT + 1)
    with path.open("w", buffering=1024 * 1024) as stream:
        stream.write("type,client,tx,amount\n")

        def write_group(client, group):
            transaction = group * 2
            stream.write(
                f"deposit,{client},{transaction},2.0001\n"
                f"withdrawal,{client},{transaction + 1},1.0000\n"
                f"dispute,{client},{transaction}\n"
                f"resolve,{client},{transaction}\n"
            )
            counts[client] += 1
            last_ids[client] = transaction

        def close_account(client):
            stream.write(
                f"dispute,{client},{last_ids[client]}\n"
                f"chargeback,{client},{last_ids[client]}\n"
            )

        if pattern == "late":
            for group in range(groups):
                write_group(group % CLIENT_COUNT + 1, group)
                if (group + 1) % 2_500_000 == 0:
                    print(f"generated {pattern}: {(group + 1) * 4:,} records", flush=True)
            for client in range(1, CLIENT_COUNT + 1):
                close_account(client)
        else:
            group = 0
            per_client, extra = divmod(groups, CLIENT_COUNT)
            for client in range(1, CLIENT_COUNT + 1):
                for _ in range(per_client + (client <= extra)):
                    write_group(client, group)
                    group += 1
                    if group % 2_500_000 == 0:
                        print(f"generated {pattern}: {group * 4:,} records", flush=True)
                if client <= close_count:
                    close_account(client)
            assert group == groups
    assert sum(counts) == groups and all(count > 0 for count in counts[1:])
    assert groups * 4 + close_count * 2 == rows
    output = ["client,available,held,total,locked\n"]
    for client in range(1, CLIENT_COUNT + 1):
        lock = client <= close_count
        total = decimal(counts[client] * 10_001 - (20_001 if lock else 0))
        output.append(f"{client},{total},0.0000,{total},{str(lock).lower()}\n")
    expected = "".join(output).encode()
    path.with_suffix(".expected.csv").write_bytes(expected)
    with path.open("rb") as stream:
        input_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    with path.open("rb") as stream:
        record_count = sum(chunk.count(b"\n") for chunk in iter(lambda: stream.read(1024 * 1024), b"")) - 1
    assert record_count == rows
    return path, expected, groups, close_count, input_hash


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rows", nargs="+", type=int, default=[1_000_000, 10_000_000, 100_000_000])
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--patterns", nargs="+", choices=["late", "half", "all"], default=["late", "half", "all"])
    parser.add_argument("--report", default=".agent/worklog/chargeback-index.json")
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    directory = root / "target/chargeback-benchmark"
    variants = json.loads((directory / "variants.json").read_text())
    for variant, metadata in variants.items():
        assert hashlib.sha256((directory / variant).read_bytes()).hexdigest() == metadata["binary_sha256"]
    report = {
        "date": date.today().isoformat(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "clients": CLIENT_COUNT,
        "rounds": arguments.rounds,
        "method": "median fresh CLI process time; maximum child VmHWM at 1 ms; complete output validation",
        "variants": variants,
        "results": [],
    }
    report_path = root / arguments.report
    for rows in arguments.rows:
        for pattern in arguments.patterns:
            print(f"generating {pattern}: {rows:,} records", flush=True)
            path, output, groups, close_count, input_hash = create_input(directory, pattern, rows)
            result = {
                "pattern": pattern,
                "rows": rows,
                "deposits": groups,
                "chargebacks": close_count,
                "input_bytes": path.stat().st_size,
                "input_sha256": input_hash,
                "expected_output_sha256": hashlib.sha256(output).hexdigest(),
                "variants": {variant: {"samples": []} for variant in variants},
            }
            report["results"].append(result)
            for round_index in range(arguments.rounds):
                order = list(variants) if round_index % 2 == 0 else list(reversed(variants))
                for variant in order:
                    sample = measure(directory / variant, path, output, 1)[0]
                    entry = result["variants"][variant]
                    entry["samples"].append(sample)
                    seconds = statistics.median(sample["seconds"] for sample in entry["samples"])
                    entry.update({
                        "median_seconds": seconds,
                        "rows_per_second": rows / seconds,
                        "mib_per_second": result["input_bytes"] / 1024**2 / seconds,
                        "peak_rss_kib": max(sample["peak_rss_kib"] for sample in entry["samples"]),
                    })
                    report_path.write_text(json.dumps(report, indent=2) + "\n")
                    print(
                        f'{pattern} {rows:,} {variant} sample {round_index + 1}: '
                        f'{sample["seconds"]:.3f} s, {sample["peak_rss_kib"] / 1024:.1f} MiB',
                        flush=True,
                    )
            summary = {
                variant: {key: value for key, value in entry.items() if key != "samples"}
                for variant, entry in result["variants"].items()
            }
            print(json.dumps({"complete": pattern, "rows": rows, "variants": summary}), flush=True)


if __name__ == "__main__":
    main()
