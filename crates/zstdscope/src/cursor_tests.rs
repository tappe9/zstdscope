use crate::{cursor::Cursor, ZstdError};

#[test]
fn reads_little_endian_integer_widths() {
    let input = [
        0x11,
        0x22, 0x33,
        0x44, 0x55, 0x66,
        0x77, 0x88, 0x99, 0xaa,
        0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x10, 0x20, 0x30,
    ];
    let mut cursor = Cursor::new(&input);

    assert_eq!(cursor.read_u8().unwrap(), 0x11);
    assert_eq!(cursor.read_u16_le().unwrap(), 0x3322);
    assert_eq!(cursor.read_u24_le().unwrap(), 0x665544);
    assert_eq!(cursor.read_u32_le().unwrap(), 0xaa998877);
    assert_eq!(cursor.read_u64_le().unwrap(), 0x302010ffeeddccbb);
}

#[test]
fn tracks_position_and_remaining_bytes() {
    let input = [1, 2, 3, 4];
    let mut cursor = Cursor::new(&input);

    assert_eq!(cursor.position(), 0);
    assert_eq!(cursor.remaining(), 4);

    assert_eq!(cursor.read_u16_le().unwrap(), 0x0201);

    assert_eq!(cursor.position(), 2);
    assert_eq!(cursor.remaining(), 2);
}

#[test]
fn truncated_read_reports_offset_needed_and_remaining() {
    let input = [0xaa, 0xbb];
    let mut cursor = Cursor::new(&input);
    cursor.skip(1).unwrap();

    assert_eq!(
        cursor.read_u32_le().unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 1,
            needed: 4,
            remaining: 1,
        }
    );
    assert_eq!(cursor.position(), 1);
}

#[test]
fn skip_can_land_exactly_at_end() {
    let input = [1, 2, 3];
    let mut cursor = Cursor::new(&input);

    cursor.skip(3).unwrap();

    assert_eq!(cursor.position(), 3);
    assert_eq!(cursor.remaining(), 0);
}

#[test]
fn skip_beyond_input_returns_unexpected_eof_without_moving() {
    let input = [1, 2, 3];
    let mut cursor = Cursor::new(&input);

    assert_eq!(
        cursor.skip(4).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 0,
            needed: 4,
            remaining: 3,
        }
    );
    assert_eq!(cursor.position(), 0);
}

#[test]
fn overflowing_position_increment_returns_arithmetic_overflow() {
    let input = [1];
    let mut cursor = Cursor::new(&input);
    cursor.read_u8().unwrap();

    assert_eq!(
        cursor.skip(usize::MAX).unwrap_err(),
        ZstdError::ArithmeticOverflow { offset: 1 }
    );
    assert_eq!(cursor.position(), 1);
}
