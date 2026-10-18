mod common;

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use common::assert_accounts;

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

struct InputFile {
    path: PathBuf,
}

impl InputFile {
    fn new(input: &str) -> Self {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-inputs");
        std::fs::create_dir_all(&directory).expect("failed to create test directory");
        let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!("{}-{sequence}.csv", std::process::id()));
        std::fs::write(&path, input).expect("failed to create test input");
        Self { path }
    }
}

impl Drop for InputFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).expect("failed to remove test input");
    }
}

fn process(input: &str) -> Output {
    let file = InputFile::new(input);
    Command::new(env!("CARGO_BIN_EXE_legendary-octo-spork"))
        .arg(&file.path)
        .output()
        .expect("failed to execute CLI")
}

fn balances(input: &str, expected_accounts: &str, error_count: usize) {
    let output = process(input);
    let diagnostics = String::from_utf8(output.stderr).expect("invalid diagnostic encoding");
    assert!(output.status.success(), "{diagnostics}");
    assert_accounts(&output.stdout, expected_accounts.as_bytes());
    assert_eq!(diagnostics.lines().count(), error_count, "{diagnostics}");
}

#[test]
fn deposits_and_withdrawals_update_each_client() {
    balances(
        "type      , client, tx, amount
         deposit   , 2     , 1 , 2
         deposit   , 1     , 2 , 1.0001
         withdrawal, 1     , 3 , 0.5
         deposit   , 1     , 4 , 0.0001\n",
        "client, available, held  , total , locked
         1     , 0.5002   , 0.0000, 0.5002, false
         2     , 2.0000   , 0.0000, 2.0000, false\n",
        0,
    );
}

#[test]
fn withdrawals_cannot_use_money_on_hold() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 2
         deposit   , 1     , 2 , 1
         dispute   , 1     , 1 ,
         withdrawal, 1     , 3 , 1.0001
         withdrawal, 1     , 4 , 1\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 2.0000, 2.0000, false\n",
        1,
    );
}

#[test]
fn resolve_allows_a_later_dispute() {
    balances(
        "type   , client, tx, amount
         deposit, 1     , 1 , 2.125
         dispute, 1     , 1
         resolve, 1     , 1
         dispute, 1     , 1\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 2.1250, 2.1250, false\n",
        0,
    );
}

#[test]
fn disputes_can_make_available_balances_negative() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 1
         withdrawal, 1     , 2 , 0.9999
         dispute   , 1     , 1\n",
        "client, available, held  , total , locked
         1     , -0.9999  , 1.0000, 0.0001, false\n",
        0,
    );
}

#[test]
fn chargeback_can_make_total_negative_and_stops_account_activity() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 1
         withdrawal, 1     , 2 , 1
         dispute   , 1     , 1 ,
         chargeback, 1     , 1 ,
         deposit   , 1     , 3 , 5
         withdrawal, 1     , 4 , 0
         resolve   , 1     , 1 ,
         dispute   , 1     , 1 ,
         chargeback, 1     , 1 ,
         deposit   , 2     , 5 , 3\n",
        "client, available, held  , total  , locked
         1     , -1.0000  , 0.0000, -1.0000, true
         2     , 3.0000   , 0.0000, 3.0000 , false\n",
        5,
    );
}

#[test]
fn invalid_state_changes_do_not_change_balances() {
    balances(
        "type      , client, tx, amount
         dispute   , 1     , 99,
         resolve   , 1     , 99,
         chargeback, 1     , 99,
         deposit   , 1     , 1 , 3
         withdrawal, 1     , 2 , 1
         dispute   , 1     , 2 ,
         resolve   , 1     , 2 ,
         chargeback, 1     , 2 ,
         resolve   , 1     , 1 ,
         chargeback, 1     , 1 ,
         dispute   , 1     , 1 ,
         dispute   , 1     , 1 ,
         resolve   , 1     , 1 ,
         resolve   , 1     , 1 ,\n",
        "client, available, held  , total , locked
         1     , 2.0000   , 0.0000, 2.0000, false\n",
        10,
    );
}

#[test]
fn other_clients_cannot_change_a_deposit() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 2
         deposit   , 2     , 2 , 3
         dispute   , 2     , 1 ,
         dispute   , 1     , 1 ,
         resolve   , 2     , 1 ,
         chargeback, 2     , 1 ,\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 2.0000, 2.0000, false
         2     , 3.0000   , 0.0000, 3.0000, false\n",
        3,
    );
}

#[test]
fn withdrawal_ids_do_not_create_deposit_history() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 5
         withdrawal, 1     , 2 , 1
         withdrawal, 1     , 2 , 1
         dispute   , 1     , 2 ,
         deposit   , 2     , 2 , 2
         dispute   , 2     , 2 ,
         chargeback, 2     , 2 ,\n",
        "client, available, held  , total , locked
         1     , 3.0000   , 0.0000, 3.0000, false
         2     , 0.0000   , 0.0000, 0.0000, true\n",
        1,
    );
}

#[test]
fn invalid_amounts_and_overflow_do_not_change_balances() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 922337203685477.5807
         deposit   , 1     , 2 , 0.0001
         withdrawal, 1     , 3 , -1
         deposit   , 1     , 4 , -1
         withdrawal, 1     , 5 , 0.0001
         deposit   , 1     , 2 , 0.0001\n",
        "client, available           , held  , total               , locked
         1     , 922337203685477.5807, 0.0000, 922337203685477.5807, false\n",
        3,
    );
}

#[test]
fn dispute_overflow_preserves_balances_and_transaction_status() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 922337203685477.5807
         withdrawal, 1     , 2 , 922337203685477.5807
         dispute   , 1     , 1 ,
         deposit   , 1     , 3 , 922337203685477.5807
         dispute   , 1     , 3 ,
         resolve   , 1     , 1 ,
         dispute   , 1     , 3 ,
         resolve   , 1     , 3 ,\n",
        "client, available           , held  , total               , locked
         1     , 922337203685477.5807, 0.0000, 922337203685477.5807, false\n",
        1,
    );
}

#[test]
fn withdrawals_before_deposits_do_not_change_later_balances() {
    balances(
        "type      , client, tx, amount
         withdrawal, 1     , 7 , 2
         deposit   , 1     , 1 , 2
         withdrawal, 1     , 7 , 2
         dispute   , 1     , 7 ,\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 0.0000, 0.0000, false\n",
        2,
    );
}

#[test]
fn a_chargeback_preserves_other_dispute_amounts() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 2
         deposit   , 1     , 2 , 3
         dispute   , 1     , 1
         dispute   , 1     , 2
         chargeback, 1     , 1
         resolve   , 1     , 2\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 3.0000, 3.0000, true\n",
        1,
    );
}

#[test]
fn chargeback_preserves_available_balance_at_the_numeric_limit() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 922337203685477.5806
         deposit   , 1     , 2 , 0.0001
         withdrawal, 1     , 3 , 922337203685477.5807
         dispute   , 1     , 1
         dispute   , 1     , 2
         chargeback, 1     , 1
         resolve   , 1     , 2\n",
        "client, available            , held  , total                , locked
         1     , -922337203685477.5807, 0.0001, -922337203685477.5806, true\n",
        1,
    );
}

#[test]
fn control_records_cannot_replace_the_deposit_amount() {
    balances(
        "type      , client, tx, amount
         deposit   , 1     , 1 , 1
         dispute   , 1     , 1 , not-money
         resolve   , 1     , 1 , 0.00001
         dispute   , 1     , 1 , -999
         chargeback, 1     , 1 , 1e99\n",
        "client, available, held  , total , locked
         1     , 0.0000   , 0.0000, 0.0000, true\n",
        0,
    );
}

#[test]
fn withdrawals_without_an_account_do_not_create_one() {
    balances(
        "type      , client, tx, amount
         withdrawal, 1     , 1 , 1\n",
        "client,available,held,total,locked\n",
        1,
    );
}

#[test]
fn header_only_input_still_produces_output_headers() {
    balances(
        "type,client,tx,amount\n",
        "client,available,held,total,locked\n",
        0,
    );
}

#[test]
fn malformed_csv_stops_processing_without_account_output() {
    for (case, input) in [
        ("empty input", ""),
        (
            "missing amount column",
            "type   , client, tx
             deposit, 1     , 1\n",
        ),
        (
            "duplicate client column",
            "type   , client, client, amount
             deposit, 1     , 1     , 1\n",
        ),
        (
            "excess precision after a valid row",
            "type   , client, tx, amount
             deposit, 1     , 1 , 1
             deposit, 1     , 2 , 0.00001\n",
        ),
        (
            "empty deposit amount",
            "type   , client, tx, amount
             deposit, 1     , 1 ,\n",
        ),
        (
            "empty withdrawal amount",
            "type      , client, tx, amount
             withdrawal, 1     , 1 ,\n",
        ),
        (
            "missing deposit amount field",
            "type   , client, tx, amount
             deposit, 1     , 1\n",
        ),
        (
            "missing withdrawal amount field",
            "type      , client, tx, amount
             withdrawal, 1     , 1\n",
        ),
        (
            "missing dispute transaction ID",
            "type   , client, tx, amount
             dispute, 1\n",
        ),
        (
            "exponent amount",
            "type   , client, tx, amount
             deposit, 1     , 1 , 1e2\n",
        ),
        (
            "nondecimal amount",
            "type   , client, tx, amount
             deposit, 1     , 1 , NaN\n",
        ),
        (
            "amount outside i64 range",
            "type   , client, tx, amount
             deposit, 1     , 1 , 922337203685477.5808\n",
        ),
        (
            "client ID outside u16 range",
            "type   , client, tx, amount
             deposit, 65536 , 1 , 1\n",
        ),
        (
            "transaction ID outside u32 range",
            "type   , client, tx        , amount
             deposit, 1     , 4294967296, 1\n",
        ),
        (
            "unknown transaction type",
            "type   , client, tx, amount
             unknown, 1     , 1 , 1\n",
        ),
        (
            "extra deposit field",
            "type   , client, tx, amount
             deposit, 1     , 1 , 1     , extra\n",
        ),
        (
            "extra dispute field",
            "type   , client, tx, amount
             dispute, 1     , 1 ,       , extra\n",
        ),
    ] {
        let output = process(input);
        assert!(!output.status.success(), "{case}:\n{input}");
        assert!(output.stdout.is_empty(), "{case}:\n{input}");
        assert!(!output.stderr.is_empty(), "{case}:\n{input}");
    }
}

#[test]
fn csv_errors_identify_the_row_and_amount_problem() {
    for (input, line, problem) in [
        (
            "type   , client, tx, amount
             deposit, 1     , 1 ,\n",
            "line 2",
            "missing amount",
        ),
        (
            "type   , client, tx, amount
             deposit, 1     , 1 , 1
             deposit, 1     , 2 , 0.00001\n",
            "line 3",
            "amount exceeds four decimal places",
        ),
    ] {
        let output = process(input);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostics.contains(line), "{diagnostics}");
        assert!(diagnostics.contains(problem), "{diagnostics}");
    }
}

#[test]
fn cli_requires_exactly_one_input_path() {
    let file = InputFile::new("type,client,tx,amount\n");
    for arguments in [vec![], vec![&file.path, &file.path]] {
        let output = Command::new(env!("CARGO_BIN_EXE_legendary-octo-spork"))
            .args(arguments)
            .output()
            .expect("failed to execute CLI");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("usage:"));
    }
}
