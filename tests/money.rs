use legendary_octo_spork::{Money, MoneyError};

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
