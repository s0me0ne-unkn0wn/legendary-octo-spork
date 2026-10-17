# CSV fixtures and unordered output

Status: account assertions pass in debug; fixture readability and final checks remain.

Definition of Done: fix tests for unordered account output without changing production code; present input and expected accounts as readable CSV tables; preserve failure and boundary cases; update other checks that assume row order; verify and create local commits.

The user removed account sorting and a temporary output allocation in commit b3657dc and requested test fixes plus readable fixtures.

Initial facts:
- The working tree starts clean. All source changes from b3657dc belong to the user.
- A baseline cargo test run fails four multi-client CLI checks solely because account row order differs.
- The benchmark and measurement script also compare complete output byte order.
- CLI fixtures encode many rows with newline escapes and omit column labels from expected balances.
- CSV input supports surrounding whitespace. Indented, aligned fixtures can pass directly to the parser without preprocessing.

Plan:
1. Compare headers and complete CSV record lists while ignoring account row order. Preserve duplicate rows and exact field values. Reuse the assertion in CLI, stream, and benchmark checks; update the measurement script and stale sorting documentation.
2. Write aligned multiline CSV fixtures in cli.rs and streams.rs. Include expected account headers and label malformed cases. Preserve field counts, empty amounts, trailing newlines, and diagnostic line positions.
3. Run debug and release tests, formatting, Clippy, the benchmark, and a small measurement script check. Confirm no production code changes.
4. Review the diff and fixture data, update task records, and create small local commits.

Open work: plan steps.

Output checks: one shared assertion compares exact headers, parses complete records, sorts only record lists in test code, and compares full lists including duplicate counts. The benchmark uses it after timing. The measurement script compares the header and sorted data lines. README now states unspecified output order and linear processing time. All 31 debug integration tests pass before fixture conversion. Production code remains unchanged.
