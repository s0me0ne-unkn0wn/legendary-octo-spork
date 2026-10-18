from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

from measure_throughput import measure


def memory_metrics():
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    vm = dict(line.split() for line in Path("/proc/vmstat").read_text().splitlines())
    pressure = {
        parts[0]: int(parts[-1].split("=")[1])
        for line in Path("/proc/pressure/memory").read_text().splitlines()
        if (parts := line.split())
    }
    return {
        "available_kib": int(memory["MemAvailable"].split()[0]),
        "swap_in_pages": int(vm["pswpin"]),
        "swap_out_pages": int(vm["pswpout"]),
        "some_pressure_us": pressure["some"],
        "full_pressure_us": pressure["full"],
    }


root = Path(__file__).resolve().parents[2]
directory = root / "target/master-benchmark"
report = json.loads((root / ".agent/worklog/master-benchmark.json").read_text())
previous = next(result for result in report["results"] if result["pattern"] == "deposits" and result["rows"] == 100_000_000)
binary = directory / "master"
input_path = directory / "deposits-100000000.csv"
assert subprocess.check_output(["git", "rev-parse", "HEAD"], text=True, cwd=root).strip() == report["binary"]["commit"]
assert hashlib.sha256(binary.read_bytes()).hexdigest() == report["binary"]["binary_sha256"]
with input_path.open("rb") as stream:
    assert hashlib.file_digest(stream, "sha256").hexdigest() == previous["input_sha256"]
output = input_path.with_suffix(".expected.csv").read_bytes()
assert hashlib.sha256(output).hexdigest() == previous["expected_output_sha256"]
readme = (root / "README.md").read_bytes()
before = memory_metrics()
print("input and binary match; starting one 100M-deposit run", flush=True)
sample = measure(binary, input_path, output, 1)[0]
after = memory_metrics()
assert (root / "README.md").read_bytes() == readme
result = {
    "date_utc": datetime.now(timezone.utc).isoformat(),
    "binary": report["binary"],
    "rows": previous["rows"],
    "input_sha256": previous["input_sha256"],
    "sample": sample,
    "rows_per_second": previous["rows"] / sample["seconds"],
    "mib_per_second": previous["input_bytes"] / 1024**2 / sample["seconds"],
    "throughput_change_pct": (previous["median_seconds"] / sample["seconds"] - 1) * 100,
    "memory_before": before,
    "memory_after": after,
    "counter_changes": {key: after[key] - before[key] for key in before if key != "available_kib"},
    "readme_sha256": hashlib.sha256(readme).hexdigest(),
}
(root / ".agent/worklog/deposits-repeat.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2), flush=True)
