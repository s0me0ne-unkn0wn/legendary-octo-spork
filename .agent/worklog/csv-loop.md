# CSV processing loop

Status: complete. Behavior, docs, tests, throughput checks, and final review pass. This record accompanies the local commit.

Definition of Done: replace CsvTransactions with a direct read_byte_record loop; preserve parsing, diagnostics, generic streams, and process_txs; update tests and docs; verify behavior and throughput; create a local commit.

The user approved the recommended loop and requested implementation. No further approval is necessary.

Initial facts:
- CsvTransactions reuses a ByteRecord and borrows amount text.
- process_txs buffers diagnostics and flushes them before propagating source errors.
- Two tests use CsvTransactions directly; other CSV tests use process_csv.
- The public Engine and process_txs APIs cover incremental records and transaction iterators without CSV.

Assumptions: remove the CsvTransactions public API as part of removing its wrapper. Preserve the other public APIs and all CSV transaction rules.

Plan:
1. Replace tests of the CSV iterator with engine and pipeline behavior checks. Verify the checks against the current code and measure baseline throughput.
2. Process CSV through a direct read_byte_record loop. Preserve the reusable byte record, borrowed amount field, header and field validation, error causes, and diagnostic flush failures.
3. Remove the iterator export and docs. Run debug and release tests, formatting, Clippy, and the release benchmark.
4. Review and simplify the diff, update task notes, and create a local commit.

Changes: replace CsvTransactions with a private read_clients function containing a direct read_byte_record loop. process_csv buffers and flushes diagnostics before propagating read or parse errors and writing account output. Remove the iterator export and docs. Convert iterator tests to engine and pipeline checks; cover prior diagnostics and diagnostic write or flush failures before a CSV error.

Verification: updated stream checks pass before the refactor. All 38 integration tests pass after the refactor in debug and release, including TCP, partial I/O, interrupted reads, transaction iterators, and error propagation. Formatting, Clippy with warnings as errors, and whitespace checks pass. The release benchmark verifies complete account output and empty diagnostics for every run.

Throughput: one warmup and five measurement runs per workload. Before the change, one million deposits take 0.197025 seconds (5,075,502 rows/second); 1,002,048 mixed records take 0.155542 seconds (6,442,299 rows/second). After the change, deposits take 0.198150 seconds (5,046,685 rows/second); mixed records take 0.146319 seconds (6,848,395 rows/second). The measurements show no material slowdown. README and benchmark.json contain current figures.

Final review: the CSV loop reuses one byte record and borrows amount text. Header checks, control records, typed error causes, and diagnostic flush failures remain correct. Generic stream and transaction iterator APIs remain available. Production code decreases by 27 lines. No code comments or unrelated changes. No open work.
