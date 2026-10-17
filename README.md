# legendary-octo-spork

Process account transactions from CSV with exact integer arithmetic and four decimal places.

```sh
cargo run --release -- input.csv > accounts.csv
```

The CLI accepts exactly one file path. It writes account CSV to stdout and transaction errors to stderr. Invalid CSV or an I/O failure produces a nonzero exit status. The processor reads the complete input before it writes account output.

Input requires the columns `type,client,txid,amount` exactly once, in any order. The parser trims surrounding whitespace. Every row must contain four fields; control records can leave `amount` empty.

```csv
type,client,txid,amount
deposit,1,1,2.0001
withdrawal,1,2,0.5000
dispute,1,1,
resolve,1,1,
deposit,2,3,1
```

This input produces:

```csv
client,available,held,total,locked
1,1.5001,0.0000,1.5001,false
2,1.0000,0.0000,1.0000,false
```

Output uses ascending client IDs and exactly four decimal places. Client IDs use `u16`; transaction IDs use `u32`. Both types accept zero. Amount syntax requires integer digits and optionally a decimal point with one to four fractional digits. The processor rejects excess precision, exponent notation, and amounts outside the numeric range. It never rounds or converts amounts through floating point.

| Operation | Requirements | Account changes |
| --- | --- | --- |
| Deposit | New transaction ID; nonnegative amount | Increase total and available |
| Withdrawal | New transaction ID; nonnegative amount; sufficient available funds | Decrease total and available |
| Dispute | The same client's deposit without an active dispute | Increase funds on hold; decrease available |
| Resolve | The same client's deposit with an active dispute | Decrease funds on hold; increase available |
| Chargeback | The same client's deposit with an active dispute | Decrease total and funds on hold; lock the account |

A resolve permits another dispute on the same deposit. Withdrawals do not support dispute, resolve, or chargeback. Disputes can make available funds negative after a withdrawal. A chargeback can make total funds negative and prevents all later activity on the account, including changes to other disputes.

Transaction IDs have global scope. The engine rejects duplicate deposits and withdrawals, references to another client's transaction, invalid state changes, negative amounts, and arithmetic overflow. These errors leave balances and transaction status unchanged; the processor reports each error and continues. Only successful deposits and withdrawals reserve new transaction IDs. The first monetary operation with a new transaction ID and a nonnegative amount creates a client account, including a withdrawal with insufficient funds. Invalid control records do not create accounts. Control records always use the deposit amount; a nonempty amount field still requires valid decimal syntax.

The library separates transaction processing from CSV and stream access:

| API | Use |
| --- | --- |
| `process_csv(input, output, diagnostics)` | CSV pipeline over any `Read` input and `Write` outputs, using `i64` amounts |
| `CsvTransactions::new(input)` | Fallible transaction iterator over any `Read` input; reuse one CSV byte record |
| `process_txs(transactions, diagnostics)` | Consume any iterator of `Result<Transaction, Error>` and return the account map |
| `Engine::default()` and `apply(transaction)` | Process records incrementally while preserving account and transaction state |
| `write_clients(&clients, output)` | Serialize an account map to any `Write` output |

For memory streams:

```rust
use legendary_octo_spork::process_csv;

let input = b"type,client,txid,amount\ndeposit,1,1,1.2345\n";
let mut output = Vec::new();
let mut diagnostics = Vec::new();
process_csv(input.as_slice(), &mut output, &mut diagnostics)?;
```

For a blocking TCP connection:

```rust
use std::error::Error;
use std::net::TcpStream;
use legendary_octo_spork::process_csv;

fn process_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let input = stream.try_clone()?;
    process_csv(input, &mut stream, std::io::stderr().lock())?;
    Ok(())
}
```

The peer must finish its input, for example with `shutdown(Shutdown::Write)`, before reading the account response. The integration suite checks this exchange over a real loopback TCP connection. For connections without an end of input, feed records to `Engine::apply` and use `write_clients` when the application requires a snapshot. These APIs use blocking I/O.

`Money(i64)` stores minor units, so `Money(12_345)` represents `1.2345`. Parsing and balance updates use checked `i64` arithmetic. `ClientId`, `TxId`, and `Money` implement `Deref`. Transaction variants require an amount for deposits and withdrawals, and account fields remain private so callers cannot bypass balance checks.

The processor does not collect input records. It reuses CSV record storage and uses 64 KiB buffers for input, output, and diagnostics. The reader retries interrupted reads before CSV parsing. Processing takes expected linear time in the number of records, plus account sorting. Memory grows with client count, unique successful monetary transactions, and the largest CSV record. The engine retains transaction history for duplicate detection, later disputes, and withdrawal validation. Repeated dispute and resolve operations reuse an existing history entry.

Run checks and the throughput benchmark with:

```sh
cargo test --locked
cargo test --release --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo bench --bench throughput --locked
```

The benchmark generates input before timing, performs one warmup, and reports the median of five runs. Every run checks all account rows and requires an empty diagnostic stream. It covers one million unique deposits and about one million mixed records, including all five transaction types. Adjust its size with `TX_BENCH_ROWS` and its repetitions with `TX_BENCH_ROUNDS`.

On 2026-10-17, Rust 1.94.1 on an Intel Core i9-13900K in this Linux container produced:

| In-memory CSV workload | Records | Median time | Records/second | Input MiB/second |
| --- | ---: | ---: | ---: | ---: |
| Unique deposits, 1,024 clients | 1,000,000 | 0.193 s | 5.19 million | 127.65 |
| Deposits, withdrawals, disputes, resolves, and chargebacks | 1,002,048 | 0.155 s | 6.46 million | 144.50 |

Separate CLI measurements use files with one million and four million rows, with three runs per input and complete output validation. Linux `VmHWM` samples track peak process memory after startup:

| CLI workload | Records | Median time | Records/second | Peak memory |
| --- | ---: | ---: | ---: | ---: |
| Unique deposits, 1,024 clients | 1,000,000 | 0.200 s | 4.99 million | 101.3 MiB |
| Unique deposits, 1,024 clients | 4,000,000 | 0.807 s | 4.96 million | 398.2 MiB |
| Repeated dispute/resolve on one deposit | 1,000,000 | 0.115 s | 8.73 million | 2.3 MiB |
| Repeated dispute/resolve on one deposit | 4,000,000 | 0.420 s | 9.53 million | 2.2 MiB |

These measurements describe this environment; the benchmark does not enforce a hardware-dependent timing limit. History growth accounts for the memory increase with unique deposits. Repeated control records do not increase history size.
