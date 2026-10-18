#[path = "../tests/common/mod.rs"]
mod common;

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Duration;

use common::assert_accounts;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
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

fn execute(workload: &Workload) -> (Vec<u8>, Vec<u8>) {
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    process_csv(
        black_box(workload.input.as_bytes()),
        &mut output,
        &mut diagnostics,
    )
    .expect("failed to process benchmark workload");
    (output, diagnostics)
}

fn throughput(criterion: &mut Criterion) {
    let rows: u32 = std::env::var("TX_BENCH_ROWS").map_or(1_000_000, |value| {
        value.parse().expect("invalid TX_BENCH_ROWS")
    });
    assert!(rows > 0, "TX_BENCH_ROWS must exceed zero");
    let mut group = criterion.benchmark_group("csv");
    for workload in [deposits(rows), mixed((rows / 4).max(1))] {
        let (output, diagnostics) = execute(&workload);
        assert_accounts(&output, &workload.output);
        assert!(diagnostics.is_empty(), "invalid benchmark transactions");
        group.throughput(Throughput::ElementsAndBytes {
            elements: workload.rows,
            bytes: workload.input.len() as u64,
        });
        group.bench_function(BenchmarkId::new(workload.label, workload.rows), |bencher| {
            bencher.iter(|| execute(&workload));
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(10));
    targets = throughput
}
criterion_main!(benches);
