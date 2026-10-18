# Money arithmetic

Status: complete. This record accompanies the Money arithmetic commit. The working tree starts clean at b7c71e3.

Definition of Done: implement Money::checked_add and Money::checked_sub, use them in engine balance calculations and CSV output, preserve error variants and atomic account changes, verify signed limits and transaction behavior, update docs, and create a local commit.

Facts:
- Engine arithmetic extracts i64 values, checks addition and subtraction, and wraps results in Money.
- Account output subtracts the inner i64 values directly. Engine updates already validate that difference before assigning balances.
- Money parsing and formatting work on its internal representation. Those conversions can continue to use i64 directly.
- Existing CLI tests cover insufficient funds, arithmetic overflow, negative available amounts, and chargebacks at numeric limits.
- The benchmark constructs expected Money values from integer counts without accessing Money internals.

Plan:
1. Add one arithmetic API test for exact minor units and signed boundaries, then verify that it fails before implementation.
2. Add two methods with Money operands and Option<Money> results. Remove inner-value access and redundant Money construction from the engine. Use checked subtraction and TransactionError::Overflow for account output.
3. Update the Money API documentation, run debug and release tests, formatting, Clippy, and Criterion workload validation.
4. Review the diff, update task records, and create one local commit.

Implementation: Money::checked_add and Money::checked_sub accept and return Money through Option. The engine uses Money values throughout its arithmetic, comparisons, and assignments. Account output uses checked_sub and propagates TransactionError::Overflow rather than subtracting raw integers. Balance checks still run before account or dispute state changes. Parsing and formatting continue to manage the internal i64 representation.

Verification: the new arithmetic test first fails to compile because the previous Deref methods accept and return i64. It passes after implementation and checks exact minor units, signed overflow, zero at both limits, and subtraction involving i64::MIN. All 31 integration tests pass in debug and release. Formatting, Clippy with warnings as errors, whitespace checks, and Criterion test mode pass. Both default benchmark workloads validate complete account output and empty diagnostics. Review confirms that the engine and account output no longer access Money's inner value.

Open work: none.
