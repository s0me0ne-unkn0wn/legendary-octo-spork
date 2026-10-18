from datetime import date
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from measure_throughput import decimal, measure


CLIENT_COUNT = 65_534
DEPOSIT_ROWS = 1_000_000
MIXED_GROUPS = 250_000
ROUNDS = 3


def create_input(directory, workload):
    path = directory / f"{workload}.csv"
    counts = [0] * (CLIENT_COUNT + 1)
    last_ids = [0] * (CLIENT_COUNT + 1)
    with path.open("w", buffering=1024 * 1024) as stream:
        stream.write("type,client,tx,amount\n")
        if workload == "deposits":
            for transaction in range(DEPOSIT_ROWS):
                client = transaction % CLIENT_COUNT + 1
                stream.write(f"deposit,{client},{transaction},1.0001\n")
                counts[client] += 1
            rows = DEPOSIT_ROWS
        else:
            for group in range(MIXED_GROUPS):
                client = group % CLIENT_COUNT + 1
                transaction = group * 2
                stream.write(
                    f"deposit,{client},{transaction},2.0001\n"
                    f"withdrawal,{client},{transaction + 1},1.0000\n"
                    f"dispute,{client},{transaction}\n"
                    f"resolve,{client},{transaction}\n"
                )
                counts[client] += 1
                last_ids[client] = transaction
            for client in range(1, CLIENT_COUNT + 1):
                stream.write(
                    f"dispute,{client},{last_ids[client]}\n"
                    f"chargeback,{client},{last_ids[client]}\n"
                )
            rows = MIXED_GROUPS * 4 + CLIENT_COUNT * 2
    assert counts[0] == 0 and all(count > 0 for count in counts[1:])
    output = ["client,available,held,total,locked\n"]
    for client in range(1, CLIENT_COUNT + 1):
        total = decimal(counts[client] * 10_001 - (20_001 if workload == "mixed" else 0))
        lock = "true" if workload == "mixed" else "false"
        output.append(f"{client},{total},0.0000,{total},{lock}\n")
    return path, "".join(output).encode(), rows


def main():
    root = Path(__file__).resolve().parents[2]
    directory = root / "target/clients-65534"
    directory.mkdir(parents=True, exist_ok=True)
    report = {
        "date": date.today().isoformat(),
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "clients": CLIENT_COUNT,
        "client_min": 1,
        "client_max": CLIENT_COUNT,
        "memory_method": "maximum VmHWM sample at 1 ms across three CLI processes",
        "results": [],
    }
    binary = root / "target/release/legendary-octo-spork"
    for workload in ["deposits", "mixed"]:
        path, output, rows = create_input(directory, workload)
        criterion_path = root / f"target/criterion/csv/{workload}-{CLIENT_COUNT}-clients/{rows}/new"
        benchmark = json.loads((criterion_path / "benchmark.json").read_text())
        throughput = benchmark["throughput"]["ElementsAndBytes"]
        assert throughput == {"elements": rows, "bytes": path.stat().st_size}
        samples = measure(binary, path, output, ROUNDS)
        seconds = statistics.median(sample["seconds"] for sample in samples)
        result = {
            "workload": workload,
            "rows": rows,
            "input_bytes": path.stat().st_size,
            "input_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "expected_output_sha256": hashlib.sha256(output).hexdigest(),
            "criterion": {
                "benchmark": benchmark,
                "estimates_ns": json.loads((criterion_path / "estimates.json").read_text()),
                "sample": json.loads((criterion_path / "sample.json").read_text()),
            },
            "median_seconds": seconds,
            "rows_per_second": rows / seconds,
            "peak_rss_kib": max(sample["peak_rss_kib"] for sample in samples),
            "samples": samples,
        }
        report["results"].append(result)
        print(json.dumps({key: value for key, value in result.items() if key != "criterion"}), flush=True)
    (root / ".agent/worklog/clients-65534.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
