pub fn assert_accounts(actual: &[u8], expected: &[u8]) {
    let mut actual = csv::Reader::from_reader(actual);
    let mut expected = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(expected);
    assert_eq!(
        actual.headers().expect("invalid account output header"),
        expected.headers().expect("invalid account fixture header")
    );
    let rows = |reader: &mut csv::Reader<&[u8]>| {
        let mut rows = reader
            .records()
            .collect::<Result<Vec<_>, _>>()
            .expect("invalid account CSV");
        rows.sort_unstable_by(|left, right| left.iter().cmp(right.iter()));
        rows
    };
    assert_eq!(rows(&mut actual), rows(&mut expected));
}
