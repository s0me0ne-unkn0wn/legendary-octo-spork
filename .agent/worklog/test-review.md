# Test review

Status: complete. Test review, cleanup, checks, and final review pass. This record accompanies the local commit.

Definition of Done: remove tests that repeat existing coverage or exercise standard library and CSV crate behavior without testing application logic; preserve transaction rules, numeric boundaries, retries, and error propagation; review the changes, run checks, and create a local commit.

The user requested this review and removal, with memory_streams_support_the_csv_pipeline as an example.

Initial facts:
- The memory test repeats the CLI deposit, amount formatting, and output checks.
- The incremental engine check repeats resolve and withdrawal behavior from CLI tests.
- The standalone partial I/O check exercises CSV crate buffering; it does not test the interruption retry branch.
- The 100,000-row arithmetic test repeats exact money and per-client CLI checks. The benchmark already validates full account output for larger workloads.
- The interruption test covers RetryReader; write and flush failures cover explicit diagnostics and output handling.
- The missing-account engine test covers zero withdrawals and a typed error, which the CLI checks do not cover.
- Input failure tests use the same error and output expectations before and after a valid row.

Plan:
1. Remove the four redundant checks and their unused imports and ShortStream helper. Put short read chunks directly in the interruption test helper.
2. Combine both input failure scenarios in one test with the same error and output checks.
3. Run debug and release tests, formatting, Clippy, and whitespace checks. Review unique coverage, update task records, and create a local commit.

Changes: remove memory_streams_support_the_csv_pipeline, incremental_processing_preserves_dispute_state, partial_reads_and_writes_preserve_record_boundaries, and small_transaction_differences_accumulate_exactly_per_client. Remove unused Cursor, formatting, and serialization imports, plus ShortStream and its Read/Write implementations. The interruption helper now returns short chunks directly. Combine header and body read failures in one test with complete cause and empty output checks.

Verification: all 31 remaining integration tests pass in debug and release. Formatting, Clippy with warnings as errors, and whitespace checks pass. All CLI and money tests remain. The remaining stream tests cover zero withdrawals on missing accounts, read retries, output/diagnostic write and flush failures, input errors, and prior diagnostic output on a CSV error.

Final review: the diff changes only tests and task records. Each removal has coverage in CLI, money, or benchmark checks, or tests behavior that the CSV crate implements. No code comments or unrelated changes. No open work.
