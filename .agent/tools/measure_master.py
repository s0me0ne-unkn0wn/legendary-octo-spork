import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from measure_chargebacks import CLIENT_COUNT, create_input as create_chargeback_input
from measure_throughput import decimal, measure


def create_input(directory, pattern, rows):
    if pattern in ("late", "half", "all"):
        return create_chargeback_input(directory, pattern, rows)
    assert rows >= CLIENT_COUNT * 4 and rows % 4 == 0
    path = directory / f"{pattern}-{rows}.csv"
    deposits = rows if pattern == "deposits" else rows // 4
    counts = [0] * (CLIENT_COUNT + 1)
    with path.open("w", buffering=1024 * 1024) as stream:
        stream.write("type,client,tx,amount\n")
        for index in range(deposits):
            client = index % CLIENT_COUNT + 1
            if pattern == "deposits":
                stream.write(f"deposit,{client},{index},1.0001\n")
            else:
                transaction = index * 2
                stream.write(
                    f"deposit,{client},{transaction},2.0001\n"
                    f"withdrawal,{client},{transaction + 1},1.0000\n"
                    f"dispute,{client},{transaction}\n"
                    f"resolve,{client},{transaction}\n"
                )
            counts[client] += 1
            if (index + 1) % 10_000_000 == 0:
                print(f"generated {pattern}: {index + 1:,} deposits", flush=True)
    output = ["client,available,held,total,locked\n"]
    for client in range(1, CLIENT_COUNT + 1):
        total = decimal(counts[client] * 10_001)
        output.append(f"{client},{total},0.0000,{total},false\n")
    expected = "".join(output).encode()
    path.with_suffix(".expected.csv").write_bytes(expected)
    with path.open("rb") as stream:
        input_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    with path.open("rb") as stream:
        record_count = sum(chunk.count(b"\n") for chunk in iter(lambda: stream.read(1024 * 1024), b"")) - 1
    assert record_count == rows and sum(counts) == deposits
    return path, expected, deposits, 0, input_hash


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rows", nargs="+", type=int, default=[1_000_000, 10_000_000, 100_000_000])
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--patterns", nargs="+", choices=["deposits", "mixed", "late", "half", "all"], default=["deposits", "mixed", "late", "half", "all"])
    parser.add_argument("--report", default=".agent/worklog/master-benchmark.json")
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    directory = root / "target/master-benchmark"
    metadata = json.loads((directory / "binary.json").read_text())
    binary = directory / "master"
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == metadata["binary_sha256"]
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], text=True, cwd=root).strip() == metadata["commit"]
    report = {
        "start_utc": datetime.now(timezone.utc).isoformat(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "kernel": subprocess.check_output(["uname", "-sr"], text=True).strip(),
        "clients": CLIENT_COUNT,
        "client_min": 1,
        "client_max": CLIENT_COUNT,
        "rounds": arguments.rounds,
        "method": "median fresh CLI process time; maximum child VmHWM sampled at 1 ms; complete output validation outside timing",
        "binary": metadata,
        "results": [],
    }
    report_path = root / arguments.report
    for rows in arguments.rows:
        for pattern in arguments.patterns:
            print(f"generating {pattern}: {rows:,} records", flush=True)
            path, output, deposits, chargebacks, input_hash = create_input(directory, pattern, rows)
            result = {
                "pattern": pattern,
                "rows": rows,
                "deposits": deposits,
                "chargebacks": chargebacks,
                "input_bytes": path.stat().st_size,
                "input_sha256": input_hash,
                "expected_output_sha256": hashlib.sha256(output).hexdigest(),
                "samples": [],
            }
            report["results"].append(result)
            for round_index in range(arguments.rounds):
                sample = measure(binary, path, output, 1)[0]
                assert sample["peak_rss_kib"] > 0
                result["samples"].append(sample)
                seconds = statistics.median(sample["seconds"] for sample in result["samples"])
                result.update({
                    "median_seconds": seconds,
                    "rows_per_second": rows / seconds,
                    "mib_per_second": result["input_bytes"] / 1024**2 / seconds,
                    "peak_rss_kib": max(sample["peak_rss_kib"] for sample in result["samples"]),
                })
                report_path.write_text(json.dumps(report, indent=2) + "\n")
                print(
                    f'{pattern} {rows:,} sample {round_index + 1}: '
                    f'{sample["seconds"]:.3f} s, {sample["peak_rss_kib"] / 1024:.1f} MiB',
                    flush=True,
                )
            print(json.dumps({key: value for key, value in result.items() if key != "samples"}), flush=True)
    report["end_utc"] = datetime.now(timezone.utc).isoformat()
    report_path.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
