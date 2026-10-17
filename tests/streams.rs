use std::convert::Infallible;
use std::io::{self, Cursor, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;

use legendary_octo_spork::{
    ClientId, CsvTransactions, Engine, Money, Transaction, TxId, TxType, process_csv, process_txs,
    write_clients,
};

#[test]
fn memory_streams_support_the_csv_pipeline() {
    let input = Cursor::new(b"type,client,txid,amount\ndeposit,1,1,1.2345\n");
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    process_csv(input, &mut output, &mut diagnostics).expect("failed to process memory input");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n1,1.2345,0.0000,1.2345,false\n"
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn transaction_iterators_support_i128_amounts() {
    let transactions = [
        Transaction {
            client_id: ClientId(7),
            id: TxId(8),
            ty: TxType::Deposit(Money(100_000_000_000_000_000_000_i128)),
        },
        Transaction {
            client_id: ClientId(7),
            id: TxId(9),
            ty: TxType::Withdrawal(Money(1)),
        },
    ];
    let clients = process_txs(
        transactions.into_iter().map(Ok::<_, Infallible>),
        io::sink(),
    )
    .expect("failed to process i128 transactions");
    let mut output = Vec::new();
    write_clients(&clients, &mut output).expect("failed to write i128 clients");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n7,9999999999999999.9999,0.0000,9999999999999999.9999,false\n"
    );
}

#[test]
fn csv_transactions_support_i128_without_decimal_rounding() {
    let input = b"type,client,txid,amount\ndeposit,1,1,17014118346046923173168730371588410.5727\n";
    let transactions = CsvTransactions::<_, i128>::new(input.as_slice())
        .expect("failed to create i128 transaction source");
    let clients = process_txs(transactions, io::sink()).expect("failed to process i128 CSV");
    assert_eq!(clients[&ClientId(1)].total(), Money(i128::MAX));
}

#[test]
fn incremental_processing_preserves_state_between_inputs() {
    let mut engine = Engine::<i64>::default();
    for input in [
        "type,client,txid,amount\ndeposit,1,1,2\n",
        "type,client,txid,amount\ndispute,1,1,\n",
        "type,client,txid,amount\nresolve,1,1,\nwithdrawal,1,2,1\n",
    ] {
        for transaction in CsvTransactions::<_, i64>::new(input.as_bytes())
            .expect("failed to create transaction source")
        {
            engine
                .apply(transaction.expect("failed to read transaction"))
                .expect("failed to apply transaction");
        }
    }
    assert_eq!(engine.clients()[&ClientId(1)].available(), Money(10_000));
}

struct ShortStream<Stream> {
    inner: Stream,
    limit: usize,
}

impl<Stream: Read> Read for ShortStream<Stream> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = buffer.len().min(self.limit);
        self.inner.read(&mut buffer[..length])
    }
}

impl<Stream: Write> Write for ShortStream<Stream> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.inner.write(&buffer[..buffer.len().min(self.limit)])
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[test]
fn partial_reads_and_writes_preserve_record_boundaries() {
    let input = ShortStream {
        inner: b"type,client,txid,amount\ndeposit,1,1,1.0001\n".as_slice(),
        limit: 1,
    };
    let mut output = Vec::new();
    let writer = ShortStream {
        inner: &mut output,
        limit: 2,
    };
    process_csv(input, writer, io::sink()).expect("failed to process partial stream operations");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n1,1.0001,0.0000,1.0001,false\n"
    );
}

struct InterruptReader<Input> {
    input: Input,
    interrupt: bool,
}

impl<Input: Read> Read for InterruptReader<Input> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.interrupt {
            self.interrupt = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.interrupt = true;
        self.input.read(buffer)
    }
}

#[test]
fn read_interrupts_do_not_discard_partial_transactions() {
    let input = InterruptReader {
        input: ShortStream {
            inner: b"type,client,txid,amount\ndeposit,1,1,1.0001\n".as_slice(),
            limit: 3,
        },
        interrupt: true,
    };
    let mut output = Vec::new();
    process_csv(input, &mut output, io::sink()).expect("failed to retry stream interrupts");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n1,1.0001,0.0000,1.0001,false\n"
    );
}

struct FaultWriter {
    flush_only: bool,
}

impl Write for FaultWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.flush_only {
            Ok(buffer.len())
        } else {
            Err(io::Error::other("write failure"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failure"))
    }
}

#[test]
fn output_write_and_flush_errors_do_not_report_success() {
    for flush_only in [false, true] {
        let result = process_csv(
            b"type,client,txid,amount\n".as_slice(),
            FaultWriter { flush_only },
            io::sink(),
        );
        assert!(result.is_err());
    }
}

#[test]
fn diagnostic_write_and_flush_errors_do_not_report_success() {
    for flush_only in [false, true] {
        let mut output = Vec::new();
        let result = process_csv(
            b"type,client,txid,amount\ndispute,1,1,\n".as_slice(),
            &mut output,
            FaultWriter { flush_only },
        );
        assert!(result.is_err());
        assert!(output.is_empty());
    }
}

struct FaultReader;

impl Read for FaultReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failure"))
    }
}

#[test]
fn input_failures_do_not_produce_account_output() {
    let mut output = Vec::new();
    let result = process_csv(FaultReader, &mut output, io::sink());
    assert!(result.is_err());
    assert!(output.is_empty());
    assert!(
        result
            .expect_err("input failure did not propagate")
            .to_string()
            .contains("read failure")
    );
}

#[test]
fn iterator_failures_stop_processing() {
    let transactions = [
        Ok(Transaction {
            client_id: ClientId(1),
            id: TxId(1),
            ty: TxType::Deposit(Money(10_000_i64)),
        }),
        Err(io::Error::other("transaction source failure")),
    ];
    let error = process_txs(transactions, io::sink())
        .expect_err("transaction source failure did not propagate");
    assert!(error.to_string().contains("transaction source failure"));
}

#[test]
fn tcp_streams_support_the_same_csv_pipeline() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind test listener");
    let address = listener
        .local_addr()
        .expect("failed to read listener address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("failed to accept test connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("failed to set read timeout");
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .expect("failed to set write timeout");
        let input = stream.try_clone().expect("failed to clone test stream");
        process_csv(input, &mut stream, io::sink()).expect("failed to process TCP input");
    });
    let mut stream = TcpStream::connect(address).expect("failed to connect to test listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("failed to set read timeout");
    stream
        .write_all(b"type,client,txid,amount\ndeposit,1,1,1.2345\n")
        .expect("failed to write TCP input");
    stream
        .shutdown(Shutdown::Write)
        .expect("failed to finish TCP input");
    let mut output = Vec::new();
    stream
        .read_to_end(&mut output)
        .expect("failed to read TCP output");
    server.join().expect("test server panic");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n1,1.2345,0.0000,1.2345,false\n"
    );
}

#[test]
fn input_failure_after_a_valid_transaction_stops_account_output() {
    let input = b"type,client,txid,amount\ndeposit,1,1,1\n"
        .as_slice()
        .chain(FaultReader);
    let mut output = Vec::new();
    let result = process_csv(input, &mut output, io::sink());
    assert!(result.is_err());
    assert!(output.is_empty());
}

#[test]
fn csv_iteration_stops_after_a_record_error() {
    let input = b"type,client,txid,amount\ndeposit,1,1,0.00001\ndeposit,1,2,1\n";
    let mut transactions =
        CsvTransactions::<_, i64>::new(input.as_slice()).expect("failed to create CSV source");
    assert!(transactions.next().expect("missing error record").is_err());
    assert!(transactions.next().is_none());
    assert!(transactions.next().is_none());
}

#[test]
fn small_transaction_differences_accumulate_exactly_per_client() {
    let transactions = (0..100_000).map(|id| {
        Ok::<_, Infallible>(Transaction {
            client_id: ClientId((id % 100) as u16),
            id: TxId(id),
            ty: if id / 100 % 2 == 0 {
                TxType::Deposit(Money(10_001_i64))
            } else {
                TxType::Withdrawal(Money(10_000_i64))
            },
        })
    });
    let clients = process_txs(transactions, io::sink()).expect("failed to process large input");
    assert_eq!(clients.len(), 100);
    for client in clients.values() {
        assert_eq!(client.available(), Money(500));
        assert_eq!(client.on_hold(), Money(0));
        assert!(!client.lock());
    }
}
