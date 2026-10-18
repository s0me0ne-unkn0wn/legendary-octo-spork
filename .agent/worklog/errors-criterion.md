# Error enum and Criterion

Status: the error task passes verification and is ready for its commit. The benchmark task remains open. The working tree starts clean at cdd250e.

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

Open work: commit the error task, migrate benchmarks to Criterion, measure performance, update docs, verify, and create the second commit.
