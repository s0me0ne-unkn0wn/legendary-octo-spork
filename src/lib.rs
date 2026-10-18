pub mod engine;
pub mod error;
pub mod types;

mod csv_io;

pub use csv_io::{process_csv, write_clients};

const RECORD_BUFFER_SIZE: usize = 128;
const RECORD_MAX_FIELDS: usize = 4;
const IO_BUFFER_CAPACITY: usize = 64 * 1024;
const MONEY_PRECISION: u32 = 4;
const MONEY_SCALE: i64 = 10_i64.pow(MONEY_PRECISION);
