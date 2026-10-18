# Repeat 100 million deposits

Status: complete. One fresh process completes the same 100-million-deposit workload and passes complete account validation with empty diagnostics.

Definition of Done: run the existing master release binary once on the same 100-million-deposit input, validate complete account output and empty diagnostics, report elapsed time, throughput, peak process memory, and system memory pressure evidence in chat. Do not change README or create commits.

Plan: verify binary and input identity; record memory availability, swap counters, and memory pressure totals around one fresh CLI run; compare the result with the previous median; verify README bytes remain unchanged.

Facts: master remains at 04d5518. The previous median is 27.326 seconds, 3.66M tx/s, with 5,045.1 MiB maximum RSS. Current available memory is about 83 GiB. The previous task does not contain measurements of memory pressure during its runs, so this repeat cannot establish the cause of those earlier times.

Result: 26.588 seconds, 3.76M tx/s, 99.7 input MiB/s, 5,045.0 MiB peak process RSS. Throughput improves 2.8% over the previous median. Available system memory remains about 83 GiB. Swap-in, swap-out, and both memory pressure totals have zero change during the run. The repeat reproduces the slower throughput without observed memory pressure; it does not establish the cause of the earlier measurements.

Verification: binary and complete input hashes match the previous master benchmark. The process produces all 1,024 expected account rows and empty diagnostics. README bytes match the snapshot before the run. Master, source, tests, Cargo files, and Criterion benchmark have no changes. No staging or commits. Raw data: .agent/worklog/deposits-repeat.json. Open work: none.
