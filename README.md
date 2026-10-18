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

Output row order is unspecified. Amounts use exactly four decimal places. Client IDs use `u16`; transaction IDs use `u32`. Both types accept zero. Deposit and withdrawal amounts require integer digits and optionally a decimal point with one to four fractional digits. The processor rejects excess precision, exponent notation, and amounts outside the numeric range. It never rounds or converts amounts through floating point.

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

Pipeline functions return `anyhow::Result`. The `Error` enum in `src/error.rs` identifies the operation, CSV line, or input path. Inspect it with `error.downcast_ref::<Error>()`; amount errors also support `error.downcast_ref::<MoneyError>()`. Source errors remain available through the error chain. Format errors with `{error:#}` to include the complete chain. `MoneyError` and `TransactionError` remain separate types.

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

The processor does not collect input records. Its CSV loop reuses one byte record and borrows amount text from that record. It uses 64 KiB buffers for input, output, and diagnostics. The reader retries interrupted reads before CSV parsing. Processing takes expected linear time in the number of records. Memory grows with client count, successful deposits, and the largest CSV record. The engine stores deposit amounts, owners, and dispute flags. Withdrawals create no history entries, and repeated dispute and resolve operations update the same deposit.

Run checks and the throughput benchmark with:

```sh
cargo test --locked
cargo test --release --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo bench --bench throughput --locked
```

The [Criterion](https://docs.rs/criterion/0.8.2/criterion/) benchmark covers one million unique deposits and about one million mixed records, including all five transaction types. It generates input and checks complete account output and empty diagnostics before measurement. Timed iterations include CSV parsing, processing, account and history allocation, CSV output, and cleanup. Each workload uses a three-second warmup, 30 samples, and a ten-second target measurement time. Criterion reports records/second, input MiB/second, and 95% confidence intervals, and saves results under `target/criterion/`.

Adjust input size with `TX_BENCH_ROWS`. Use Criterion arguments to change sampling or select a workload, for example:

```sh
cargo bench --bench throughput --locked -- deposits --sample-size 50 --measurement-time 20
```

On 2026-10-18, Criterion 0.8.2 with Rust 1.94.1 on an Intel Core i9-13900K in this Linux container produced these mean estimates:

| In-memory CSV workload | Records | Mean time | 95% time interval | Records/second | Input MiB/second |
| --- | ---: | ---: | ---: | ---: | ---: |
| Unique deposits, 1,024 clients | 1,000,000 | 170.37 ms | 170.22–170.53 ms | 5.87 million | 144.45 |
| Deposits, withdrawals, disputes, resolves, and chargebacks | 1,002,048 | 152.55 ms | 152.10–153.02 ms | 6.57 million | 143.70 |

Separate CLI measurements use files with one million and four million rows, with three runs per input and complete output validation. Linux `VmHWM` samples track peak process memory after startup:

| CLI workload | Records | Median time | Records/second | Peak memory |
| --- | ---: | ---: | ---: | ---: |
| Unique deposits, 1,024 clients | 1,000,000 | 0.209 s | 4.78 million | 77.3 MiB |
| Unique deposits, 1,024 clients | 4,000,000 | 0.838 s | 4.77 million | 302.4 MiB |
| Withdrawals after one initial deposit | 1,000,000 | 0.160 s | 6.26 million | 2.2 MiB |
| Withdrawals after one initial deposit | 4,000,000 | 0.609 s | 6.56 million | 2.2 MiB |
| Repeated dispute/resolve on one deposit | 1,000,000 | 0.115 s | 8.69 million | 2.2 MiB |
| Repeated dispute/resolve on one deposit | 4,000,000 | 0.406 s | 9.86 million | 2.2 MiB |

The withdrawal record counts include the initial deposit. These measurements describe this environment; the benchmark does not enforce a hardware-dependent timing limit. History growth accounts for the memory increase with unique deposits. Withdrawals and repeated control records do not increase history size.
