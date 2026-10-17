# legendary-octo-spork

Process account transactions from a CSV file with exact integer arithmetic and four decimal places. The CLI supports file processing; network connections are a possible future use of the library API.

```sh
cargo run --release -- input.csv > accounts.csv
```

The CLI accepts exactly one file path. It writes account CSV to stdout and transaction errors to stderr. Invalid CSV or an I/O failure produces a nonzero exit status. The processor reads the complete input before it writes account output.

Input requires the columns `type,client,tx,amount` exactly once, in any order. The parser trims surrounding whitespace. Deposits and withdrawals require an amount. Dispute, resolve, and chargeback records use only `type`, `client`, and `tx`; they can omit a trailing amount field or leave it empty. Any supplied amount on these records has no effect.

```csv
type,client,tx,amount
deposit,1,1,2.0001
withdrawal,1,2,0.5000
dispute,1,1
resolve,1,1
deposit,2,3,1
```

This input produces:

```csv
client,available,held,total,locked
1,1.5001,0.0000,1.5001,false
2,1.0000,0.0000,1.0000,false
```

Output uses ascending client IDs and exactly four decimal places. Client IDs use `u16`; transaction IDs use `u32`. Both types accept zero. Deposit and withdrawal amounts require integer digits and optionally a decimal point with one to four fractional digits. The processor rejects excess precision, exponent notation, and amounts outside the numeric range. It never rounds or converts amounts through floating point.

| Operation | Requirements | Account changes |
| --- | --- | --- |
| Deposit | Nonnegative amount | Create the account if necessary; increase total and available |
| Withdrawal | Existing account; nonnegative amount; sufficient available funds | Decrease total and available |
| Dispute | The same client's deposit without an active dispute | Increase funds on hold; decrease available |
| Resolve | The same client's deposit with an active dispute | Decrease funds on hold; increase available |
| Chargeback | The same client's deposit with an active dispute | Decrease total and funds on hold; lock the account |

A resolve permits another dispute on the same deposit. Withdrawals do not support dispute, resolve, or chargeback. Disputes can make available funds negative after a withdrawal. A chargeback can make total funds negative and prevents all later activity on the account, including changes to other disputes.

Deposit IDs have global scope, and the processor assumes that input deposit IDs are unique. It performs no duplicate ID validation. Only deposits create accounts. A withdrawal on a missing account returns an error without creating an account. The engine also rejects references to another client's deposit, invalid state changes, negative amounts, and arithmetic overflow. These errors leave balances and deposit state unchanged; the processor reports each error and continues. Disputes, resolves, and chargebacks always use the deposit's full amount.

The library separates transaction processing from CSV and stream access:

| API | Use |
| --- | --- |
| `process_csv(input, output, diagnostics)` | CSV pipeline over any `Read` input and `Write` outputs, using `i64` amounts |
| `Engine::default()` and `apply(transaction)` | Process records incrementally while preserving account and transaction state |
| `write_clients(&clients, output)` | Serialize an account map to any `Write` output |

Pipeline functions return `anyhow::Result`. Error context preserves the operation and source error. Format errors with `{error:#}` to include the complete cause chain. `MoneyError` and `TransactionError` remain separate types.

For memory streams:

```rust
use legendary_octo_spork::process_csv;

let input = b"type,client,tx,amount\ndeposit,1,1,1.2345\n";
let mut output = Vec::new();
let mut diagnostics = Vec::new();
process_csv(input.as_slice(), &mut output, &mut diagnostics)?;
```

For possible future use with a blocking TCP connection:

```rust
use std::net::TcpStream;
use legendary_octo_spork::process_csv;

fn process_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let input = stream.try_clone()?;
    process_csv(input, &mut stream, std::io::stderr().lock())
}
```

The peer must finish its input, for example with `shutdown(Shutdown::Write)`, before reading the account response. These APIs use blocking I/O.

`Money(i64)` stores minor units, so `Money(12_345)` represents `1.2345`. Parsing and balance updates use checked `i64` arithmetic. `ClientId`, `TxId`, and `Money` implement `Deref`. Transaction variants require an amount for deposits and withdrawals. Account fields have crate visibility, and the engine validates balance calculations before assigning fields. Client IDs exist as account map keys; output calculates available funds as total minus funds on hold.

The processor does not collect input records. Its CSV loop reuses one byte record and borrows amount text from that record. It uses 64 KiB buffers for input, output, and diagnostics. The reader retries interrupted reads before CSV parsing. Processing takes expected linear time in the number of records, plus account sorting. Memory grows with client count, successful deposits, and the largest CSV record. The engine stores deposit amounts, owners, and dispute flags. Withdrawals create no history entries, and repeated dispute and resolve operations update the same deposit.

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
| Unique deposits, 1,024 clients | 1,000,000 | 0.201 s | 4.97 million | 122.20 |
| Deposits, withdrawals, disputes, resolves, and chargebacks | 1,002,048 | 0.154 s | 6.52 million | 142.73 |

Separate CLI measurements use files with one million and four million rows, with three runs per input and complete output validation. Linux `VmHWM` samples track peak process memory after startup:

| CLI workload | Records | Median time | Records/second | Peak memory |
| --- | ---: | ---: | ---: | ---: |
| Unique deposits, 1,024 clients | 1,000,000 | 0.213 s | 4.70 million | 77.4 MiB |
| Unique deposits, 1,024 clients | 4,000,000 | 0.856 s | 4.67 million | 302.3 MiB |
| Withdrawals after one initial deposit | 1,000,000 | 0.159 s | 6.28 million | 2.2 MiB |
| Withdrawals after one initial deposit | 4,000,000 | 0.605 s | 6.61 million | 2.3 MiB |
| Repeated dispute/resolve on one deposit | 1,000,000 | 0.112 s | 8.92 million | 2.2 MiB |
| Repeated dispute/resolve on one deposit | 4,000,000 | 0.412 s | 9.70 million | 2.3 MiB |

The withdrawal record counts include the initial deposit. These measurements describe this environment; the benchmark does not enforce a hardware-dependent timing limit. History growth accounts for the memory increase with unique deposits. Withdrawals and repeated control records do not increase history size.
