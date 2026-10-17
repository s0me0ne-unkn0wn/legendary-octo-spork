use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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

fn balances(input: &str, account_rows: &str, error_count: usize) {
    let output = process(input);
    let diagnostics = String::from_utf8(output.stderr).expect("invalid diagnostic encoding");
    assert!(output.status.success(), "{diagnostics}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("invalid output encoding"),
        format!("client,available,held,total,locked\n{account_rows}")
    );
    assert_eq!(diagnostics.lines().count(), error_count, "{diagnostics}");
}

#[test]
fn deposits_and_withdrawals_update_each_client() {
    balances(
        "type,client,txid,amount\ndeposit,2,1,2\ndeposit,1,2,1.0001\nwithdrawal,1,3,0.5\ndeposit,1,4,0.0001\n",
        "1,0.5002,0.0000,0.5002,false\n2,2.0000,0.0000,2.0000,false\n",
        0,
    );
}

#[test]
fn withdrawals_cannot_use_money_on_hold() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,2\ndeposit,1,2,1\ndispute,1,1,\nwithdrawal,1,3,1.0001\nwithdrawal,1,4,1\n",
        "1,0.0000,2.0000,2.0000,false\n",
        1,
    );
}

#[test]
fn resolve_allows_a_later_dispute() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,2.125\ndispute,1,1,\nresolve,1,1,\ndispute,1,1,\n",
        "1,0.0000,2.1250,2.1250,false\n",
        0,
    );
}

#[test]
fn disputes_can_make_available_balances_negative() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,1\nwithdrawal,1,2,0.9999\ndispute,1,1,\n",
        "1,-0.9999,1.0000,0.0001,false\n",
        0,
    );
}

#[test]
fn chargeback_can_make_total_negative_and_stops_account_activity() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,1\nwithdrawal,1,2,1\ndispute,1,1,\nchargeback,1,1,\ndeposit,1,3,5\nwithdrawal,1,4,0\nresolve,1,1,\ndispute,1,1,\nchargeback,1,1,\ndeposit,2,5,3\n",
        "1,-1.0000,0.0000,-1.0000,true\n2,3.0000,0.0000,3.0000,false\n",
        5,
    );
}

#[test]
fn invalid_state_changes_do_not_change_balances() {
    balances(
        "type,client,txid,amount\ndispute,1,99,\nresolve,1,99,\nchargeback,1,99,\ndeposit,1,1,3\nwithdrawal,1,2,1\ndispute,1,2,\nresolve,1,2,\nchargeback,1,2,\nresolve,1,1,\nchargeback,1,1,\ndispute,1,1,\ndispute,1,1,\nresolve,1,1,\nresolve,1,1,\n",
        "1,2.0000,0.0000,2.0000,false\n",
        10,
    );
}

#[test]
fn other_clients_cannot_change_a_deposit() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,2\ndeposit,2,2,3\ndispute,2,1,\ndispute,1,1,\nresolve,2,1,\nchargeback,2,1,\n",
        "1,0.0000,2.0000,2.0000,false\n2,3.0000,0.0000,3.0000,false\n",
        3,
    );
}

#[test]
fn transaction_ids_prevent_duplicate_credits_and_debits() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,5\ndeposit,1,1,20\ndeposit,2,1,20\nwithdrawal,1,2,1\nwithdrawal,1,2,1\ndeposit,1,2,10\ndispute,1,1,\nchargeback,1,1,\ndeposit,2,1,10\n",
        "1,-1.0000,0.0000,-1.0000,true\n",
        5,
    );
}

#[test]
fn invalid_amounts_and_overflow_do_not_change_balances() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,922337203685477.5807\ndeposit,1,2,0.0001\nwithdrawal,1,3,-1\ndeposit,1,4,-1\nwithdrawal,1,5,0.0001\ndeposit,1,2,0.0001\n",
        "1,922337203685477.5807,0.0000,922337203685477.5807,false\n",
        3,
    );
}

#[test]
fn dispute_overflow_preserves_balances_and_transaction_status() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,922337203685477.5807\nwithdrawal,1,2,922337203685477.5807\ndispute,1,1,\ndeposit,1,3,922337203685477.5807\ndispute,1,3,\nresolve,1,1,\ndispute,1,3,\nresolve,1,3,\n",
        "1,922337203685477.5807,0.0000,922337203685477.5807,false\n",
        1,
    );
}

#[test]
fn withdrawals_without_funds_do_not_reserve_transaction_ids() {
    balances(
        "type,client,txid,amount\nwithdrawal,1,7,2\ndeposit,1,1,2\nwithdrawal,1,7,2\ndispute,1,7,\n",
        "1,0.0000,0.0000,0.0000,false\n",
        2,
    );
}

#[test]
fn a_chargeback_preserves_other_dispute_amounts() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,2\ndeposit,1,2,3\ndispute,1,1,\ndispute,1,2,\nchargeback,1,1,\nresolve,1,2,\n",
        "1,0.0000,3.0000,3.0000,true\n",
        1,
    );
}

#[test]
fn control_records_cannot_replace_the_deposit_amount() {
    balances(
        "type,client,txid,amount\ndeposit,1,1,1\ndispute,1,1,999\nresolve,1,1,0\ndispute,1,1,0\nchargeback,1,1,999\n",
        "1,0.0000,0.0000,0.0000,true\n",
        0,
    );
}

#[test]
fn insufficient_funds_still_reports_the_client_account() {
    balances(
        "type,client,txid,amount\nwithdrawal,1,1,1\n",
        "1,0.0000,0.0000,0.0000,false\n",
        1,
    );
}

#[test]
fn decimal_math_preserves_the_smallest_unit() {
    balances(
        "type,client,txid,amount\ndeposit,0,0,0.1\ndeposit,0,4294967295,0.2\nwithdrawal,0,2,0.3\ndeposit,65535,3,0.0001\n",
        "0,0.0000,0.0000,0.0000,false\n65535,0.0001,0.0000,0.0001,false\n",
        0,
    );
}

#[test]
fn headers_allow_column_order_and_whitespace() {
    balances(
        " amount , txid , client , type \n 1.25 , 1 , 2 , deposit \n , 1 , 2 , dispute \n",
        "2,0.0000,1.2500,1.2500,false\n",
        0,
    );
}

#[test]
fn header_only_input_still_produces_output_headers() {
    balances("type,client,txid,amount\n", "", 0);
}

#[test]
fn malformed_csv_stops_processing_without_account_output() {
    for input in [
        "",
        "type,client,txid\ndeposit,1,1\n",
        "type,client,client,amount\ndeposit,1,1,1\n",
        "type,client,txid,amount\ndeposit,1,1,1\ndeposit,1,2,0.00001\n",
        "type,client,txid,amount\ndeposit,1,1,\n",
        "type,client,txid,amount\nwithdrawal,1,1,\n",
        "type,client,txid,amount\ndeposit,1,1,1e2\n",
        "type,client,txid,amount\ndeposit,1,1,NaN\n",
        "type,client,txid,amount\ndeposit,1,1,922337203685477.5808\n",
        "type,client,txid,amount\ndeposit,65536,1,1\n",
        "type,client,txid,amount\ndeposit,1,4294967296,1\n",
        "type,client,txid,amount\nunknown,1,1,1\n",
        "type,client,txid,amount\ndeposit,1,1,1,extra\n",
    ] {
        let output = process(input);
        assert!(!output.status.success(), "input: {input}");
        assert!(output.stdout.is_empty(), "input: {input}");
        assert!(!output.stderr.is_empty(), "input: {input}");
    }
}

#[test]
fn csv_errors_identify_the_row_and_amount_problem() {
    for (input, line, problem) in [
        (
            "type,client,txid,amount\ndeposit,1,1,\n",
            "line 2",
            "missing amount",
        ),
        (
            "type,client,txid,amount\ndeposit,1,1,1\ndeposit,1,2,0.00001\n",
            "line: 3",
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
    let file = InputFile::new("type,client,txid,amount\n");
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

#[test]
fn missing_file_error_contains_the_path() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/no-such-input.csv");
    let output = Command::new(env!("CARGO_BIN_EXE_legendary-octo-spork"))
        .arg(&path)
        .output()
        .expect("failed to execute CLI");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&*path.to_string_lossy()));
}
