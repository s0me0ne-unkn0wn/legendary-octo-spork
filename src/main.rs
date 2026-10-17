use std::fs::File;
use std::io;
use std::path::Path;

use anyhow::{Context, Result, bail};
use legendary_octo_spork::process_csv;

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments
        .next()
        .context("usage: legendary-octo-spork <input.csv>")?;
    if arguments.next().is_some() {
        bail!("usage: legendary-octo-spork <input.csv>");
    }
    let input = File::open(&path)
        .with_context(|| format!("failed to open '{}'", Path::new(&path).display()))?;
    process_csv(input, io::stdout().lock(), io::stderr().lock())
}
