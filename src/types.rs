use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::Money;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct ClientId(pub u16);

impl Deref for ClientId {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct TxId(pub u32);

impl Deref for TxId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TxType<Value> {
    Deposit(Money<Value>),
    Withdrawal(Money<Value>),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transaction<Value> {
    pub ty: TxType<Value>,
    pub client_id: ClientId,
    pub id: TxId,
}
