use zstdscope::{ByteSpan, ZstdError};

#[test]
fn byte_span_end_uses_checked_arithmetic() {
    assert_eq!(
        ByteSpan {
            offset: 4,
            length: 6,
        }
        .end(),
        Some(10)
    );

    assert_eq!(
        ByteSpan {
            offset: u64::MAX,
            length: 1,
        }
        .end(),
        None
    );
}

#[test]
fn byte_span_reports_empty_length() {
    assert!(
        ByteSpan {
            offset: 42,
            length: 0,
        }
        .is_empty()
    );
    assert!(
        !ByteSpan {
            offset: 42,
            length: 1,
        }
        .is_empty()
    );
}

#[test]
fn zstd_error_implements_standard_error() {
    fn assert_error<T: std::error::Error>() {}

    assert_error::<ZstdError>();
}
