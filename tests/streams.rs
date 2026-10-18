mod common;

use std::io::{self, Read, Write};

use common::assert_accounts;
use legendary_octo_spork::{
    engine::{Engine, TransactionError},
    error::Error,
    process_csv,
    types::ClientId,
    types::{Money, MoneyError, Transaction, TxId, TxType},
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

#[test]
fn account_lock_rejects_activity_after_history_removal() {
    let mut engine = Engine::default();
    for (client, id, ty) in [
        (1, 1, TxType::Deposit(Money(10_000))),
        (2, 2, TxType::Deposit(Money(20_000))),
        (1, 1, TxType::Dispute),
        (1, 1, TxType::Chargeback),
    ] {
        engine
            .apply(Transaction {
                client_id: ClientId(client),
                id: TxId(id),
                ty,
            })
            .expect("failed to process chargeback test");
    }
    for id in [1, 99] {
        for ty in [
            TxType::Deposit(Money(1)),
            TxType::Withdrawal(Money(0)),
            TxType::Dispute,
            TxType::Resolve,
            TxType::Chargeback,
        ] {
            assert_eq!(
                engine.apply(Transaction {
                    client_id: ClientId(1),
                    id: TxId(id),
                    ty,
                }),
                Err(TransactionError::AccountLock)
            );
        }
    }
    assert_eq!(
        engine.apply(Transaction {
            client_id: ClientId(2),
            id: TxId(1),
            ty: TxType::Dispute,
        }),
        Err(TransactionError::UnknownTransaction)
    );
    for ty in [TxType::Dispute, TxType::Resolve, TxType::Dispute] {
        engine
            .apply(Transaction {
                client_id: ClientId(2),
                id: TxId(2),
                ty,
            })
            .expect("chargeback changes another client's deposit");
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
        input: b"type   , client, tx, amount
                 deposit, 1     , 1 , 1.0001\n"
            .as_slice(),
        interrupt: true,
    };
    let mut output = Vec::new();
    process_csv(input, &mut output, io::sink()).expect("failed to retry stream interrupts");
    assert_accounts(
        &output,
        b"client, available, held  , total , locked
          1     , 1.0001   , 0.0000, 1.0001, false\n",
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
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::WriteZero.into())
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
        assert_eq!(error.downcast_ref::<Error>(), Some(&Error::WriteAccounts));
        assert_eq!(
            error
                .root_cause()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            if flush_only {
                io::ErrorKind::WriteZero
            } else {
                io::ErrorKind::BrokenPipe
            }
        );
    }
}

#[test]
fn diagnostic_write_and_flush_errors_do_not_report_success() {
    for input in [
        "type   , client, tx, amount
         dispute, 1     , 1\n",
        "type   , client, tx, amount
         dispute, 1     , 1
         deposit, 1     , 2 , 0.00001\n",
    ] {
        for flush_only in [false, true] {
            let mut output = Vec::new();
            let result = process_csv(input.as_bytes(), &mut output, FaultWriter { flush_only });
            let error = result.expect_err("diagnostic failure did not propagate");
            assert_eq!(
                error.downcast_ref::<Error>(),
                Some(&Error::WriteDiagnostics)
            );
            assert_eq!(
                error
                    .root_cause()
                    .downcast_ref::<io::Error>()
                    .unwrap()
                    .kind(),
                if flush_only {
                    io::ErrorKind::WriteZero
                } else {
                    io::ErrorKind::BrokenPipe
                }
            );
            assert!(output.is_empty());
        }
    }
}

struct FaultReader;

impl Read for FaultReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::ConnectionReset.into())
    }
}

#[test]
fn input_failures_do_not_produce_account_output() {
    for input in [
        "",
        "type   , client, tx, amount
         deposit, 1     , 1 , 1\n",
    ] {
        let mut output = Vec::new();
        let result = process_csv(input.as_bytes().chain(FaultReader), &mut output, io::sink());
        assert!(output.is_empty());
        let error = result.expect_err("input failure did not propagate");
        assert_eq!(error.downcast_ref::<Error>(), Some(&Error::ReadCsv));
        let source = error.downcast_ref::<csv::Error>().unwrap();
        assert!(matches!(
            source.kind(),
            csv::ErrorKind::Io(error) if error.kind() == io::ErrorKind::ConnectionReset
        ));
    }
}

#[test]
fn csv_errors_preserve_prior_diagnostics_and_stop_processing() {
    let input = b"type      , client, tx, amount
                  withdrawal, 1     , 1 , 1
                  deposit   , 1     , 2 , 0.00001
                  withdrawal, 2     , 3 , 1\n";
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let error = process_csv(input.as_slice(), &mut output, &mut diagnostics)
        .expect_err("invalid CSV did not stop processing");
    assert!(output.is_empty());
    assert_eq!(
        diagnostics,
        format!(
            "client 1, transaction 1: {}\n",
            TransactionError::InsufficientFunds
        )
        .as_bytes()
    );
    assert_eq!(
        error.downcast_ref::<Error>(),
        Some(&Error::InvalidAmount { line: 3 })
    );
    assert_eq!(
        error.downcast_ref::<MoneyError>(),
        Some(&MoneyError::Precision)
    );
}

#[test]
fn csv_errors_identify_the_row_and_amount_problem() {
    for (input, context, source) in [
        (
            "type   , client, tx, amount
             deposit, 1     , 1 ,\n",
            Error::MissingAmount { line: 2 },
            None,
        ),
        (
            "type   , client, tx, amount
             deposit, 1     , 1 , 1
             deposit, 1     , 2 , 0.00001\n",
            Error::InvalidAmount { line: 3 },
            Some(MoneyError::Precision),
        ),
        (
            "type   , client, tx, amount
             deposit, 1     , 1 , NaN\n",
            Error::InvalidAmount { line: 2 },
            Some(MoneyError::Syntax),
        ),
        (
            "type   , client, tx, amount
             deposit, 1     , 1 , 922337203685477.5808\n",
            Error::InvalidAmount { line: 2 },
            Some(MoneyError::Overflow),
        ),
    ] {
        let mut output = Vec::new();
        let error = process_csv(input.as_bytes(), &mut output, io::sink())
            .expect_err("invalid amount did not stop processing");
        assert!(output.is_empty());
        assert_eq!(error.downcast_ref::<Error>(), Some(&context));
        assert_eq!(error.downcast_ref::<MoneyError>(), source.as_ref());
    }
}

#[test]
fn csv_structure_errors_identify_the_header_or_record() {
    for (input, context) in [
        (
            "type   , client, client, amount
             deposit, 1     , 1     , 1\n",
            Error::InvalidHeader,
        ),
        (
            "type   , client, tx, amount
             deposit, 1     , 1 , 1     , extra\n",
            Error::ExtraFields { line: 2 },
        ),
    ] {
        let mut output = Vec::new();
        let error = process_csv(input.as_bytes(), &mut output, io::sink())
            .expect_err("invalid CSV structure did not stop processing");
        assert!(output.is_empty());
        assert_eq!(error.downcast_ref::<Error>(), Some(&context));
    }
}

#[test]
fn csv_field_errors_preserve_the_record_position() {
    let input = b"type   , client, tx, amount
                  deposit, 1     , 1 , 1
                  unknown, 1     , 2 , 1\n";
    let mut output = Vec::new();
    let error = process_csv(input.as_slice(), &mut output, io::sink())
        .expect_err("invalid transaction type did not stop processing");
    assert!(output.is_empty());
    assert_eq!(error.downcast_ref::<Error>(), Some(&Error::ReadCsv));
    assert_eq!(
        error
            .downcast_ref::<csv::Error>()
            .unwrap()
            .position()
            .map(csv::Position::line),
        Some(3)
    );
}
