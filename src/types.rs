use crate::{MONEY_PRECISION, MONEY_SCALE};

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
pub enum TxType {
    Deposit(Money),
    Withdrawal(Money),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub ty: TxType,
    pub client_id: ClientId,
    pub id: TxId,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Money(pub i64);

impl Deref for Money {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoneyError {
    Syntax,
    Precision,
    Overflow,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Syntax => "amount must contain decimal digits with an optional minus sign",
            Self::Precision => "amount exceeds four decimal places",
            Self::Overflow => "amount exceeds the numeric range",
        })
    }
}

impl std::error::Error for MoneyError {}

impl FromStr for Money {
    type Err = MoneyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(digits) => (true, digits),
            None => (false, text),
        };
        let (whole, fraction) = match digits.split_once('.') {
            Some((whole, fraction)) if !fraction.is_empty() => (whole, fraction),
            Some(_) => return Err(MoneyError::Syntax),
            None => (digits, ""),
        };
        if whole.is_empty() {
            return Err(MoneyError::Syntax);
        }
        if fraction.len() > MONEY_PRECISION as usize {
            return Err(MoneyError::Precision);
        }
        let mut minor_units = 0_i64;
        for digit in whole.bytes().chain(fraction.bytes()) {
            if !digit.is_ascii_digit() {
                return Err(MoneyError::Syntax);
            }
            let digit_value = i64::from(digit - b'0');
            minor_units = minor_units
                .checked_mul(10)
                .and_then(|value| {
                    if negative {
                        value.checked_sub(digit_value)
                    } else {
                        value.checked_add(digit_value)
                    }
                })
                .ok_or(MoneyError::Overflow)?;
        }
        for _ in fraction.len()..MONEY_PRECISION as usize {
            minor_units = minor_units.checked_mul(10).ok_or(MoneyError::Overflow)?;
        }
        Ok(Self(minor_units))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = (self.0 / MONEY_SCALE).unsigned_abs();
        let fraction = (self.0 % MONEY_SCALE).unsigned_abs();
        if self.0 < 0 {
            formatter.write_str("-")?;
        }
        write!(
            formatter,
            "{whole}.{fraction:0width$}",
            width = MONEY_PRECISION as usize
        )
    }
}

impl Serialize for Money {
    fn serialize<Output: Serializer>(
        &self,
        serializer: Output,
    ) -> Result<Output::Ok, Output::Error> {
        serializer.collect_str(self)
    }
}

struct MoneyVisitor;

impl<'input> Visitor<'input> for MoneyVisitor {
    type Value = Money;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a decimal amount with at most four decimal places")
    }

    fn visit_str<Error: de::Error>(self, value: &str) -> Result<Self::Value, Error> {
        value.parse().map_err(de::Error::custom)
    }
}

impl<'input> Deserialize<'input> for Money {
    fn deserialize<Input: Deserializer<'input>>(deserializer: Input) -> Result<Self, Input::Error> {
        deserializer.deserialize_str(MoneyVisitor)
    }
}
