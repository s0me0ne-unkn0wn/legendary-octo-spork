# Transaction processor implementation

Status: task complete. Implementation, review, tests, performance measurements, and documentation are complete. Local commits contain the implementation and benchmark documentation.

Goal: implement the CSV transaction processor with exact amounts, generic stream APIs, state validation, throughput measurements, documentation, and local commits.

Definition of Done: correct account output for valid and invalid transaction sequences; reusable CSV record storage; generic input, output, diagnostic, and transaction iterator APIs; numeric type support beyond i64; integration coverage for sockets, short reads and writes, arithmetic limits, and I/O failures; release throughput measurements; formatting and Clippy checks; local commits with a clean working tree.

The user approved the Definition of Done.

Known facts:
- The scaffold uses type,client,txid,amount as CSV input and client,available,held,total,locked as output.
- Money requires four decimal places and integer arithmetic.
- Only deposits support disputes. Resolve and chargeback require an active dispute.
- The repository starts with a non-compiling main.rs and no tests.
- Initial file creation failed because the repository owner differed from the process user. A later access check reports the current user as owner with write access. No permission changes were necessary.

Decisions:
- Use a library engine, a fallible CSV transaction iterator, generic stream functions, and a small CLI.
- Use exact decimal parsing and checked arithmetic. Support i64 and i128 through a numeric trait.
- Transaction IDs have global scope. Check ownership for control records.
- Reject negative amounts, insufficient funds, duplicate IDs, overflow, and activity after account lock. Report semantic errors and continue.
- Stop on CSV or I/O errors. Write account output only after the input completes.
- Sort account output by client ID.
- Reuse the CSV byte record and buffer diagnostics.

Plan:
1. Add integration tests for CLI behavior, transaction transitions, exact money values, account isolation, malformed CSV, and generic streams.
2. Implement amount types, transaction and account states, checked processing, CSV adapters, and CLI errors.
3. Extend tests for I/O faults, numeric limits, sockets, and large streams.
4. Measure release throughput and memory on valid workloads. Document commands, API use, behavior, and measurements.
5. Review code and wording, run appropriate checks, update notes, and create small local commits. Never push.

Verification:
- Initial cargo test --locked fails on the scaffold parameter syntax and missing HashMap type.
- The first integration test check also fails because the library does not exist yet.
- cargo test --locked: 37 integration tests pass, including TCP streams, partial I/O, failure propagation, state transitions, exact amounts, numeric limits, and a 100,000-record iterator.
- cargo clippy --all-targets --locked -- -D warnings: no warnings before benchmark addition.
- cargo test --release --locked: the same 37 integration tests pass.
- The final release CSV benchmark reports about 5.19 million deposit rows per second and 6.46 million mixed rows per second. Each run checks the full output and diagnostics. Final measurements exist in benchmark.json.
- Separate CLI checks validate one million and four million rows. New deposits use about 101 and 398 MiB; repeated dispute/resolve records use about 2.2 to 2.3 MiB at both sizes. Raw measurements exist in throughput.json.
- Linux child resource accounting includes Python memory before exec. The measurement script samples the process VmHWM after exec to avoid that measurement error.
- A new integration test demonstrates that the CSV dependency stops on Interrupted reads. RetryReader now retries at the Read boundary before CSV parsing.
- Final cargo test --locked and cargo test --release --locked checks: 38 integration tests pass, including the stream interrupt regression test.
- Final cargo fmt --all -- --check: pass.
- Final cargo clippy --all-targets --locked -- -D warnings: no warnings.
- Final cargo bench --bench throughput --locked: both workloads pass output validation.
- Review confirms no new code comments. Required CSV strings use held and locked; internal account fields use on_hold and lock.

Open work: none. Final repository verification checks commit contents, whitespace, and Git status.
