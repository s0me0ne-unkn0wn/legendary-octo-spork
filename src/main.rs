use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use legendary_octo_spork::process_csv;

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments
        .next()
        .ok_or("usage: legendary-octo-spork <input.csv>")?;
    if arguments.next().is_some() {
        return Err("usage: legendary-octo-spork <input.csv>".into());
    }
    let input = File::open(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to open '{}': {error}", Path::new(&path).display()),
        )
    })?;
    process_csv(input, io::stdout().lock(), io::stderr().lock())?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "error: {error}");
            ExitCode::FAILURE
        }
    }
}
