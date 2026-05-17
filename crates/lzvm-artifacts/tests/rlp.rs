use lzvm_artifacts::rlp::{encode_rlp, parse_rlp, RlpError, RlpItem};

#[test]
fn decodes_canonical_rlp_values() {
    assert_eq!(
        parse_rlp(&[0x7f]).expect("single byte should decode"),
        RlpItem::Bytes(vec![0x7f])
    );
    assert_eq!(
        parse_rlp(&[0x80]).expect("empty bytes should decode"),
        RlpItem::Bytes(Vec::new())
    );
    assert_eq!(
        parse_rlp(&[0x83, b'c', b'a', b't']).expect("short bytes should decode"),
        RlpItem::Bytes(b"cat".to_vec())
    );
    assert_eq!(
        parse_rlp(&[0xc0]).expect("empty list should decode"),
        RlpItem::List(Vec::new())
    );
    assert_eq!(
        parse_rlp(&[0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g'])
            .expect("list should decode"),
        RlpItem::List(vec![
            RlpItem::Bytes(b"cat".to_vec()),
            RlpItem::Bytes(b"dog".to_vec()),
        ])
    );
    assert_eq!(
        parse_rlp(&[0xc7, 0xc0, 0xc5, 0x83, b'd', b'o', b'g', 0xc0])
            .expect("nested list should decode"),
        RlpItem::List(vec![
            RlpItem::List(Vec::new()),
            RlpItem::List(vec![
                RlpItem::Bytes(b"dog".to_vec()),
                RlpItem::List(Vec::new())
            ]),
        ])
    );
}

#[test]
fn encodes_short_rlp_values() {
    assert_eq!(encode_rlp(&RlpItem::Bytes(vec![0x7f])), vec![0x7f]);
    assert_eq!(encode_rlp(&RlpItem::Bytes(Vec::new())), vec![0x80]);
    assert_eq!(
        encode_rlp(&RlpItem::Bytes(b"cat".to_vec())),
        vec![0x83, b'c', b'a', b't']
    );
    assert_eq!(encode_rlp(&RlpItem::List(Vec::new())), vec![0xc0]);
    assert_eq!(
        encode_rlp(&RlpItem::List(vec![
            RlpItem::Bytes(b"cat".to_vec()),
            RlpItem::Bytes(b"dog".to_vec()),
        ])),
        vec![0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g']
    );
}

#[test]
fn encodes_long_rlp_payloads() {
    let long_bytes = RlpItem::Bytes(vec![b'a'; 56]);
    let mut expected_bytes = vec![0xb8, 56];
    expected_bytes.extend(std::iter::repeat_n(b'a', 56));
    assert_eq!(encode_rlp(&long_bytes), expected_bytes);

    let long_list = RlpItem::List(vec![RlpItem::Bytes(Vec::new()); 56]);
    let mut expected_list = vec![0xf8, 56];
    expected_list.extend(std::iter::repeat_n(0x80, 56));
    assert_eq!(encode_rlp(&long_list), expected_list);
}

#[test]
fn round_trips_nested_rlp_items() {
    let item = RlpItem::List(vec![
        RlpItem::List(Vec::new()),
        RlpItem::List(vec![
            RlpItem::Bytes(b"dog".to_vec()),
            RlpItem::List(Vec::new()),
        ]),
    ]);

    assert_eq!(parse_rlp(&encode_rlp(&item)), Ok(item));
}

#[test]
fn rejects_non_canonical_single_byte_strings() {
    let error = parse_rlp(&[0x81, 0x7f]).expect_err("single byte should use compact form");

    assert!(matches!(
        error,
        RlpError::NonCanonicalSingleByte { offset: 0 }
    ));
}

#[test]
fn rejects_trailing_bytes() {
    let error = parse_rlp(&[0x80, 0x80]).expect_err("trailing bytes should be rejected");

    assert!(matches!(
        error,
        RlpError::UnexpectedTrailingBytes { offset: 1 }
    ));
}

#[test]
fn decodes_long_rlp_payloads() {
    let mut long_bytes = vec![0xb8, 56];
    long_bytes.extend(std::iter::repeat_n(b'a', 56));
    assert_eq!(
        parse_rlp(&long_bytes).expect("long bytes should decode"),
        RlpItem::Bytes(vec![b'a'; 56])
    );

    let mut long_list = vec![0xf8, 56];
    long_list.extend(std::iter::repeat_n(0x80, 56));
    assert_eq!(
        parse_rlp(&long_list).expect("long list should decode"),
        RlpItem::List(vec![RlpItem::Bytes(Vec::new()); 56])
    );
}

#[test]
fn rejects_non_canonical_long_lengths() {
    let error = parse_rlp(&[0xb8, 0x01, 0x80]).expect_err("short payload should use short form");

    assert!(matches!(error, RlpError::NonCanonicalLength { offset: 0 }));
}

#[test]
fn rejects_truncated_payloads() {
    let error = parse_rlp(&[0x83, b'c']).expect_err("truncated payload should be rejected");

    assert!(matches!(
        error,
        RlpError::UnexpectedEof {
            offset: 1,
            needed: 3,
            available: 1,
        }
    ));
}
