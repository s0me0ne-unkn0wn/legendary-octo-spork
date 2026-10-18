# Legendary Octo Spork

Legendary Octo Spork is a novel transaction processing engine! It aggregates incoming CSV transaction history into the account balance report, taking care of deposites, withdrawals, disputes and chargebacks.

# Usage example

```sh
cargo run --r -- input.csv > accounts.csv
```

Given the input:

```csv
type,client,tx,amount
deposit,1,1,2.0001
withdrawal,1,2,0.5000
dispute,1,1
resolve,1,1
deposit,2,3,1
```

the following output is produced:

```csv
client,available,held,total,locked
1,1.5001,0.0000,1.5001,false
2,1.0000,0.0000,1.0000,false
```

# Design bits

* Money values are `i64` and stored as fixed point with 4-digit precision. The limits of `i64` are orders of magnitute higher than the total issuance of U.S. dollars expressed in 1/100s of cants, or the total issuance of Bitcoin expressed in satoshi. Whenever wider limits are needed, the inner type may be easily replaced with `i128` for the price of slower arithmetics.
* Inexecutable transactions (containing wrong data, or of invalid format, or requesting operation outside of the account limits) are logged to `stderr`.
* Deposits are assumed to be in fiat money, and withdrawals are to blockchain. Thus, only deposits are deemed disputable.
* For that matter, the withdrawals are not memorized by the engine for the sake of memory optimization. A trade-off is we cannot check for duplicate transaction IDs for withdrawals, but the IDs are guaranteed to be uniq anyway.
* Charging back a deposit due to a dispute may put an account to negative total. Writing off losses is up to the financial department, we're purely adding numbers here and not making any financial decisions.
* Currently, a chargeback locks account from all the operations. It's arguable if deposits should be still allowed for the sake of minimizing losses.
* A withdrawal without a previous deposit does not create an empty client account. Reasoning: withdrawing from a non-existent account is an inexecutable transaction. Inexecutable transactions must be idempotent for the sake of data consistency.
* CSV processing logic is reusable with any other stream implementing `Read`.
* Lower level transaction processing logic is generalized to ingest data from arbitrary parsers providing transaction data, therefor it's reusable as well.

# Examples of usage with other stream types

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

# Benchmarks

The [Criterion](https://docs.rs/criterion/0.8.2/criterion/) benchmark covers one million unique deposits and about one million mixed records, including all five transaction types.

On 2026-10-18, Criterion 0.8.2 with Rust 1.94.1 on an Intel Core i9-13900K:

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

## Chargeback history comparison

On 2026-10-18, Rust 1.94.1 on an Intel Core i9-13900K, with 1,024 clients (`1..=1024`).

Two implementation branches:

- `perf/chargeback-records`: remove only the deposit that completes a chargeback.
- `perf/client-deposit-index`: store deposit IDs per client and remove all of that account's deposit history when a chargeback locks it. Account balances and funds on hold remain in the account record.

Each mixed group contains a deposit, withdrawal, dispute, and resolve. Each chargeback has a preceding dispute. The operation counts below include all of these records exactly.

- At EOF: distribute groups across clients, then charge back all 1,024 accounts.
- Half during input: process clients in batches; charge back the first 512 accounts after their batches and leave the remaining accounts open.
- All during input: process clients in batches and charge back each account after its batch.

Each table entry uses three fresh release CLI processes reading the same CSV file and writing account CSV to a file. The variant order alternates between runs. Throughput uses median elapsed time, including startup and file I/O. Peak process memory uses the maximum Linux `VmHWM` sample at 1 ms intervals. Input generation and complete output validation run outside the CLI process; all 54 runs validate every account and empty diagnostics.

| Chargeback pattern | Operations | Peak memory without index | Peak memory with index | Ops/second without index | Ops/second with index |
| --- | ---: | ---: | ---: | ---: | ---: |
| At EOF | 1,000,000 | 21.1 MiB | 21.9 MiB | 6.03 million | 5.66 million |
| Half during input | 1,000,000 | 20.9 MiB | 11.9 MiB | 5.99 million | 6.14 million |
| All during input | 1,000,000 | 21.1 MiB | 2.5 MiB | 5.90 million | 6.26 million |
| At EOF | 10,000,000 | 152.3 MiB | 160.2 MiB | 6.27 million | 5.77 million |
| Half during input | 10,000,000 | 152.3 MiB | 82.2 MiB | 6.12 million | 6.52 million |
| All during input | 10,000,000 | 152.2 MiB | 2.6 MiB | 6.20 million | 6.64 million |
| At EOF | 100,000,000 | 1,202.4 MiB | 1,264.4 MiB | 5.88 million | 5.33 million |
| Half during input | 100,000,000 | 1,202.4 MiB | 631.3 MiB | 5.99 million | 6.39 million |
| All during input | 100,000,000 | 1,202.2 MiB | 4.5 MiB | 6.01 million | 6.79 million |

At 100 million operations, the index adds 5.2% peak memory and reduces throughput 9.5% with chargebacks at EOF. With half the accounts closing during input, it reduces peak memory 47.5%. With every account closing during input, peak memory decreases from 1,202.2 MiB to 4.5 MiB.

The index benefits the measured workloads where chargebacks remove deposit histories before later deposits arrive. The EOF and all-batched patterns both have 1,024 chargebacks, so timing and history size per account determine the saving. Hash-table capacity grows in steps and remains available for reuse after removal. Open accounts still require their full deposit history. The all-batched case retains one client batch at a time, and that batch grows with the total operation count in this comparison.
