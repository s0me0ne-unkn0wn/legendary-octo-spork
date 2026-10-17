mod csv_io;
mod engine;
mod money;
mod types;

pub use csv_io::{process_csv, write_clients};
pub use engine::{ClientInfo, Engine, TransactionError, process_txs};
pub use money::{MONEY_PRECISION, MONEY_SCALE, Money, MoneyError};
pub use types::{ClientId, Transaction, TxId, TxType};

const IO_BUFFER_CAPACITY: usize = 64 * 1024;
