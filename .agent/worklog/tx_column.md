# CSV transaction column

Status: complete. CSV input requires tx. Parser fields, fixtures, benchmarks, generators, documentation, and task memory use the same column.

Definition of Done: require tx in the input header, update parser fields, fixtures, benchmarks, generators, and documentation, verify the change, and create a local commit. The user approved this scope.

Plan: update existing fixtures first and check the failure; update the parser and other schema references; run checks and commit.

Known facts: the parser validates exact column names and uses Serde field names. Rename the CSV field directly. Transaction IDs in the engine retain their existing type and behavior.

Verification:
- The updated deposit fixture fails before the parser change and passes afterward.
- All 37 integration tests pass in debug and release profiles.
- Formatting and Clippy checks pass.
- Both benchmark workloads pass output validation with TX_BENCH_ROWS=4096 and TX_BENCH_ROUNDS=1.
- Review confirms consistent schema references and no transaction behavior changes.

Open work: none. The local commit contains the complete rename.
