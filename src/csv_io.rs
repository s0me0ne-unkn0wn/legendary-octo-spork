use std::collections::HashMap;
use std::io::{self, BufWriter, Read, Write};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    ClientId, ClientInfo, Engine, Error, IO_BUFFER_CAPACITY, Money, RECORD_BUFFER_SIZE,
    RECORD_MAX_FIELDS, Transaction, TransactionError, TxId, TxType,
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
struct CsvTransaction<'record> {
    #[serde(rename = "type")]
    ty: CsvTxType,
    client: ClientId,
    tx: TxId,
    #[serde(borrow)]
    amount: Option<&'record str>,
}

struct RetryReader<I>(I);

impl<I: Read> Read for RetryReader<I> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.0.read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }
}

fn process_txs(
    input: impl Read,
    diagnostics: &mut impl Write,
) -> Result<HashMap<ClientId, ClientInfo>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .buffer_capacity(IO_BUFFER_CAPACITY)
        .from_reader(RetryReader(input));
    let headers = reader.byte_headers().context(Error::ReadCsv)?.clone();
    let columns: [&[u8]; 4] = [b"type", b"client", b"tx", b"amount"];
    if headers.len() != columns.len()
        || columns
            .iter()
            .any(|column| headers.iter().filter(|header| header == column).count() != 1)
    {
        bail!(Error::InvalidHeader);
    }
    let mut engine = Engine::default();
    let mut record = csv::ByteRecord::with_capacity(RECORD_BUFFER_SIZE, RECORD_MAX_FIELDS);
    while reader
        .read_byte_record(&mut record)
        .context(Error::ReadCsv)?
    {
        let line = record.position().map_or(0, csv::Position::line);
        if record.len() > headers.len() {
            bail!(Error::ExtraFields { line });
        }
        let row: CsvTransaction<'_> = record.deserialize(Some(&headers)).context(Error::ReadCsv)?;
        let amount = || {
            row.amount
                .context(Error::MissingAmount { line })?
                .parse::<Money>()
                .context(Error::InvalidAmount { line })
        };
        let ty = match row.ty {
            CsvTxType::Deposit => TxType::Deposit(amount()?),
            CsvTxType::Withdrawal => TxType::Withdrawal(amount()?),
            CsvTxType::Dispute => TxType::Dispute,
            CsvTxType::Resolve => TxType::Resolve,
            CsvTxType::Chargeback => TxType::Chargeback,
        };
        let transaction = Transaction {
            ty,
            client_id: row.client,
            id: row.tx,
        };
        if let Err(error) = engine.apply(transaction) {
            writeln!(
                diagnostics,
                "client {}, transaction {}: {error}",
                *transaction.client_id, *transaction.id
            )
            .context(Error::WriteDiagnostics)?;
        }
    }
    Ok(engine.into_clients())
}

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

pub fn write_clients(clients: &HashMap<ClientId, ClientInfo>, output: impl Write) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .buffer_capacity(IO_BUFFER_CAPACITY)
        .from_writer(output);
    writer
        .write_record(["client", "available", "held", "total", "locked"])
        .context(Error::WriteAccounts)?;
    for (client_id, client) in clients.iter() {
        writer
            .serialize(CsvClient {
                client: *client_id,
                available: client
                    .total
                    .checked_sub(client.on_hold)
                    .ok_or(TransactionError::Overflow)?,
                on_hold: client.on_hold,
                total: client.total,
                lock: client.lock,
            })
            .context(Error::WriteAccounts)?;
    }
    writer.flush().context(Error::WriteAccounts)
}

pub fn process_csv(input: impl Read, output: impl Write, diagnostics: impl Write) -> Result<()> {
    let mut diagnostics = BufWriter::with_capacity(IO_BUFFER_CAPACITY, diagnostics);
    let clients = process_txs(input, &mut diagnostics);
    diagnostics.flush().context(Error::WriteDiagnostics)?;
    let clients = clients?;
    write_clients(&clients, output)
}
