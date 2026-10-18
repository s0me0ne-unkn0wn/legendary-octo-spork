# Benchmarks with 65,534 clients

Status: complete. This record accompanies the benchmark commit. The initial HEAD is 99c0a27. README contains user edits before this task; the index starts clean.

Definition of Done: measure deposit and mixed transaction throughput and peak process memory with exactly 65,534 clients, validate all output, append results to README without changing existing bytes, and commit only task changes.

Assumptions:
- The explicit client count takes priority over the inconsistent inclusive range. Use IDs 1..=65534, which contains exactly 65,534 clients.
- Use the existing default workload size: 1,000,000 deposits or 250,000 mixed groups. Each mixed group contains a deposit, withdrawal, dispute, and resolve. Final disputes and chargebacks add 131,068 records, for 1,131,068 mixed records total.
- Criterion reports in-memory throughput. Three fresh CLI processes per workload report file throughput and peak memory on the same CSV bytes. Input generation and validation do not contribute to CLI process memory.
- Reuse Linux VmHWM sampling at 1 ms from the existing measurement tool. GNU time does not exist in this environment.

Plan:
1. Save the current README bytes and diff. Add a client count argument to benchmark data generation and include it in Criterion IDs, so results cannot compare different client counts.
2. Generate matching file workloads and expected complete account CSV in .agent tooling. Validate client count, record count, balances, flags, formatting, and empty diagnostics.
3. Build and verify the benchmark, then measure Criterion workloads and CLI timing/memory sequentially without competing build or test processes.
4. Append tables with explicit measurement methods and the client ID assumption. Verify the original README prefix and all reported values.
5. Stage the append separately from the user's README edits, verify the staged diff, update task records, and create a local commit.

Implementation: TX_BENCH_CLIENTS configures the Criterion workload client count, with a default of 1,024 and a valid range of 1..=65535. Benchmark clients start at ID 1. Criterion IDs include client count. The measurement script generates identical file input, checks its record and byte counts against Criterion metadata, and validates all 65,534 output accounts after each CLI run. No production or test code changes.

Criterion results: 1,000,000 deposits average 194.45 ms, 5.14 million records/s, and 135.91 MiB/s. The 1,131,068 mixed records average 183.67 ms, 6.16 million records/s, and 144.23 MiB/s. Each workload uses 30 samples and 60 measured iterations. clients-65534.json contains metadata, estimates, raw samples, input hashes, and expected output hashes.

CLI results: the median of three deposit runs is 0.234 s, 4.27 million records/s, and 81.5 MiB peak process memory. Mixed records use 0.203 s, 5.58 million records/s, and 25.1 MiB. Memory figures use maximum child VmHWM samples, rather than wait4 resource statistics that can include the Python parent's pre-exec memory peak.

Verification: all 31 tests pass (4 unit tests and 27 integration tests). Formatting, Clippy with warnings as errors, and whitespace checks pass. Criterion test mode validates both 65,534-client workloads and both default 1,024-client workloads. The recorded sample means, confidence intervals, and CLI medians match the README values.

README protection: preserve the original 4,859 bytes with SHA-256 5f777f9436c59b1458845eb824167bb40b5cf15b26b7f6005281fe24224e28e0 exactly. Append both result tables and a reproduction command. The original bytes, initial user diff, appended bytes, and staging patch exist under target/clients-65534 for verification. Stage only the append; leave the initial user diff in the working tree.

Staging verification: the index contains exactly HEAD's README plus the appended result bytes. The initial verification confirms the original README prefix and user diff. A later concurrent README edit removes the appended section, so append the same result bytes to the latest file contents. Preserve that latest prefix byte for byte and leave it unstaged.

Commands: TX_BENCH_CLIENTS=65534 cargo bench --bench throughput --locked; python3 -B .agent/tools/measure_clients.py. The memory tool reuses existing CLI validation and VmHWM sampling. No new dependencies or production changes.

Open work: none.

Latest README prefix: 4859 bytes, SHA-256 5f777f9436c59b1458845eb824167bb40b5cf15b26b7f6005281fe24224e28e0. The working file equals that prefix plus the staged append.

Commit issue: the first commit attempt fails while writing Git tree objects. Thirteen .git/objects subdirectories belong to UID/GID 1000 and lack write access for the current UID 1001. The repository directory and index belong to the current user. No permission changes occur. All task files and the README append remain staged for a retry after recording this issue.
