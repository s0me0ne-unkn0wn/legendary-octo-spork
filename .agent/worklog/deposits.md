# Deposit history and control records

Status: complete. Behavior, docs, tests, measurements, and final review pass. This record accompanies the local commit.

Definition of Done: withdrawals on missing accounts return an error without creating accounts; history contains only deposits and no duplicate ID checks; control records use the deposit's full amount and do not parse an amount field; update tests and docs, verify, and create local commits.

The user requested these changes and approved the Definition of Done.

Initial causes:
- A shared entry operation creates accounts for withdrawals.
- The history map stores withdrawal markers solely for duplicate detection and a separate error message.
- The CSV adapter requires four fields and deserializes all nonempty amount fields as Money.
- Transaction control variants already contain no amount.

Assumptions: deposit IDs are unique in valid input. Remove duplicate ID validation. The CSV header still includes amount; control rows can omit a trailing amount field, and any supplied control amount has no effect.

Plan:
1. Add failing behavior checks for missing accounts, deposit-only history, optional control amounts, and amount parse errors.
2. Use separate account lookups for deposits and withdrawals, retain only deposit data, and parse amounts only for monetary records.
3. Update docs and benchmarks, verify tests and performance, review code, and commit.

Changes: separate account creation from withdrawal lookup, store only deposit amount and dispute state, remove duplicate checks and withdrawal markers, borrow raw CSV amounts and parse them only for deposits and withdrawals, and allow control records with three fields.

Verification: updated CLI checks fail before the fix. All 38 integration tests pass in debug and release, including direct engine checks for missing accounts and control records over TCP. Formatting, Clippy with warnings as errors, and release benchmarks pass. The benchmark verifies all account output and empty diagnostics.

Measurements: in-memory CSV processes 4.90 million deposit rows per second and 6.26 million mixed rows per second. CLI withdrawal streams use 2.2 MiB for one million rows and 2.3 MiB for four million rows. Deposit history uses 77.4 MiB for one million rows and 302.3 MiB for four million rows. README and measurement JSON files contain current results.

Final review: only deposits create accounts or history entries. Control records reference deposit amounts; malformed monetary records still return parse errors. Generic stream APIs and checked arithmetic remain intact. No code comments or unrelated changes. No open work.
