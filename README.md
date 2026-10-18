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
