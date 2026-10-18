from decimal import Decimal
import hashlib
import json
from pathlib import Path
import statistics
import subprocess


root = Path(__file__).resolve().parents[2]
directory = root / "target/master-benchmark"
report = json.loads((root / ".agent/worklog/master-benchmark.json").read_text())
start = json.loads((directory / "start.json").read_text())
assert report["end_utc"] and report["rounds"] == 3 and report["clients"] == 1024
assert report["binary"]["commit"] == start["commit"]
assert subprocess.check_output(["git", "rev-parse", "HEAD"], text=True, cwd=root).strip() == start["commit"]
assert subprocess.check_output(["git", "branch", "--show-current"], text=True, cwd=root).strip() == "master"
assert hashlib.sha256((directory / "master").read_bytes()).hexdigest() == report["binary"]["binary_sha256"]
assert not subprocess.check_output(["git", "diff", "HEAD", "--name-only", "--", "src", "tests", "benches", "Cargo.toml", "Cargo.lock"], cwd=root)
assert not subprocess.check_output(["git", "diff", "--cached", "--name-only"], cwd=root)
assert len(report["results"]) == 15
assert {(result["pattern"], result["rows"]) for result in report["results"]} == {
    (pattern, rows)
    for pattern in ["deposits", "mixed", "late", "half", "all"]
    for rows in [1_000_000, 10_000_000, 100_000_000]
}
previous = json.loads((root / ".agent/worklog/chargeback-index.json").read_text())
previous_cases = {(result["pattern"], result["rows"]): result for result in previous["results"]}
for result in report["results"]:
    pattern, rows = result["pattern"], result["rows"]
    close_count = 0 if pattern in ("deposits", "mixed") else 512 if pattern == "half" else 1024
    deposit_count = rows if pattern == "deposits" else (rows - close_count * 2) // 4
    assert result["deposits"] == deposit_count and result["chargebacks"] == close_count
    path = directory / f"{pattern}-{rows}.csv"
    input_hash = hashlib.sha256()
    newline_count = 0
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            input_hash.update(chunk)
            newline_count += chunk.count(b"\n")
    assert newline_count - 1 == rows and input_hash.hexdigest() == result["input_sha256"]
    assert path.stat().st_size == result["input_bytes"]
    output = ["client,available,held,total,locked\n"]
    per_client, extra = divmod(deposit_count, 1024)
    for client in range(1, 1025):
        count = per_client + (client <= extra)
        lock = client <= close_count
        total = Decimal(count * 10_001 - (20_001 if lock else 0)) / 10_000
        output.append(f"{client},{total:.4f},0.0000,{total:.4f},{str(lock).lower()}\n")
    output_bytes = "".join(output).encode()
    assert path.with_suffix(".expected.csv").read_bytes() == output_bytes
    assert hashlib.sha256(output_bytes).hexdigest() == result["expected_output_sha256"]
    account_rows = path.with_suffix(".output").read_bytes().splitlines()
    assert account_rows[:1] == output_bytes.splitlines()[:1]
    assert sorted(account_rows[1:]) == sorted(output_bytes.splitlines()[1:])
    assert path.with_suffix(".stderr").stat().st_size == 0
    samples = result["samples"]
    assert len(samples) == 3 and all(sample["seconds"] > 0 and sample["peak_rss_kib"] > 0 for sample in samples)
    assert result["median_seconds"] == statistics.median(sample["seconds"] for sample in samples)
    assert result["peak_rss_kib"] == max(sample["peak_rss_kib"] for sample in samples)
    assert result["rows_per_second"] == rows / result["median_seconds"]
    assert result["mib_per_second"] == result["input_bytes"] / 1024**2 / result["median_seconds"]
    if pattern in ("late", "half", "all"):
        assert result["input_sha256"] == previous_cases[pattern, rows]["input_sha256"]
    print(f"verified {pattern}: {rows:,} transactions", flush=True)
for rows in [1_000_000, 10_000_000, 100_000_000]:
    cases = {result["pattern"]: result for result in report["results"] if result["rows"] == rows}
    assert cases["late"]["expected_output_sha256"] == cases["all"]["expected_output_sha256"]
print("verified 15 cases, 45 samples, exact inputs, complete account outputs, empty diagnostics, report calculations, and master identity")
