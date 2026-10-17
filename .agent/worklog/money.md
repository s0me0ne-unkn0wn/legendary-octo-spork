# Fixed money type

Status: complete. Money uses i64 directly; numeric type parameters no longer exist. Generic input and output streams remain available.

Goal and Definition of Done: use Money(i64), remove MoneyValue and numeric type parameters, preserve generic stream APIs and transaction behavior, update tests and documentation, verify the changes, and create a local commit.

The user requested this narrower numeric API and approved the Definition of Done.

Known facts: the existing implementation and 38 integration tests pass. The numeric abstraction affects money, transaction types, the engine, CSV records, and documentation. Input and output generic parameters remain necessary for the stream APIs.

Plan:
1. Update existing tests for the concrete money API and remove i128-only coverage.
2. Replace generic money types and simplify direct i64 arithmetic and Serde implementations.
3. Update documentation and memory, check the complete diff, run tests, formatting, and Clippy, and commit.

Changes: remove the numeric trait, macro, and marker fields; use direct checked i64 arithmetic; derive Engine's default implementation; update API examples and tests. Remove two i128-only tests and retain iterator coverage with i64 amounts.

Verification:
- The updated money test initially fails to compile against the generic API.
- cargo test --locked: 36 integration tests pass.
- cargo test --release --locked: 36 integration tests pass.
- cargo clippy --all-targets --locked -- -D warnings: no warnings, including the benchmark target.
- Review confirms no numeric type parameters or i128 support in source, tests, benchmarks, or README.md.

Open work: none. The commit includes the source, tests, documentation, and task notes.
