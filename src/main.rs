// TODO: implement Deref for the wrapper types

#[repr(transparent)]
struct ClientId(u16);

#[repr(transparent)]
struct TxId(u32);

#[repr(transparent)]
struct Money<T>(T);

enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

struct Transaction<T> {
    ty: TxType,
    client_id: ClientId,
    id: TxId,
    value: Money<T>,
}

struct ClientInfo<T> {
    id: ClientId,
    total: Money<T>,
    held: Money<T>,
    locked: bool,
}

fn process_txs(in: ...) -> HashMap<ClientId, ClientInfo> {
    // TODO: accept a generic stream producing Transaction records. Reuse allocated memory wherever possible.
    // Store in a hashmap updating the transaction status.
    // Valid state transitions: deposit -> dispute -> { resolve | chargeback }.
    // Withdrawals cannot be disputed.
    // Invalid states (log to stderr): dispute/resolve/chargeback of a non-existing or withdrawal transaction; resolve/chargeback of a non-disputed transaction
    // Collect into the hashmap of the account balances.
}

fn main() {
    // TODO: use `csv` and `serde` to ingest a csv file provided as a first argument to this CLI binary (type,client,txid,amount) and
    // convert in into a stream of [`Transaction`]s. Feed it into `process_txs`. Get the resulting hashmap and
    // dump its contents to stdout, serialized to csv: client,available,held,total,locked. available=total-held; locked is "true" of "false" without quotes.
    //
    // ADR: Money are going to be i64 for this implementation.
    // Negative values are to hold negative balances ("deposit-withdraw-dispute deposit" case).
    // We're storing money values with 4-digit precision (make a constant for that). Never use floating point when converting money values.
    // i64::MAX ≈ 10^18
    // That's enough for any sane currency.
    // USD total issuance in 1/100s of cent ≈ 240×10^14
    // BTC total issuance in satoshi ≈ 2×10^15
    // For non-so-sane currencies, u128 may be needed, hence the generic money type.
}
