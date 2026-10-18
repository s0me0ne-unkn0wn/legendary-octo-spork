# Error enum and Criterion

Status: complete. Commit e3bd6ec completes the error task. This record accompanies the benchmark commit. The working tree starts clean at cdd250e.

Definition of Done: add a simple error enum in a separate module, replace message-based error checks with variant and source checks, preserve diagnostics and stream behavior, and commit. Then replace custom benchmark timing with Criterion, measure current throughput and CLI memory, update documentation, verify, and create a second commit.

Facts:
- Pipeline functions already use anyhow for source chains. MoneyError and TransactionError provide domain variants.
- CSV operation context uses strings. Stream tests compare wording and CLI tests search for amount messages.
- The benchmark uses custom Instant timing and five-run medians. Input generation and full output checks run outside timing.
- Output row order has no contract. Test and benchmark validation must compare complete account records without row order.

Plan:
1. Write typed error assertions, then add one Error context enum with operation, line, and path variants. Preserve anyhow source chains and existing domain error types.
2. Verify debug and release tests, formatting, and Clippy. Review and commit only the error change.
3. Add Criterion as a development dependency and use its benchmark harness, throughput reporting, and timing loop. Generate input and verify complete output outside timing.
4. Measure both in-memory workloads and refresh CLI measurements. Update README results and commands from recorded measurements.
5. Verify checks, review the benchmark change, update task records, and create the second commit.

Error implementation: src/error.rs exports one Error enum for CLI arguments, input paths, CSV operations, amount lines, and output operations. anyhow continues to manage source errors. MoneyError and TransactionError remain unchanged. Tests check error variants, numeric line positions, MoneyError values, and I/O error kinds. Diagnostic assertions format TransactionError rather than copying its wording.

Verification: all 30 integration tests pass in debug and release. Clippy with warnings as errors and whitespace checks pass. The typed tests first fail because Error does not exist, then pass after implementation. CSV read failures preserve csv::ErrorKind::Io; csv::Error does not expose the I/O error through std::error::Error::source, so the test inspects the CSV variant directly.

Benchmark implementation: Criterion 0.8.2 replaces custom timing, warmup, median calculation, and the rounds environment variable. The harness uses BenchmarkId and Throughput::ElementsAndBytes for both existing workloads, so Criterion reports records/s and input MiB/s together. Each workload validates complete account CSV and empty diagnostics before Criterion measurement. The timing loop includes pipeline allocation and cleanup, with input generation outside timing. TX_BENCH_ROWS remains available; Criterion arguments control samples and measurement time.

Benchmark checks: default-size Criterion test mode validates both workloads. All 30 debug and release integration tests pass after the dependency change. Formatting, Clippy with warnings as errors, and whitespace checks pass.

Measurements: the final Criterion run uses 30 samples per workload, with 60 deposit iterations and 90 mixed iterations. Mean times are 170.37 ms for 1,000,000 deposits and 152.55 ms for 1,002,048 mixed records. Throughput is 5.87 million records/s and 6.57 million records/s. README includes native Criterion 95% confidence intervals and input MiB/s. The report stores Criterion metadata, estimates, and raw samples in benchmark.json; record_criterion.py collects them without custom timing or statistical analysis.

CLI measurements: refresh all six one-million/four-million-row cases with three runs each and complete output validation. Unique deposits use about 77.3 MiB and 302.4 MiB peak memory. Withdrawal and repeated dispute/resolve inputs use about 2.2 MiB. throughput.json contains all samples.

Final review: README values match the stored Criterion estimates and CLI medians exactly at display precision. Raw Criterion sample means match the recorded estimates. The documented filter and sampling arguments pass Criterion test mode. Existing runtime dependency versions and checksums remain unchanged; the benchmark commit changes no production source or integration tests.

Open work: none.
