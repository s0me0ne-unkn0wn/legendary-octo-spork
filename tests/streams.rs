use std::convert::Infallible;
use std::io::{self, Cursor, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;

use legendary_octo_spork::{
    ClientId, Engine, Money, Transaction, TransactionError, TxId, TxType, process_csv, process_txs,
    write_clients,
};

#[test]
fn memory_streams_support_the_csv_pipeline() {
    let input = Cursor::new(b"type,client,tx,amount\ndeposit,1,1,1.2345\n");
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
fn transaction_iterators_accept_records_without_csv() {
    let transactions = [
        Transaction {
            client_id: ClientId(7),
            id: TxId(8),
            ty: TxType::Deposit(Money(12_345)),
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
    .expect("failed to process transaction iterator");
    let mut output = Vec::new();
    write_clients(&clients, &mut output).expect("failed to write clients");
    assert_eq!(
        output,
        b"client,available,held,total,locked\n7,1.2344,0.0000,1.2344,false\n"
    );
}

#[test]
fn incremental_processing_preserves_dispute_state() {
    let mut engine = Engine::default();
    for (id, ty) in [
        (1, TxType::Deposit(Money(20_000))),
        (1, TxType::Dispute),
        (1, TxType::Resolve),
        (2, TxType::Withdrawal(Money(10_000))),
    ] {
        engine
            .apply(Transaction {
                client_id: ClientId(1),
                id: TxId(id),
                ty,
            })
            .expect("failed to apply transaction");
    }
    assert_eq!(engine.clients()[&ClientId(1)].available(), Money(10_000));
}

#[test]
fn withdrawals_on_missing_accounts_return_errors_without_clients() {
    let mut engine = Engine::default();
    for amount in [0, 10_000] {
        let transaction = Transaction {
            client_id: ClientId(1),
            id: TxId(1),
            ty: TxType::Withdrawal(Money(amount)),
        };
        assert_eq!(
            engine.apply(transaction),
            Err(TransactionError::InsufficientFunds)
        );
        assert!(engine.clients().is_empty());
    }
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
        inner: b"type,client,tx,amount\ndeposit,1,1,1.0001\n".as_slice(),
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
            inner: b"type,client,tx,amount\ndeposit,1,1,1.0001\n".as_slice(),
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
            b"type,client,tx,amount\n".as_slice(),
            FaultWriter { flush_only },
            io::sink(),
        );
        let error = result.expect_err("output failure did not propagate");
        assert!(error.to_string().contains("failed to write account CSV"));
        assert_eq!(
            error.root_cause().to_string(),
            if flush_only {
                "flush failure"
            } else {
                "write failure"
            }
        );
    }
}

#[test]
fn diagnostic_write_and_flush_errors_do_not_report_success() {
    for input in [
        "type,client,tx,amount\ndispute,1,1\n",
        "type,client,tx,amount\ndispute,1,1\ndeposit,1,2,0.00001\n",
    ] {
        for flush_only in [false, true] {
            let mut output = Vec::new();
            let result = process_csv(input.as_bytes(), &mut output, FaultWriter { flush_only });
            let error = result.expect_err("diagnostic failure did not propagate");
            assert!(error.to_string().contains("failed to write diagnostics"));
            assert_eq!(
                error.root_cause().to_string(),
                if flush_only {
                    "flush failure"
                } else {
                    "write failure"
                }
            );
            assert!(output.is_empty());
        }
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
    assert!(output.is_empty());
    let error = result.expect_err("input failure did not propagate");
    assert!(error.to_string().contains("failed to read CSV"));
    assert_eq!(error.root_cause().to_string(), "read failure");
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
    assert!(error.to_string().contains("failed to read transaction"));
    assert_eq!(
        error
            .downcast_ref::<io::Error>()
            .expect("missing transaction source error")
            .kind(),
        io::ErrorKind::Other
    );
    assert_eq!(error.root_cause().to_string(), "transaction source failure");
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
        .write_all(b"type,client,tx,amount\ndeposit,1,1,1.2345\ndispute,1,1\nresolve,1,1\n")
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
    let input = b"type,client,tx,amount\ndeposit,1,1,1\n"
        .as_slice()
        .chain(FaultReader);
    let mut output = Vec::new();
    let result = process_csv(input, &mut output, io::sink());
    assert!(result.is_err());
    assert!(output.is_empty());
}

#[test]
fn csv_errors_preserve_prior_diagnostics_and_stop_processing() {
    let input = b"type,client,tx,amount\nwithdrawal,1,1,1\ndeposit,1,2,0.00001\nwithdrawal,2,3,1\n";
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let error = process_csv(input.as_slice(), &mut output, &mut diagnostics)
        .expect_err("invalid CSV did not stop processing");
    assert!(output.is_empty());
    assert_eq!(
        diagnostics,
        b"client 1, transaction 1: insufficient available funds\n"
    );
    assert_eq!(
        error.root_cause().to_string(),
        "amount exceeds four decimal places"
    );
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
