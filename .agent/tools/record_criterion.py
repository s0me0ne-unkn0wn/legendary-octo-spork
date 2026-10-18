from datetime import date
import json
from pathlib import Path
import subprocess


def main():
    root = Path(__file__).resolve().parents[2]
    report = {
        "date": date.today().isoformat(),
        "command": "cargo bench --bench throughput --locked",
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "criterion": "0.8.2",
        "cpu": next(
            line.split(":", 1)[1].strip()
            for line in Path("/proc/cpuinfo").read_text().splitlines()
            if line.startswith("model name")
        ),
        "warmup_seconds": 3,
        "measurement_seconds": 10,
        "benchmarks": [],
    }
    for directory in sorted((root / "target/criterion/csv").glob("*/*/new")):
        benchmark = json.loads((directory / "benchmark.json").read_text())
        estimates = json.loads((directory / "estimates.json").read_text())
        sample = json.loads((directory / "sample.json").read_text())
        report["benchmarks"].append({
            "benchmark": benchmark,
            "estimates_ns": estimates,
            "sample": sample,
        })
        throughput = benchmark["throughput"]["ElementsAndBytes"]
        mean = estimates["mean"]
        interval = mean["confidence_interval"]
        seconds = mean["point_estimate"] / 1_000_000_000
        print(
            f'{benchmark["full_id"]}: mean={seconds * 1000:.2f} ms, '
            f'CI=[{interval["lower_bound"] / 1_000_000:.2f}, '
            f'{interval["upper_bound"] / 1_000_000:.2f}] ms, '
            f'records/s={throughput["elements"] / seconds / 1_000_000:.2f} million, '
            f'MiB/s={throughput["bytes"] / 1024**2 / seconds:.2f}'
        )
    (root / ".agent/worklog/benchmark.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
