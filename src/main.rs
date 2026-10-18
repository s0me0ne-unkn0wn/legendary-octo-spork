use std::fs::File;
use std::io;

use anyhow::{Context, Result, bail};
use legendary_octo_spork::{Error, process_csv};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next().context(Error::Usage)?;
    if arguments.next().is_some() {
        bail!(Error::Usage);
    }
    let input = File::open(&path).with_context(|| Error::OpenInput(path.into()))?;
    process_csv(input, io::stdout().lock(), io::stderr().lock())
}
