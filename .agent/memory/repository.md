# Repository and task rules

- Follow AGENTS.md. Write no code comments. Use readable identifiers and direct technical language.
- Maintain .agent/STATUS.md as a symlink into .agent/worklog/.
- Create small local commits. Never push.
- Use git -c safe.directory=/home/claude/legendary-octo-spork for repository commands if ownership checks require it.
- No Git user identity exists in repository or global config. Use per-command author settings for commits if necessary.
- The product ships as a text file processor. TCP is a documentation example of future use, with no TCP tests or socket-specific requirements. Preserve ordinary file I/O checks.
- The public library exports process_csv, Engine, and write_clients. Input and output use standard blocking Read and Write traits. The user requested removal of the iterator-based process_txs API.
- The private CSV processing function uses the name process_txs after the user requested renaming read_clients. Do not export it as a replacement iterator API.
- process_csv uses a direct read_byte_record loop with one reusable byte record and borrowed amount text. The user requested removal of the CsvTransactions wrapper. Flush diagnostics before propagating CSV errors and before account output.
- CSV input uses type,client,tx,amount. The transaction column is tx.
- Control records use the deposit ID and full deposit amount. They can omit a trailing amount field; the parser ignores any control amount.
- Only deposits create accounts and history entries. Withdrawals on missing accounts return InsufficientFunds. The engine assumes unique deposit IDs and performs no duplicate ID validation.
- Amounts use Money(i64) and exact four-place integer arithmetic. The user requested removal of generic money values. Preserve generic input and output streams.
- Pipeline functions return anyhow::Result and preserve operation context and source errors. Format errors with {error:#} for the complete cause chain. MoneyError and TransactionError remain typed.
- Tests cover CLI behavior, domain state transitions, amount limits, memory streams, partial I/O, interrupted reads, and I/O errors. Both debug and release profiles pass 35 integration tests; the per-client arithmetic check uses the CSV pipeline.
- benches/throughput.rs validates complete CSV output. Task measurements exist in .agent/worklog/benchmark.json and .agent/worklog/throughput.json.
- README.md documents API examples, transaction behavior, memory growth, benchmark results, and commands.
