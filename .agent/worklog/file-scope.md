# File processing scope

Status: complete. Tests, documentation, checks, and final review pass. This record accompanies the local commit.

Definition of Done: remove TCP tests and socket-specific support code; retain the TCP documentation example as a possible future use; preserve file processing and file I/O checks; suggest further simplifications; verify and create a local commit.

The user requested this cleanup and clarified that the product ships as a text file processor. TCP support is an example of possible future use, with no current requirement.

Initial facts:
- Socket code exists only in one integration test and the documentation example. The test contains all listener, thread, timeout, clone, and shutdown operations.
- Generic Read and Write traits support the file processor, in-memory benchmarks, and error checks.
- Partial I/O and interrupted reads also apply to files. Their tests and RetryReader do not depend on TCP.
- Production CSV processing does not call process_txs. Only tests and docs reference that separate iterator API.

Assumptions: remove code that exists only for socket testing. Retain ordinary file I/O checks and generic Read/Write parameters. Suggest removal of the unused transaction iterator API separately.

Plan:
1. Remove the TCP integration test and network/time imports.
2. Document the file-only product scope, retain the TCP example, and remove the claim of TCP test coverage.
3. Run debug and release tests, formatting, Clippy, and whitespace checks. Review the diff and commit.

Changes: remove the TCP test and its network/time imports, including all listener, server thread, timeout, clone, and shutdown code. Retain the TCP example in README as a possible future use. Remove the claim of loopback test coverage and the extra network snapshot instructions. Document the current file processing scope.

Verification: all 37 integration tests pass in debug and release. Formatting, Clippy with warnings as errors, and whitespace checks pass. A source/test/benchmark search finds no socket or TCP operations; the remaining TCP references exist only in README. Production code remains unchanged.

Recommendation: remove process_txs and its iterator-specific tests in a separate cleanup. The file processor does not use this API; it duplicates the CSV loop's transaction error reporting and diagnostic buffering. Preserve Read/Write parameters and partial/interrupted I/O checks because they also apply to files.

Final review: the diff contains only the TCP test removal, documentation changes, and task records. No code comments or unrelated changes. No open work.
