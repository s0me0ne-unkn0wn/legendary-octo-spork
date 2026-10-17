use std::collections::HashMap;
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

#[derive(Debug)]
struct Deposit {
    client_id: ClientId,
    amount: Money,
    dispute: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionError {
    NegativeAmount,
    AccountLock,
    InsufficientFunds,
    UnknownTransaction,
    ClientMismatch,
    DisputeConflict,
    NoDispute,
    Overflow,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NegativeAmount => "amount must not be negative",
            Self::AccountLock => "account lock prevents transactions",
            Self::InsufficientFunds => "insufficient available funds",
            Self::UnknownTransaction => "transaction does not exist",
            Self::ClientMismatch => "transaction belongs to another client",
            Self::DisputeConflict => "deposit has an active dispute",
            Self::NoDispute => "transaction requires an active dispute",
            Self::Overflow => "account balance exceeds the numeric range",
        })
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug, Default)]
pub struct Engine {
    clients: HashMap<ClientId, ClientInfo>,
    deposits: HashMap<TxId, Deposit>,
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
        let account = if deposit {
            self.clients
                .entry(transaction.client_id)
                .or_insert_with(|| ClientInfo::new(transaction.client_id))
        } else {
            self.clients
                .get_mut(&transaction.client_id)
                .ok_or(TransactionError::InsufficientFunds)?
        };
        if account.lock {
            return Err(TransactionError::AccountLock);
        }
        let total = if deposit {
            account
                .total
                .0
                .checked_add(amount)
                .ok_or(TransactionError::Overflow)?
        } else {
            if account.available().0 < amount {
                return Err(TransactionError::InsufficientFunds);
            }
            account
                .total
                .0
                .checked_sub(amount)
                .ok_or(TransactionError::Overflow)?
        };
        account.update(total, account.on_hold.0)?;
        if deposit {
            self.deposits.insert(
                transaction.id,
                Deposit {
                    client_id: transaction.client_id,
                    amount: Money(amount),
                    dispute: false,
                },
            );
        }
        Ok(())
    }

    fn apply_control(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        let deposit = self
            .deposits
            .get_mut(&transaction.id)
            .ok_or(TransactionError::UnknownTransaction)?;
        if deposit.client_id != transaction.client_id {
            return Err(TransactionError::ClientMismatch);
        }
        let account = self
            .clients
            .get_mut(&transaction.client_id)
            .expect("transaction account missing");
        if account.lock {
            return Err(TransactionError::AccountLock);
        }
        let amount = deposit.amount.0;
        let (total, on_hold, dispute, lock) = match (transaction.ty, deposit.dispute) {
            (TxType::Dispute, false) => (
                account.total.0,
                account
                    .on_hold
                    .0
                    .checked_add(amount)
                    .ok_or(TransactionError::Overflow)?,
                true,
                false,
            ),
            (TxType::Resolve, true) => (
                account.total.0,
                account
                    .on_hold
                    .0
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?,
                false,
                false,
            ),
            (TxType::Chargeback, true) => (
                account
                    .total
                    .0
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?,
                account
                    .on_hold
                    .0
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?,
                false,
                true,
            ),
            (TxType::Dispute, true) => return Err(TransactionError::DisputeConflict),
            _ => return Err(TransactionError::NoDispute),
        };
        account.update(total, on_hold)?;
        account.lock = lock;
        deposit.dispute = dispute;
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
