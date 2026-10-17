# Client account fields

Status: complete. Behavior, tests, docs, checks, throughput, and final review pass. This record accompanies the local commit.

Definition of Done: derive Default for ClientInfo; use pub(crate) fields; remove its constructor, getters, and update method; perform balance calculations and assignments explicitly in the engine; preserve overflow checks and unchanged state on errors; update callers, tests, and docs; verify and create a local commit.

The user requested this change. No further approval is necessary.

Initial facts:
- ClientInfo stores a client ID that also exists as its HashMap key.
- Its update method validates available balance and assigns total and on_hold together, including fields that an operation does not change.
- CSV output and one integration test use its getters. Exported snapshot signatures refer to the ClientInfo type.
- Money does not implement Default yet.

Assumptions: preserve the existing snapshot signatures while making ClientInfo fields crate-visible. Remove the duplicate ID and use map keys for output. Add Default to Money so account values can derive Default and account creation can use or_default.

Plan:
1. Test engine results through account CSV and add a chargeback case near the numeric limit. Verify behavior before implementation.
2. Remove ClientInfo's impl, derive Default, expose fields within the crate, and replace combined updates with checked calculations and direct assignments per operation.
3. Update CSV serialization and docs. Run debug and release tests, formatting, Clippy, and the release benchmark.
4. Review and simplify the diff, update task records, and create a local commit.

Changes: ClientInfo derives Default and contains only total, on_hold, and lock as pub(crate) fields. Remove its entire impl and duplicate client ID. Money also derives Default. Deposits use or_default. Each operation calculates balances, checks for overflow, and assigns only the fields it changes. Dispute flags and account locks change only after successful validation. CSV serialization accesses fields directly and uses map keys for sorting and client output.

Verification: updated integration checks pass before implementation. All 36 integration tests pass after implementation in debug and release, including a chargeback at the numeric limit with another dispute, negative available balances, arithmetic overflow with unchanged state, and account ID output. Formatting, Clippy with warnings as errors, and whitespace checks pass. No ClientInfo methods or calls to them remain.

Throughput: the release benchmark validates full output and empty diagnostics in every run. One warmup and five measurement runs report 4,965,747 deposit rows/second and 6,524,834 mixed rows/second. README and benchmark.json contain current figures.

Final review: existing snapshot signatures remain intact. Engine operations validate calculations before mutations, and serialization calculates available funds from balances that fit the numeric range. No code comments or unrelated changes. No open work.
