use std::collections::HashMap;
use std::io::{self, BufWriter, Read, Write};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{ClientId, ClientInfo, Engine, IO_BUFFER_CAPACITY, Money, RECORD_BUFFER_SIZE, RECORD_MAX_FIELDS, Transaction, TxId, TxType};

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

fn read_clients<Input: Read>(
    input: Input,
    diagnostics: &mut impl Write,
) -> Result<HashMap<ClientId, ClientInfo>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .buffer_capacity(IO_BUFFER_CAPACITY)
        .from_reader(RetryReader(input));
    let headers = reader.byte_headers().context("failed to read CSV")?.clone();
    let columns: [&[u8]; 4] = [b"type", b"client", b"tx", b"amount"];
    if headers.len() != columns.len()
        || columns
            .iter()
            .any(|column| headers.iter().filter(|header| header == column).count() != 1)
    {
        bail!("invalid CSV header: require type,client,tx,amount exactly once in any order");
    }
    let mut engine = Engine::default();
    let mut record = csv::ByteRecord::with_capacity(RECORD_BUFFER_SIZE, RECORD_MAX_FIELDS);
    while reader
        .read_byte_record(&mut record)
        .context("failed to read CSV")?
    {
        let line = record.position().map_or(0, csv::Position::line);
        if record.len() > headers.len() {
            bail!("too many fields at CSV line {line}");
        }
        let row: CsvTransaction<'_> = record
            .deserialize(Some(&headers))
            .context("failed to read CSV")?;
        let amount = || {
            row.amount
                .with_context(|| format!("missing amount at CSV line {line}: deposits and withdrawals require an amount"))?
                .parse::<Money>()
                .with_context(|| format!("invalid amount at CSV line {line}"))
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
            .context("failed to write diagnostics")?;
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
    let mut diagnostics = BufWriter::with_capacity(IO_BUFFER_CAPACITY, diagnostics);
    let clients = read_clients(input, &mut diagnostics);
    diagnostics.flush().context("failed to write diagnostics")?;
    let clients = clients?;
    write_clients(&clients, output)
}
