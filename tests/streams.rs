use std::io::{self, Read, Write};

use legendary_octo_spork::{
    ClientId, Engine, Money, Transaction, TransactionError, TxId, TxType, process_csv,
};

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
        let length = buffer.len().min(3);
        self.input.read(&mut buffer[..length])
    }
}

#[test]
fn read_interrupts_do_not_discard_partial_transactions() {
    let input = InterruptReader {
        input: b"type,client,tx,amount\ndeposit,1,1,1.0001\n".as_slice(),
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
    for input in ["", "type,client,tx,amount\ndeposit,1,1,1\n"] {
        let mut output = Vec::new();
        let result = process_csv(input.as_bytes().chain(FaultReader), &mut output, io::sink());
        assert!(output.is_empty());
        let error = result.expect_err("input failure did not propagate");
        assert!(error.to_string().contains("failed to read CSV"));
        assert_eq!(error.root_cause().to_string(), "read failure");
    }
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
