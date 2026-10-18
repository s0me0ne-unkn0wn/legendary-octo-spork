# Master benchmark

Status: complete. All 15 cases and 45 samples pass validation. README contains the append and its previous bytes match the initial working snapshot. No commits or staging changes exist for this task. Master is 04d5518. README has user changes before this task. The Git index has no changes. Leave the three untracked files from the previous client-count experiment unchanged.

Definition of Done: measure deposits, mixed transactions without chargebacks, and three chargeback patterns at exactly 1, 10, and 100 million input transactions; validate every output; record three fresh release CLI samples per case; analyze throughput and peak process memory; append results to the current README without changing its previous bytes or creating commits.

Facts and assumptions:
- Master includes the client deposit index. A chargeback removes all deposit history for that account.
- Use 1,024 clients with IDs 1 through 1,024, matching the previous chargeback comparison.
- Deposits use 1.0001 per row. Mixed groups contain a 2.0001 deposit, a 1.0000 withdrawal, a dispute, and a resolve.
- Mixed contains no chargebacks. The three chargeback patterns use the same groups and close all accounts at EOF, half after sequential account batches, or all after sequential account batches.
- Include all dispute and chargeback rows within each requested transaction count. Exclude the CSV header.
- Preserve production code, tests, Cargo files, and the existing Criterion benchmark. Use fresh file-to-file CLI processes for both throughput and memory.
- Use median elapsed time and maximum child VmHWM across three samples. Input generation, hashing, and account validation run outside the measured process.
- The previous measurement tools provide exact decimal formatting, process measurements, complete output validation, and the chargeback generators.
- Available memory is 82 GiB and disk space is 1.4 TiB before input generation. Confirm process limits before the full measurements.

Unknowns to measure: throughput at each volume, peak memory for open deposit histories, and memory reduction from each chargeback order.

Plan:
1. Snapshot the current README and Git state. Build master in release mode, run the existing release tests, and copy the binary to a task directory with commit and binary hashes.
2. Add a small master-only driver that reuses existing measurement and chargeback generation functions. Generate exact deposit and mixed inputs and independent expected account balances.
3. Check all five workloads at 10,000 transactions. Measure all 15 full-size cases sequentially, with three fresh processes each, and save the report after every sample.
4. Audit row counts, samples, account output, diagnostics, binary identity, and summaries. Analyze memory growth, throughput changes, and limits of the input order.
5. Append the methods, table, and analysis to the latest README. Verify the previous contents byte for byte, confirm master and source files have no changes, and leave all results uncommitted.

Preparation complete: the fresh master release build passes all 33 tests. All five 10,000-transaction checks pass full account validation with empty diagnostics. The frozen binary SHA-256 is a5f2386fbbee3aef6c0689d5532f03d3270b79d42f247b5a2a56b02dce4eaa9f, matching the indexed source from the previous comparison. No cgroup memory limit file exists at the checked paths. The working README snapshot contains 5,051 bytes with SHA-256 2f1ca9df5336a010597def96f0dbb9997612c575a906d81e475ab520d0bb1f3c. Full measurements now use the new master-only driver.

Progress: all 30 measurements at 1M and 10M pass output validation. At 10M: deposits 634.2 MiB and 4.98M tx/s; mixed 160.2 MiB and 6.53M tx/s; EOF 160.2 MiB and 6.07M tx/s; half batches 82.4 MiB and 6.64M tx/s; all batches 2.6 MiB and 6.90M tx/s. README still matches the initial working snapshot. The 100M inputs and measurements are in progress.

Progress: 100M deposits, mixed without chargebacks, and EOF chargebacks all pass three full-output checks each. Deposits: 27.326 s median, 3.66M tx/s, 5,045.1 MiB peak. Mixed: 16.838 s, 5.94M tx/s, 1,264.3 MiB. EOF: 19.064 s, 5.25M tx/s, 1,264.3 MiB. The half-closing and all-closing 100M cases remain.

Final audit: audit_master.py checks all 15 input files with streaming SHA-256 and newline counts; independently derives all 1,024 account balances with Decimal; checks complete output and empty diagnostics; verifies all 45 sample counts, medians, throughputs, and peak maxima; matches the nine chargeback input hashes to the previous comparison; and confirms master, binary identity, and unchanged source/tests/Cargo/Criterion files.

Final 100M results: deposits 3.66M tx/s and 5,045.1 MiB; mixed 5.94M tx/s and 1,264.3 MiB; EOF 5.25M tx/s and 1,264.3 MiB; half closing after batches 6.32M tx/s and 631.6 MiB; all closing after batches 6.64M tx/s and 4.7 MiB. Relative to EOF, half and all batches reduce peak memory 50.0% and 99.63%. The largest single account batch contains 24,414 deposits and grows with total input size. No constant-memory claim applies to arbitrary input.

Delivery: the master benchmark section appends to README after its existing 5,051 bytes. The previous bytes remain identical. target/master-benchmark/README.audit.json records hashes and section location. Raw samples are .agent/worklog/master-benchmark.json. The frozen master binary and all exact input/output files exist in target/master-benchmark. Master remains at 04d5518. README remains unstaged and uncommitted; this task creates no commits. Open work: none after the final read-only delivery check.
