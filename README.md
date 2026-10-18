# Legendary Octo Spork

Legendary Octo Spork is a novel transaction processing engine! It aggregates incoming CSV transaction history into the account balance report, handling deposits, withdrawals, disputes, and chargebacks.

# Usage example

```sh
cargo run -r -- input.csv > accounts.csv
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

The following output is produced:

```csv
client,available,held,total,locked
1,1.5001,0.0000,1.5001,false
2,1.0000,0.0000,1.0000,false
```

# Design bits

* Money values are `i64` and stored as fixed point with 4-digit precision. The limits of `i64` are orders of magnitude higher than the total issuance of U.S. dollars expressed in 1/100s of cents, or the total issuance of Bitcoin expressed in satoshi. Whenever wider limits are needed, the inner type may be easily replaced with `i128` for the price of slower arithmetic.
* Inexecutable transactions (containing wrong data, or of invalid format, or requesting operation outside of the account limits) are logged to `stderr`.
* Deposits are assumed to be in fiat money, and withdrawals are to the blockchain. Thus, only deposits are deemed disputable.
* For that matter, the withdrawals are not memorized by the engine for the sake of memory optimization. The trade-off is that we cannot check for duplicate transaction IDs for withdrawals, but the IDs are guaranteed to be unique anyway.
* Deposits, for which disputes have been resolved, may be disputed again. A chargeback, on the other hand, is final. A chargeback on any deposit locks the account, so at that point we drop the data for all deposits on that account to free memory.
* Charging back a deposit due to a dispute may put an account to a negative total. Writing off losses is up to the financial department, we're purely adding numbers here and not making any financial decisions.
* Currently, a chargeback locks the account from all operations. It's arguable whether deposits should still be allowed to minimize losses.
* A withdrawal without a previous deposit does not create an empty client account. Reasoning: withdrawing from a non-existent account is an inexecutable transaction. Inexecutable transactions must be idempotent for the sake of data consistency.
* CSV processing logic is reusable with any other stream implementing `Read`.
* Lower-level transaction processing logic is generalized to ingest data from arbitrary parsers providing transaction data, therefore, it's reusable as well.

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

Ran on an Intel Core i9-13900K. Each input uses 1,024 clients and exactly 1, 10, or 100 million transaction rows, excluding the CSV header.

Workloads:

- **Deposits:** unique deposits, rotating through all clients.
- **Mixed:** rotating clients, each with a deposit followed by a withdrawal, dispute of that deposit, and resolution. No chargebacks.
- **Chargebacks at EOF:** mixed groups rotating through all clients, then a dispute and chargeback of each client's last deposit. All accounts close at EOF.
- **Half close after batches:** mixed groups in sequential client batches. Half of the clients close immediately after their batch; the other half remain open.
- **All close after batches:** mixed groups in sequential client batches, with each client account closing immediately after its batch.

| Workload | Transactions | Median seconds | Million tx/s | Input MiB/s | Peak RSS MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Deposits | 1,000,000 | 0.204 | 4.90 | 120.7 | 81.3 |
| Mixed | 1,000,000 | 0.164 | 6.11 | 133.6 | 22.0 |
| Chargebacks at EOF | 1,000,000 | 0.172 | 5.81 | 127.0 | 22.1 |
| Half close after batches | 1,000,000 | 0.162 | 6.18 | 135.2 | 12.1 |
| All close after batches | 1,000,000 | 0.157 | 6.35 | 138.9 | 2.5 |
| Deposits | 10,000,000 | 2.006 | 4.98 | 127.4 | 634.2 |
| Mixed | 10,000,000 | 1.532 | 6.53 | 149.1 | 160.2 |
| Chargebacks at EOF | 10,000,000 | 1.647 | 6.07 | 138.6 | 160.2 |
| Half close after batches | 10,000,000 | 1.506 | 6.64 | 151.7 | 82.4 |
| All close after batches | 10,000,000 | 1.450 | 6.90 | 157.5 | 2.6 |
| Deposits | 100,000,000 | 27.326 | 3.66 | 97.0 | 5,045.1 |
| Mixed | 100,000,000 | 16.838 | 5.94 | 141.3 | 1,264.3 |
| Chargebacks at EOF | 100,000,000 | 19.064 | 5.25 | 124.8 | 1,264.3 |
| Half close after batches | 100,000,000 | 15.826 | 6.32 | 150.3 | 631.6 |
| All close after batches | 100,000,000 | 15.057 | 6.64 | 158.0 | 4.7 |

# Future work

Investigate the throughput degradation at 100M deposits: profiling is needed.
