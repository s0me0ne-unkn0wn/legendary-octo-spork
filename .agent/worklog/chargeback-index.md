# Chargeback history comparison

Status: complete. Both implementation branches pass checks and all 54 benchmark runs validate complete output. This record accompanies the report commit, which precedes the README append. Start from master at 3b16fce. The previous client-count benchmark configuration has been rolled back by the user. Three untracked files from that experiment remain; leave them unchanged.

Definition of Done: implement removal of the deposit that completes a chargeback, create separate branches with and without a client deposit index, preserve balances and locked-account rejection, compare throughput and peak CLI memory at exactly 1, 10, and 100 million mixed operations, append results to README without changing earlier bytes, and leave README uncommitted.

Facts and assumptions:
- Deposits without chargebacks remain disputable after resolve and need history.
- A successful chargeback locks an account. Its complete deposit history then has no further valid use.
- Removing a hash entry permits allocation reuse but does not shrink the hash table. Chargebacks only at EOF cannot reduce the earlier peak.
- Index overhead is one TxId per deposit plus vector capacity and one vector per account.
- The restored benchmark uses 1,024 clients. Use that fixed count to measure deposit growth independently from client growth.
- Keep valid operations and exact row counts. Use deposit, withdrawal, dispute, resolve groups plus dispute/chargeback pairs.
- Compare chargebacks at EOF, sequential account batches with half the accounts locking, and sequential batches with every account locking. These measure late reclamation, partial reclamation, and continuous reclamation.
- Use three fresh release CLI processes per variant and input. Measure throughput from median elapsed time and memory from child VmHWM high-water samples. Generate and validate CSV outside the measured processes.

Plan:
1. Add a failing history-removal test and verify locked-account controls. Implement minimal removal without an index and verify tests, formatting, lint, and release CSV workloads.
2. Create a branch for that implementation. Add a Vec<TxId> to each account on the indexed branch. Register successful deposits and consume that vector to remove all client deposits after a successful chargeback. Verify both branches independently.
3. Build separate release binaries. Generate identical file inputs for each 1M/10M/100M case, run variants sequentially, validate every output account and empty diagnostics, and record samples.
4. Append measured comparisons, methods, and a recommendation to README. Verify the previous bytes and leave README outside all commits.
5. Review branch differences, retain both branches for comparison, and report the useful cases and overhead.

Git note: prior commits failed on loose object directories owned by UID 1000. The current agent uses UID 1001. Inspect access and publish normal Git pack objects if necessary, without changing ownership or permissions.

Implementations:
- perf/chargeback-records at 6e27bd1 removes only the deposit that completes a chargeback. A unit test confirms history removal and preservation of other deposits. Control transactions check the account lock before history lookup, so later operations return AccountLock even after removal. An integration test checks every operation on the closed account, unknown IDs, a removed ID from another client, and repeated disputes on another active account.
- perf/client-deposit-index at aef2648 adds a Vec<TxId> to ClientInfo, registers only successful deposits, and consumes that vector with mem::take when an account locks. It removes every referenced deposit, frees the vector, and preserves other accounts. The unit test checks full client history removal.
- Both branches pass 33 debug/release tests, formatting, Clippy with warnings as errors, and default Criterion workload validation. Existing benches/throughput.rs remains unchanged.

Git: commit_pack.py uses a private temporary object store and index, then writes ordinary Git pack files to the writable .git/objects/pack directory before updating the experiment branch. It changes no permissions or Git configuration and never commits README. The source commits are readable by ordinary Git commands. Master remains at 3b16fce.

Benchmark:
- Frozen release binaries are target/chargeback-benchmark/without-index and with-index. variants.json records branch commits and binary hashes.
- python3 -B .agent/tools/measure_chargebacks.py runs exact 1M, 10M, and 100M CSV rows for late, half, and all patterns. It alternates variant order across three fresh process repetitions, validates every account and empty stderr, and writes .agent/worklog/chargeback-index.json after every sample.
- Late: round-robin clients, all 1,024 chargebacks at EOF. Half: sequential client batches, first 512 clients lock immediately after their batch. All: sequential client batches, all clients lock immediately after their batch. All patterns use 1,024 clients with IDs 1..=1024.
- Each deposit/withdrawal/dispute/resolve group contributes four rows. Final dispute/chargeback pairs count within the requested total, not in addition to it. Input generation verifies actual newline count and computes file hashes without reading entire large inputs into memory.
- A 10,000-row smoke run validates all patterns and both binaries before full measurements. Smoke report is separate.
- At 10M rows: late memory is 152.3 MiB without index versus 160.2 MiB with index; half is 152.3 versus 82.2 MiB; all is 152.2 versus 2.6 MiB. All completed runs validate full output.
- All 100M cases finish. Late: 1,202.4 MiB and 5.88M ops/s without the index, versus 1,264.4 MiB and 5.33M ops/s with it. Half: 1,202.4 MiB and 5.99M ops/s versus 631.3 MiB and 6.39M ops/s. All: 1,202.2 MiB and 6.01M ops/s versus 4.5 MiB and 6.79M ops/s.
- Final audit verifies nine cases, 54 samples, exact operation counts, three samples per variant, medians, memory maxima, binary hashes, all expected account files, empty diagnostics, and matching final output hashes for late/all patterns. Both branch READMEs and the restored Criterion benchmark remain unchanged before the append.

Assessment: the index adds about 5.2% peak memory and reduces throughput about 9.5% in the 100M late case. It reduces memory about 47.5% with half the clients closing during input, and about 99.6% when all clients close in batches. Recommend the index when chargebacks remove account histories before later deposits arrive. The same 1,024 chargebacks at EOF provide no peak-memory reduction. Memory depends on live history, input order, and hash-table capacity steps. The all-batched case retains one client batch at a time; batch size still increases with total volume in this experiment, so the result does not prove constant memory for arbitrary input.

README: its initial bytes exist in target/chargeback-benchmark/README.before. Append results only after measurement verification, preserving the latest user prefix if concurrent edits occur. Leave README unstaged and uncommitted, as explicitly requested. Complete any optional report/tool commits before appending README.

Delivery: retain both experiment branches without changing master. Commit tools and raw reports before appending the prepared comparison to README. Verify the current README prefix byte for byte, leave README unstaged, and create no further commits after the append.

Open work: none after the final append and read-only verification.
