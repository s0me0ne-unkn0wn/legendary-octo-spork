# Pipeline errors

Status: complete. Pipeline functions use anyhow; domain errors remain typed.

Definition of Done: remove CsvError, ProcessError, and InputError; preserve MoneyError, TransactionError, stream support, diagnostic context, and source errors; update documentation and tests; verify and create a local commit.

The user approved the Definition of Done.

Known facts: the current pipeline uses three wrapper enums with custom Display and Error implementations. Existing tests cover CSV failures, source failures, output and diagnostic write and flush failures, and empty account output after input failure.

Decision: use anyhow Result and Context. Boxed errors need manual context wrappers to preserve the same operation and source information. Keep typed domain errors and the generic transaction iterator.

Plan:
1. Strengthen existing error checks for operation context and source preservation.
2. Replace wrapper enums with anyhow, simplify the CLI, and update documentation.
3. Review line reduction, run tests, formatting, and Clippy, update notes, and commit.

Changes: remove all three pipeline enums and their formatting and source implementations. Add context at pipeline operations. Return anyhow::Result from main for standard error reporting and exit handling. Production source contains 99 fewer lines. Update README.md for anyhow results and source error conversion.

Verification:
- Updated tests initially fail against the previous error types.
- cargo test --locked: 37 integration tests pass.
- cargo test --release --locked: 37 integration tests pass.
- cargo clippy --all-targets --locked -- -D warnings: no warnings.
- cargo fmt --all -- --check: pass.
- Tests verify CSV line numbers, amount errors, output and diagnostic write and flush errors, and source error type preservation.
- Review confirms no CsvError, ProcessError, or InputError in production code. Money and transaction errors and state transitions remain unchanged.

Open work: none. The local commit includes source, dependency, test, documentation, and task note changes.
