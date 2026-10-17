use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::iter::FusedIterator;

use serde::{Deserialize, Serialize};

use crate::{
    ClientId, ClientInfo, IO_BUFFER_CAPACITY, Money, ProcessError, Transaction, TxId, TxType,
    process_txs,
};

#[derive(Debug)]
pub enum InputError {
    Csv(csv::Error),
    Headers,
    MissingAmount { line: u64 },
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => write!(formatter, "failed to read CSV: {error}"),
            Self::Headers => formatter.write_str(
                "invalid CSV header: require type,client,txid,amount exactly once in any order",
            ),
            Self::MissingAmount { line } => write!(
                formatter,
                "missing amount at CSV line {line}: deposits and withdrawals require an amount"
            ),
        }
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum CsvTxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    ty: CsvTxType,
    client: ClientId,
    txid: TxId,
    amount: Option<Money>,
}

struct RetryReader<Input>(Input);

impl<Input: Read> Read for RetryReader<Input> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.0.read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }
}

pub struct CsvTransactions<Input> {
    reader: csv::Reader<RetryReader<Input>>,
    headers: csv::ByteRecord,
    record: csv::ByteRecord,
    stop: bool,
}

impl<Input: Read> CsvTransactions<Input> {
    pub fn new(input: Input) -> Result<Self, InputError> {
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .buffer_capacity(IO_BUFFER_CAPACITY)
            .from_reader(RetryReader(input));
        let headers = reader.byte_headers().map_err(InputError::Csv)?.clone();
        let columns: [&[u8]; 4] = [b"type", b"client", b"txid", b"amount"];
        if headers.len() != columns.len()
            || columns
                .iter()
                .any(|column| headers.iter().filter(|header| header == column).count() != 1)
        {
            return Err(InputError::Headers);
        }
        Ok(Self {
            reader,
            headers,
            record: csv::ByteRecord::with_capacity(128, 4),
            stop: false,
        })
    }

    fn read_transaction(&mut self) -> Result<Option<Transaction>, InputError> {
        if !self
            .reader
            .read_byte_record(&mut self.record)
            .map_err(InputError::Csv)?
        {
            return Ok(None);
        }
        let record: CsvTransaction = self
            .record
            .deserialize(Some(&self.headers))
            .map_err(InputError::Csv)?;
        let amount = || {
            record.amount.ok_or_else(|| InputError::MissingAmount {
                line: self.record.position().map_or(0, csv::Position::line),
            })
        };
        let ty = match record.ty {
            CsvTxType::Deposit => TxType::Deposit(amount()?),
            CsvTxType::Withdrawal => TxType::Withdrawal(amount()?),
            CsvTxType::Dispute => TxType::Dispute,
            CsvTxType::Resolve => TxType::Resolve,
            CsvTxType::Chargeback => TxType::Chargeback,
        };
        Ok(Some(Transaction {
            ty,
            client_id: record.client,
            id: record.txid,
        }))
    }
}

impl<Input: Read> Iterator for CsvTransactions<Input> {
    type Item = Result<Transaction, InputError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }
        match self.read_transaction() {
            Ok(Some(transaction)) => Some(Ok(transaction)),
            Ok(None) => {
                self.stop = true;
                None
            }
            Err(error) => {
                self.stop = true;
                Some(Err(error))
            }
        }
    }
}

impl<Input: Read> FusedIterator for CsvTransactions<Input> {}

#[derive(Serialize)]
struct CsvClient {
    client: ClientId,
    available: Money,
    #[serde(rename = "held")]
    on_hold: Money,
    total: Money,
    #[serde(rename = "locked")]
    lock: bool,
}

pub fn write_clients<Output: Write>(
    clients: &HashMap<ClientId, ClientInfo>,
    output: Output,
) -> Result<(), CsvError> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .buffer_capacity(IO_BUFFER_CAPACITY)
        .from_writer(output);
    writer
        .write_record(["client", "available", "held", "total", "locked"])
        .map_err(CsvError::Output)?;
    let mut clients: Vec<_> = clients.values().collect();
    clients.sort_unstable_by_key(|client| client.id());
    for client in clients {
        writer
            .serialize(CsvClient {
                client: client.id(),
                available: client.available(),
                on_hold: client.on_hold(),
                total: client.total(),
                lock: client.lock(),
            })
            .map_err(CsvError::Output)?;
    }
    writer
        .flush()
        .map_err(|error| CsvError::Output(error.into()))
}

#[derive(Debug)]
pub enum CsvError {
    Process(ProcessError<InputError>),
    Output(csv::Error),
}

impl fmt::Display for CsvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process(error) => error.fmt(formatter),
            Self::Output(error) => write!(formatter, "failed to write account CSV: {error}"),
        }
    }
}

impl std::error::Error for CsvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Process(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

pub fn process_csv<Input: Read, Output: Write, Diagnostics: Write>(
    input: Input,
    output: Output,
    diagnostics: Diagnostics,
) -> Result<(), CsvError> {
    let transactions = CsvTransactions::new(input)
        .map_err(|error| CsvError::Process(ProcessError::Input(error)))?;
    let clients = process_txs(transactions, diagnostics).map_err(CsvError::Process)?;
    write_clients(&clients, output)
}
