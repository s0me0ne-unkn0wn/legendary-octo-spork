use std::collections::{HashMap, hash_map::Entry};
use std::fmt;
use std::io::{BufWriter, Write};

use anyhow::Context;

use crate::{ClientId, IO_BUFFER_CAPACITY, Money, Transaction, TxId, TxType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientInfo {
    id: ClientId,
    total: Money,
    on_hold: Money,
    lock: bool,
}

impl ClientInfo {
    fn new(id: ClientId) -> Self {
        Self {
            id,
            total: Money(0),
            on_hold: Money(0),
            lock: false,
        }
    }

    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn total(&self) -> Money {
        self.total
    }

    pub fn on_hold(&self) -> Money {
        self.on_hold
    }

    pub fn available(&self) -> Money {
        Money(
            self.total
                .0
                .checked_sub(self.on_hold.0)
                .expect("account balance overflow"),
        )
    }

    pub fn lock(&self) -> bool {
        self.lock
    }

    fn update(&mut self, total: i64, on_hold: i64) -> Result<(), TransactionError> {
        total
            .checked_sub(on_hold)
            .ok_or(TransactionError::Overflow)?;
        self.total = Money(total);
        self.on_hold = Money(on_hold);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum TxStatus {
    Deposit(Money),
    Dispute(Money),
    Withdrawal,
    Chargeback,
}

#[derive(Debug)]
struct TxRecord {
    client_id: ClientId,
    status: TxStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionError {
    NegativeAmount,
    DuplicateTransaction,
    AccountLock,
    InsufficientFunds,
    UnknownTransaction,
    ClientMismatch,
    WithdrawalControl,
    DisputeConflict,
    NoDispute,
    Overflow,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NegativeAmount => "amount must not be negative",
            Self::DuplicateTransaction => "transaction ID already exists",
            Self::AccountLock => "account lock prevents transactions",
            Self::InsufficientFunds => "insufficient available funds",
            Self::UnknownTransaction => "transaction does not exist",
            Self::ClientMismatch => "transaction belongs to another client",
            Self::WithdrawalControl => {
                "withdrawal does not support disputes, resolves, or chargebacks"
            }
            Self::DisputeConflict => "transaction has an active dispute or chargeback",
            Self::NoDispute => "transaction requires an active dispute",
            Self::Overflow => "account balance exceeds the numeric range",
        })
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug, Default)]
pub struct Engine {
    clients: HashMap<ClientId, ClientInfo>,
    transactions: HashMap<TxId, TxRecord>,
}

impl Engine {
    pub fn clients(&self) -> &HashMap<ClientId, ClientInfo> {
        &self.clients
    }

    pub fn into_clients(self) -> HashMap<ClientId, ClientInfo> {
        self.clients
    }

    pub fn apply(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let (amount, deposit) = match transaction.ty {
            TxType::Deposit(amount) => (amount.0, true),
            TxType::Withdrawal(amount) => (amount.0, false),
            _ => return self.apply_control(transaction),
        };
        if amount < 0 {
            return Err(TransactionError::NegativeAmount);
        }
        let entry = match self.transactions.entry(transaction.id) {
            Entry::Vacant(entry) => entry,
            Entry::Occupied(_) => return Err(TransactionError::DuplicateTransaction),
        };
        let account = self
            .clients
            .entry(transaction.client_id)
            .or_insert_with(|| ClientInfo::new(transaction.client_id));
        if account.lock {
            return Err(TransactionError::AccountLock);
        }
        let (total, status) = if deposit {
            (
                account
                    .total
                    .0
                    .checked_add(amount)
                    .ok_or(TransactionError::Overflow)?,
                TxStatus::Deposit(Money(amount)),
            )
        } else {
            if account.available().0 < amount {
                return Err(TransactionError::InsufficientFunds);
            }
            (
                account
                    .total
                    .0
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?,
                TxStatus::Withdrawal,
            )
        };
        account.update(total, account.on_hold.0)?;
        entry.insert(TxRecord {
            client_id: transaction.client_id,
            status,
        });
        Ok(())
    }

    fn apply_control(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let record = self
            .transactions
            .get_mut(&transaction.id)
            .ok_or(TransactionError::UnknownTransaction)?;
        if record.client_id != transaction.client_id {
            return Err(TransactionError::ClientMismatch);
        }
        let account = self
            .clients
            .get_mut(&transaction.client_id)
            .expect("transaction account missing");
        if account.lock {
            return Err(TransactionError::AccountLock);
        }
        let (total, on_hold, status, lock) = match (transaction.ty, record.status) {
            (TxType::Dispute, TxStatus::Deposit(amount)) => (
                account.total.0,
                account
                    .on_hold
                    .0
                    .checked_add(amount.0)
                    .ok_or(TransactionError::Overflow)?,
                TxStatus::Dispute(amount),
                false,
            ),
            (TxType::Resolve, TxStatus::Dispute(amount)) => (
                account.total.0,
                account
                    .on_hold
                    .0
                    .checked_sub(amount.0)
                    .ok_or(TransactionError::Overflow)?,
                TxStatus::Deposit(amount),
                false,
            ),
            (TxType::Chargeback, TxStatus::Dispute(amount)) => (
                account
                    .total
                    .0
                    .checked_sub(amount.0)
                    .ok_or(TransactionError::Overflow)?,
                account
                    .on_hold
                    .0
                    .checked_sub(amount.0)
                    .ok_or(TransactionError::Overflow)?,
                TxStatus::Chargeback,
                true,
            ),
            (_, TxStatus::Withdrawal) => return Err(TransactionError::WithdrawalControl),
            (TxType::Dispute, _) => return Err(TransactionError::DisputeConflict),
            _ => return Err(TransactionError::NoDispute),
        };
        account.update(total, on_hold)?;
        account.lock = lock;
        record.status = status;
        Ok(())
    }
}

pub fn process_txs<Input, SourceError, Diagnostics>(
    transactions: Input,
    diagnostics: Diagnostics,
) -> anyhow::Result<HashMap<ClientId, ClientInfo>>
where
    Input: IntoIterator<Item = Result<Transaction, SourceError>>,
    SourceError: Into<anyhow::Error>,
    Diagnostics: Write,
{
    let mut engine = Engine::default();
    let mut diagnostics = BufWriter::with_capacity(IO_BUFFER_CAPACITY, diagnostics);
    for transaction in transactions {
        let transaction = match transaction {
            Ok(transaction) => transaction,
            Err(error) => {
                diagnostics.flush().context("failed to write diagnostics")?;
                let error: anyhow::Error = error.into();
                return Err(error.context("failed to read transaction"));
            }
        };
        if let Err(error) = engine.apply(transaction) {
            writeln!(
                diagnostics,
                "client {}, transaction {}: {error}",
                *transaction.client_id, *transaction.id
            )
            .context("failed to write diagnostics")?;
        }
    }
    diagnostics.flush().context("failed to write diagnostics")?;
    Ok(engine.into_clients())
}
