import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import time


def decimal(minor_units):
    magnitude = abs(minor_units)
    sign = "-" if minor_units < 0 else ""
    return f"{sign}{magnitude // 10_000}.{magnitude % 10_000:04}"


def create_input(directory, workload, rows):
    path = directory / f"{workload}-{rows}.csv"
    with path.open("w", buffering=1024 * 1024) as stream:
        stream.write("type,client,txid,amount\n")
        if workload == "deposits":
            stream.writelines(f"deposit,{index % 1024},{index},1.0001\n" for index in range(rows))
        else:
            stream.write("deposit,1,1,1\n")
            stream.writelines(
                f"{'dispute' if index % 2 == 0 else 'resolve'},1,1,\n"
                for index in range(rows - 1)
            )
    output = "client,available,held,total,locked\n"
    if workload == "deposits":
        for client in range(min(rows, 1024)):
            count = (rows - 1 - client) // 1024 + 1
            total = decimal(count * 10_001)
            output += f"{client},{total},0.0000,{total},false\n"
    else:
        on_hold = rows % 2 == 0
        output += f"1,{'0.0000,1.0000' if on_hold else '1.0000,0.0000'},1.0000,false\n"
    return path, output.encode()


def measure(binary, path, output, rounds):
    output_path = path.with_suffix(".output")
    diagnostic_path = path.with_suffix(".stderr")
    samples = []
    for _ in range(rounds):
        with output_path.open("wb") as stream, diagnostic_path.open("wb") as diagnostics:
            start = time.perf_counter()
            process = subprocess.Popen([str(binary), str(path)], stdout=stream, stderr=diagnostics)
            peak_rss_kib = 0
            status_path = Path(f"/proc/{process.pid}/status")
            while True:
                child_pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if child_pid != 0:
                    break
                try:
                    for line in status_path.read_text().splitlines():
                        if line.startswith("VmHWM:"):
                            peak_rss_kib = max(peak_rss_kib, int(line.split()[1]))
                except FileNotFoundError:
                    pass
                time.sleep(0.001)
            seconds = time.perf_counter() - start
            process.returncode = os.waitstatus_to_exitcode(status)
        if process.returncode != 0:
            raise RuntimeError(diagnostic_path.read_text())
        if diagnostic_path.stat().st_size != 0 or output_path.read_bytes() != output:
            raise RuntimeError("incorrect benchmark output")
        samples.append({
            "seconds": seconds,
            "peak_rss_kib": peak_rss_kib,
            "resource_peak_rss_kib": usage.ru_maxrss,
        })
    return samples


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rows", nargs="+", type=int, default=[1_000_000, 4_000_000])
    parser.add_argument("--rounds", type=int, default=3)
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    directory = root / "target/throughput-inputs"
    directory.mkdir(parents=True, exist_ok=True)
    binary = root / "target/release/legendary-octo-spork"
    results = []
    for workload in ["deposits", "disputes"]:
        for rows in arguments.rows:
            path, output = create_input(directory, workload, rows)
            samples = measure(binary, path, output, arguments.rounds)
            seconds = statistics.median(sample["seconds"] for sample in samples)
            result = {
                "workload": workload,
                "rows": rows,
                "input_bytes": path.stat().st_size,
                "median_seconds": seconds,
                "rows_per_second": rows / seconds,
                "peak_rss_kib": max(sample["peak_rss_kib"] for sample in samples),
                "samples": samples,
            }
            results.append(result)
            print(json.dumps(result), flush=True)
    report = {
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "results": results,
    }
    (root / ".agent/worklog/throughput.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
