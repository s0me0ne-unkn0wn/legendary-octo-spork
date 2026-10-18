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

impl Money {
    #[inline]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    #[inline]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

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

#[test]
fn arithmetic_preserves_minor_units_and_rejects_overflow() {
    for (amount, other, sum, difference) in [
        (10_001, 2, Some(10_003), Some(9_999)),
        (-1, 2, Some(1), Some(-3)),
        (i64::MAX, 0, Some(i64::MAX), Some(i64::MAX)),
        (i64::MIN, 0, Some(i64::MIN), Some(i64::MIN)),
        (i64::MAX, 1, None, Some(i64::MAX - 1)),
        (i64::MAX, -1, Some(i64::MAX - 1), None),
        (i64::MIN, 1, Some(i64::MIN + 1), None),
        (i64::MIN, -1, None, Some(i64::MIN + 1)),
        (0, i64::MIN, Some(i64::MIN), None),
        (i64::MIN, i64::MIN, None, Some(0)),
    ] {
        assert_eq!(Money(amount).checked_add(Money(other)), sum.map(Money));
        assert_eq!(
            Money(amount).checked_sub(Money(other)),
            difference.map(Money)
        );
    }
}

#[test]
fn decimal_values_use_exact_minor_units() {
    for (text, minor_units, output) in [
        ("0", 0, "0.0000"),
        ("-0.0000", 0, "0.0000"),
        ("1", 10_000, "1.0000"),
        ("0001.20", 12_000, "1.2000"),
        ("0.0001", 1, "0.0001"),
        ("-0.0001", -1, "-0.0001"),
        ("-0.9999", -9_999, "-0.9999"),
        ("-1.0001", -10_001, "-1.0001"),
        ("922337203685477.5807", i64::MAX, "922337203685477.5807"),
        ("-922337203685477.5808", i64::MIN, "-922337203685477.5808"),
    ] {
        let value: Money = text.parse().expect("failed to parse decimal amount");
        assert_eq!(value, Money(minor_units));
        assert_eq!(value.to_string(), output);
    }
}

#[test]
fn decimal_syntax_rejects_ambiguous_or_inexact_values() {
    for text in [
        "", "-", "+1", "--1", ".1", "1.", "1.2.3", " 1", "1 ", "1e2", "NaN", "inf", "１", "1,23",
    ] {
        assert!(text.parse::<Money>().is_err(), "amount: {text}");
    }
    assert_eq!("1.00001".parse::<Money>(), Err(MoneyError::Precision));
}

#[test]
fn amounts_outside_the_numeric_range_do_not_wrap() {
    for text in [
        "922337203685477.5808",
        "-922337203685477.5809",
        "922337203685478",
        "999999999999999999999999999999999999999999999999999999999999",
    ] {
        assert_eq!(text.parse::<Money>(), Err(MoneyError::Overflow));
    }
}
