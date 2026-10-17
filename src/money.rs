use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, Div, Rem};
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const MONEY_PRECISION: u32 = 4;
pub const MONEY_SCALE: u16 = 10_u16.pow(MONEY_PRECISION);

pub trait MoneyValue:
    Copy + Ord + fmt::Debug + fmt::Display + From<u16> + Div<Output = Self> + Rem<Output = Self>
{
    const ZERO: Self;

    fn checked_add(self, other: Self) -> Option<Self>;
    fn checked_sub(self, other: Self) -> Option<Self>;
    fn checked_mul(self, other: Self) -> Option<Self>;
}

macro_rules! impl_money_value {
    ($($value_type:ty),+ $(,)?) => {
        $(
            impl MoneyValue for $value_type {
                const ZERO: Self = 0;

                fn checked_add(self, other: Self) -> Option<Self> {
                    self.checked_add(other)
                }

                fn checked_sub(self, other: Self) -> Option<Self> {
                    self.checked_sub(other)
                }

                fn checked_mul(self, other: Self) -> Option<Self> {
                    self.checked_mul(other)
                }
            }
        )+
    };
}

impl_money_value!(i64, i128);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Money<Value>(pub Value);

impl<Value> Deref for Money<Value> {
    type Target = Value;

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

impl<Value: MoneyValue> FromStr for Money<Value> {
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
        let mut minor_units = Value::ZERO;
        let ten = Value::from(10);
        for digit in whole.bytes().chain(fraction.bytes()) {
            if !digit.is_ascii_digit() {
                return Err(MoneyError::Syntax);
            }
            let digit_value = Value::from(u16::from(digit - b'0'));
            minor_units = minor_units
                .checked_mul(ten)
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
            minor_units = minor_units.checked_mul(ten).ok_or(MoneyError::Overflow)?;
        }
        Ok(Self(minor_units))
    }
}

impl<Value: MoneyValue> fmt::Display for Money<Value> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale = Value::from(MONEY_SCALE);
        let mut whole = self.0 / scale;
        let mut fraction = self.0 % scale;
        if self.0 < Value::ZERO {
            formatter.write_str("-")?;
            whole = Value::ZERO.checked_sub(whole).ok_or(fmt::Error)?;
            fraction = Value::ZERO.checked_sub(fraction).ok_or(fmt::Error)?;
        }
        write!(
            formatter,
            "{whole}.{fraction:0width$}",
            width = MONEY_PRECISION as usize
        )
    }
}

impl<Value: MoneyValue> Serialize for Money<Value> {
    fn serialize<Output: Serializer>(
        &self,
        serializer: Output,
    ) -> Result<Output::Ok, Output::Error> {
        serializer.collect_str(self)
    }
}

struct MoneyVisitor<Value>(PhantomData<Value>);

impl<'input, Value: MoneyValue> Visitor<'input> for MoneyVisitor<Value> {
    type Value = Money<Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a decimal amount with at most four decimal places")
    }

    fn visit_str<Error: de::Error>(self, value: &str) -> Result<Self::Value, Error> {
        value.parse().map_err(de::Error::custom)
    }
}

impl<'input, Value: MoneyValue> Deserialize<'input> for Money<Value> {
    fn deserialize<Input: Deserializer<'input>>(deserializer: Input) -> Result<Self, Input::Error> {
        deserializer.deserialize_str(MoneyVisitor(PhantomData))
    }
}
