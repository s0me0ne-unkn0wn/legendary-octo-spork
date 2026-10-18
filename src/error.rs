use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Usage,
    OpenInput(PathBuf),
    ReadCsv,
    InvalidHeader,
    ExtraFields { line: u64 },
    MissingAmount { line: u64 },
    InvalidAmount { line: u64 },
    WriteAccounts,
    WriteDiagnostics,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => formatter.write_str("usage: legendary-octo-spork <input.csv>"),
            Self::OpenInput(path) => write!(formatter, "failed to open '{}'", path.display()),
            Self::ReadCsv => formatter.write_str("failed to read CSV"),
            Self::InvalidHeader => formatter.write_str(
                "invalid CSV header: require type,client,tx,amount exactly once in any order",
            ),
            Self::ExtraFields { line } => write!(formatter, "too many fields at CSV line {line}"),
            Self::MissingAmount { line } => write!(
                formatter,
                "missing amount at CSV line {line}: deposits and withdrawals require an amount",
            ),
            Self::InvalidAmount { line } => write!(formatter, "invalid amount at CSV line {line}"),
            Self::WriteAccounts => formatter.write_str("failed to write account CSV"),
            Self::WriteDiagnostics => formatter.write_str("failed to write diagnostics"),
        }
    }
}

impl std::error::Error for Error {}
