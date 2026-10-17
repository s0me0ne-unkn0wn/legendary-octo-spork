use legendary_octo_spork::{Money, MoneyError};

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
        let value: Money<i64> = text.parse().expect("failed to parse decimal amount");
        assert_eq!(value, Money(minor_units));
        assert_eq!(value.to_string(), output);
    }
}

#[test]
fn i128_values_preserve_both_numeric_limits() {
    for (text, minor_units) in [
        ("17014118346046923173168730371588410.5727", i128::MAX),
        ("-17014118346046923173168730371588410.5728", i128::MIN),
    ] {
        let value: Money<i128> = text.parse().expect("failed to parse i128 limit");
        assert_eq!(value, Money(minor_units));
        assert_eq!(value.to_string(), text);
    }
}

#[test]
fn decimal_syntax_rejects_ambiguous_or_inexact_values() {
    for text in [
        "", "-", "+1", "--1", ".1", "1.", "1.2.3", " 1", "1 ", "1e2", "NaN", "inf", "１", "1,23",
    ] {
        assert!(text.parse::<Money<i64>>().is_err(), "amount: {text}");
    }
    assert_eq!("1.00001".parse::<Money<i64>>(), Err(MoneyError::Precision));
}

#[test]
fn amounts_outside_the_numeric_range_do_not_wrap() {
    for text in [
        "922337203685477.5808",
        "-922337203685477.5809",
        "922337203685478",
        "999999999999999999999999999999999999999999999999999999999999",
    ] {
        assert_eq!(text.parse::<Money<i64>>(), Err(MoneyError::Overflow));
    }
}
