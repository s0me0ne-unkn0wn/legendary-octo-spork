use std::error::Error;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use legendary_octo_spork::{Money, process_csv};

const CLIENT_COUNT: u32 = 1024;

struct Workload {
    label: &'static str,
    input: String,
    output: Vec<u8>,
    rows: u64,
}

fn account_output(counts: &[u32], chargeback: bool) -> Vec<u8> {
    let mut output = String::from("client,available,held,total,locked\n");
    for (client, count) in counts.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let total = Money(i64::from(*count) * 10_001 - if chargeback { 20_001 } else { 0 });
        writeln!(output, "{client},{total},0.0000,{total},{chargeback}")
            .expect("failed to format benchmark output");
    }
    output.into_bytes()
}

fn deposits(rows: u32) -> Workload {
    let mut input = String::with_capacity(rows as usize * 32);
    input.push_str("type,client,tx,amount\n");
    let mut counts = vec![0; CLIENT_COUNT as usize];
    for id in 0..rows {
        let client = id % CLIENT_COUNT;
        writeln!(input, "deposit,{client},{id},1.0001").expect("failed to format benchmark input");
        counts[client as usize] += 1;
    }
    Workload {
        label: "deposits",
        input,
        output: account_output(&counts, false),
        rows: u64::from(rows),
    }
}

fn mixed(groups: u32) -> Workload {
    let mut input = String::with_capacity(groups as usize * 100);
    input.push_str("type,client,tx,amount\n");
    let mut counts = vec![0; CLIENT_COUNT as usize];
    let mut last_ids = vec![0; CLIENT_COUNT as usize];
    for group in 0..groups {
        let client = group % CLIENT_COUNT;
        let id = group * 2;
        writeln!(input, "deposit,{client},{id},2.0001").expect("failed to format benchmark input");
        writeln!(input, "withdrawal,{client},{},1.0000", id + 1)
            .expect("failed to format benchmark input");
        writeln!(input, "dispute,{client},{id}").expect("failed to format benchmark input");
        writeln!(input, "resolve,{client},{id}").expect("failed to format benchmark input");
        counts[client as usize] += 1;
        last_ids[client as usize] = id;
    }
    let mut rows = u64::from(groups) * 4;
    for (client, count) in counts.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let id = last_ids[client];
        writeln!(input, "dispute,{client},{id}").expect("failed to format benchmark input");
        writeln!(input, "chargeback,{client},{id}").expect("failed to format benchmark input");
        rows += 2;
    }
    Workload {
        label: "mixed",
        input,
        output: account_output(&counts, true),
        rows,
    }
}

fn execute(workload: &Workload) -> Duration {
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let start = Instant::now();
    process_csv(workload.input.as_bytes(), &mut output, &mut diagnostics)
        .expect("failed to process benchmark workload");
    let duration = start.elapsed();
    assert_eq!(output, workload.output, "incorrect benchmark balances");
    assert!(diagnostics.is_empty(), "invalid benchmark transactions");
    duration
}

fn measure(workload: Workload, rounds: usize) {
    execute(&workload);
    let mut durations: Vec<_> = (0..rounds).map(|_| execute(&workload)).collect();
    durations.sort_unstable();
    let seconds = durations[durations.len() / 2].as_secs_f64();
    println!(
        "{}: rows={} median_seconds={seconds:.6} rows_per_second={:.0} mib_per_second={:.2}",
        workload.label,
        workload.rows,
        workload.rows as f64 / seconds,
        workload.input.len() as f64 / (1024.0 * 1024.0) / seconds
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let rows: u32 = std::env::var("TX_BENCH_ROWS")
        .ok()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(1_000_000);
    let rounds: usize = std::env::var("TX_BENCH_ROUNDS")
        .ok()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(5);
    if rows == 0 || rounds == 0 {
        return Err("benchmark rows and rounds must exceed zero".into());
    }
    measure(deposits(rows), rounds);
    measure(mixed((rows / 4).max(1)), rounds);
    Ok(())
}
