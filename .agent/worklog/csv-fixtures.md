# CSV fixtures and unordered output

Status: complete. Tests, fixture review, formatting, lint, benchmark, measurement checks, and production source comparison pass. This record accompanies the fixture commit.

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

Open work: none.

Output checks: one shared assertion compares exact headers, parses complete records, sorts only record lists in test code, and compares full lists including duplicate counts. The benchmark uses it after timing. The measurement script compares the header and sorted data lines. README now states unspecified output order and linear processing time. All 31 debug integration tests pass before fixture conversion. Production code remains unchanged.

Fixtures: all 18 balance scenarios show input and expected accounts as aligned multiline CSV tables with headers. Malformed input cases have descriptions. Stream inputs and expected accounts use the same format. Fixtures pass directly to the production parser, with no input rewriting helper. A final escaped newline preserves complete last records in stream failure cases.

Data audit: compare the previous and current fixture values after trimming field padding. All 37 CLI input fixtures and 7 stream CSV fixtures preserve field values, field counts, row order, and final newlines. All 18 expected account tables preserve balances, flags, and diagnostic counts. Diagnostic line checks still pass.

Verification: all 31 tests pass in debug and release. Formatting, Clippy with warnings as errors, and whitespace checks pass. The full release benchmark validates every account row and empty diagnostics across all repetitions. The Python measurement tool validates 10,000 rows for deposits, withdrawals, and disputes without modifying historical measurement reports. Production source and dependency files match b3657dc exactly.

Commits: 9306a98 fixes unordered output validation. The fixture commit contains only readable fixtures and task records. No code comments or unrelated changes.
