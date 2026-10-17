# Repository and task rules

- Follow AGENTS.md. Write no code comments. Use readable identifiers and direct technical language.
- Maintain .agent/STATUS.md as a symlink into .agent/worklog/.
- Create small local commits. Never push.
- Use git -c safe.directory=/home/claude/legendary-octo-spork for repository commands if ownership checks require it.
- No Git user identity exists in repository or global config. Use per-command author settings for commits if necessary.
- The requested work implements src/main.rs transaction processing and generic streams, including throughput verification.
- The public library exports process_csv, CsvTransactions, process_txs, Engine, and write_clients. Input and output use standard blocking Read and Write traits.
- Amounts use exact four-place integer arithmetic through MoneyValue. The library supports i64 and i128.
- Tests cover CLI behavior, domain state transitions, amount limits, memory streams, TCP, partial I/O, interrupted reads, and I/O errors. Both debug and release profiles pass 38 integration tests.
- benches/throughput.rs validates complete CSV output. Task measurements exist in .agent/worklog/benchmark.json and .agent/worklog/throughput.json.
- README.md documents API examples, transaction behavior, memory growth, benchmark results, and commands.
