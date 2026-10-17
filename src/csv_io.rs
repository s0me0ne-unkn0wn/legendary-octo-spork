use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::iter::FusedIterator;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    ClientId, ClientInfo, IO_BUFFER_CAPACITY, Money, Transaction, TxId, TxType, process_txs,
};

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
    pub fn new(input: Input) -> Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .buffer_capacity(IO_BUFFER_CAPACITY)
            .from_reader(RetryReader(input));
        let headers = reader.byte_headers().context("failed to read CSV")?.clone();
        let columns: [&[u8]; 4] = [b"type", b"client", b"txid", b"amount"];
        if headers.len() != columns.len()
            || columns
                .iter()
                .any(|column| headers.iter().filter(|header| header == column).count() != 1)
        {
            bail!("invalid CSV header: require type,client,txid,amount exactly once in any order");
        }
        Ok(Self {
            reader,
            headers,
            record: csv::ByteRecord::with_capacity(128, 4),
            stop: false,
        })
    }

    fn read_transaction(&mut self) -> Result<Option<Transaction>> {
        if !self.reader.read_byte_record(&mut self.record)? {
            return Ok(None);
        }
        let record: CsvTransaction = self.record.deserialize(Some(&self.headers))?;
        let amount = || {
            record.amount.with_context(|| {
                format!(
                    "missing amount at CSV line {}: deposits and withdrawals require an amount",
                    self.record.position().map_or(0, csv::Position::line)
                )
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
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }
        match self.read_transaction().context("failed to read CSV") {
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
) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .buffer_capacity(IO_BUFFER_CAPACITY)
        .from_writer(output);
    writer
        .write_record(["client", "available", "held", "total", "locked"])
        .context("failed to write account CSV")?;
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
            .context("failed to write account CSV")?;
    }
    writer.flush().context("failed to write account CSV")
}

pub fn process_csv<Input: Read, Output: Write, Diagnostics: Write>(
    input: Input,
    output: Output,
    diagnostics: Diagnostics,
) -> Result<()> {
    let transactions = CsvTransactions::new(input)?;
    let clients = process_txs(transactions, diagnostics)?;
    write_clients(&clients, output)
}
