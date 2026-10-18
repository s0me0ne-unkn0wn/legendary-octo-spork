use std::collections::HashMap;
use std::fmt;

use crate::types::{ClientId, Money, Transaction, TxId, TxType};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClientInfo {
    pub(crate) total: Money,
    pub(crate) on_hold: Money,
    pub(crate) lock: bool,
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
            TxType::Deposit(amount) => (amount, true),
            TxType::Withdrawal(amount) => (amount, false),
            _ => return self.apply_control(transaction),
        };
        if amount < Money(0) {
            return Err(TransactionError::NegativeAmount);
        }
        let account = if deposit {
            self.clients.entry(transaction.client_id).or_default()
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
                .checked_add(amount)
                .ok_or(TransactionError::Overflow)?
        } else {
            let available = account
                .total
                .checked_sub(account.on_hold)
                .ok_or(TransactionError::Overflow)?;
            if available < amount {
                return Err(TransactionError::InsufficientFunds);
            }
            account
                .total
                .checked_sub(amount)
                .ok_or(TransactionError::Overflow)?
        };
        total
            .checked_sub(account.on_hold)
            .ok_or(TransactionError::Overflow)?;
        account.total = total;
        if deposit {
            self.deposits.insert(
                transaction.id,
                Deposit {
                    client_id: transaction.client_id,
                    amount,
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
        let amount = deposit.amount;
        match (transaction.ty, deposit.dispute) {
            (TxType::Dispute, false) => {
                let on_hold = account
                    .on_hold
                    .checked_add(amount)
                    .ok_or(TransactionError::Overflow)?;
                account
                    .total
                    .checked_sub(on_hold)
                    .ok_or(TransactionError::Overflow)?;
                account.on_hold = on_hold;
                deposit.dispute = true;
            }
            (TxType::Resolve, true) => {
                let on_hold = account
                    .on_hold
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?;
                account
                    .total
                    .checked_sub(on_hold)
                    .ok_or(TransactionError::Overflow)?;
                account.on_hold = on_hold;
                deposit.dispute = false;
            }
            (TxType::Chargeback, true) => {
                let total = account
                    .total
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?;
                let on_hold = account
                    .on_hold
                    .checked_sub(amount)
                    .ok_or(TransactionError::Overflow)?;
                total
                    .checked_sub(on_hold)
                    .ok_or(TransactionError::Overflow)?;
                account.total = total;
                account.on_hold = on_hold;
                account.lock = true;
                deposit.dispute = false;
            }
            (TxType::Dispute, true) => return Err(TransactionError::DisputeConflict),
            _ => return Err(TransactionError::NoDispute),
        }
        Ok(())
    }
}
