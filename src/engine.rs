use std::collections::{HashMap, hash_map::Entry};
use std::fmt;
use std::io::{self, BufWriter, Write};

use crate::{ClientId, IO_BUFFER_CAPACITY, Money, MoneyValue, Transaction, TxId, TxType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientInfo<Value> {
    id: ClientId,
    total: Money<Value>,
    on_hold: Money<Value>,
    lock: bool,
}

impl<Value: MoneyValue> ClientInfo<Value> {
    fn new(id: ClientId) -> Self {
        Self {
            id,
            total: Money(Value::ZERO),
            on_hold: Money(Value::ZERO),
            lock: false,
        }
    }

    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn total(&self) -> Money<Value> {
        self.total
    }

    pub fn on_hold(&self) -> Money<Value> {
        self.on_hold
    }

    pub fn available(&self) -> Money<Value> {
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

    fn update(&mut self, total: Value, on_hold: Value) -> Result<(), TransactionError> {
        total
            .checked_sub(on_hold)
            .ok_or(TransactionError::Overflow)?;
        self.total = Money(total);
        self.on_hold = Money(on_hold);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum TxStatus<Value> {
    Deposit(Money<Value>),
    Dispute(Money<Value>),
    Withdrawal,
    Chargeback,
}

#[derive(Debug)]
struct TxRecord<Value> {
    client_id: ClientId,
    status: TxStatus<Value>,
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

#[derive(Debug)]
pub struct Engine<Value = i64> {
    clients: HashMap<ClientId, ClientInfo<Value>>,
    transactions: HashMap<TxId, TxRecord<Value>>,
}

impl<Value> Default for Engine<Value> {
    fn default() -> Self {
        Self {
            clients: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
}

impl<Value: MoneyValue> Engine<Value> {
    pub fn clients(&self) -> &HashMap<ClientId, ClientInfo<Value>> {
        &self.clients
    }

    pub fn into_clients(self) -> HashMap<ClientId, ClientInfo<Value>> {
        self.clients
    }

    pub fn apply(&mut self, transaction: Transaction<Value>) -> Result<(), TransactionError> {
        let (amount, deposit) = match transaction.ty {
            TxType::Deposit(amount) => (amount.0, true),
            TxType::Withdrawal(amount) => (amount.0, false),
            _ => return self.apply_control(transaction),
        };
        if amount < Value::ZERO {
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

    fn apply_control(&mut self, transaction: Transaction<Value>) -> Result<(), TransactionError> {
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

#[derive(Debug)]
pub enum ProcessError<InputError> {
    Input(InputError),
    Diagnostics(io::Error),
}

impl<InputError: fmt::Display> fmt::Display for ProcessError<InputError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => write!(formatter, "failed to read transaction: {error}"),
            Self::Diagnostics(error) => write!(formatter, "failed to write diagnostics: {error}"),
        }
    }
}

impl<InputError: std::error::Error + 'static> std::error::Error for ProcessError<InputError> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Input(error) => Some(error),
            Self::Diagnostics(error) => Some(error),
        }
    }
}

pub fn process_txs<Value, Input, InputError, Diagnostics>(
    transactions: Input,
    diagnostics: Diagnostics,
) -> Result<HashMap<ClientId, ClientInfo<Value>>, ProcessError<InputError>>
where
    Value: MoneyValue,
    Input: IntoIterator<Item = Result<Transaction<Value>, InputError>>,
    Diagnostics: Write,
{
    let mut engine = Engine::default();
    let mut diagnostics = BufWriter::with_capacity(IO_BUFFER_CAPACITY, diagnostics);
    for transaction in transactions {
        let transaction = match transaction {
            Ok(transaction) => transaction,
            Err(error) => {
                diagnostics.flush().map_err(ProcessError::Diagnostics)?;
                return Err(ProcessError::Input(error));
            }
        };
        if let Err(error) = engine.apply(transaction) {
            writeln!(
                diagnostics,
                "client {}, transaction {}: {error}",
                *transaction.client_id, *transaction.id
            )
            .map_err(ProcessError::Diagnostics)?;
        }
    }
    diagnostics.flush().map_err(ProcessError::Diagnostics)?;
    Ok(engine.into_clients())
}
