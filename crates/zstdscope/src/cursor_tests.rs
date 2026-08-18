use crate::{ZstdError, cursor::Cursor};

#[test]
fn reads_little_endian_integer_widths() {
    let input = [
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x10, 0x20, 0x30,
    ];
    let mut cursor = Cursor::new(&input);

    assert_eq!(cursor.read_u8().unwrap(), 0x11);
    assert_eq!(cursor.read_u16_le().unwrap(), 0x3322);
    assert_eq!(cursor.read_u24_le().unwrap(), 0x665544);
    assert_eq!(cursor.read_u32_le().unwrap(), 0xaa998877);
    assert_eq!(cursor.read_u64_le().unwrap(), 0x302010ffeeddccbb);
    assert_eq!(cursor.remaining(), 0);
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
fn truncated_reads_report_required_width_without_moving() {
    let mut u8_cursor = Cursor::new(&[]);
    assert_eof(u8_cursor.read_u8().unwrap_err(), 0, 1, 0);
    assert_eq!(u8_cursor.position(), 0);

    let mut u16_cursor = Cursor::new(&[0xaa]);
    assert_eof(u16_cursor.read_u16_le().unwrap_err(), 0, 2, 1);
    assert_eq!(u16_cursor.position(), 0);

    let mut u24_cursor = Cursor::new(&[0xaa, 0xbb]);
    assert_eof(u24_cursor.read_u24_le().unwrap_err(), 0, 3, 2);
    assert_eq!(u24_cursor.position(), 0);

    let mut u32_cursor = Cursor::new(&[0xaa, 0xbb, 0xcc]);
    assert_eof(u32_cursor.read_u32_le().unwrap_err(), 0, 4, 3);
    assert_eq!(u32_cursor.position(), 0);

    let mut u64_cursor = Cursor::new(&[0; 7]);
    assert_eof(u64_cursor.read_u64_le().unwrap_err(), 0, 8, 7);
    assert_eq!(u64_cursor.position(), 0);
}

#[test]
fn truncated_read_reports_current_offset() {
    let input = [0xaa, 0xbb];
    let mut cursor = Cursor::new(&input);
    cursor.skip(1).unwrap();

    assert_eof(cursor.read_u32_le().unwrap_err(), 1, 4, 1);
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

    assert_eof(cursor.skip(4).unwrap_err(), 0, 4, 3);
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

fn assert_eof(error: ZstdError, offset: u64, needed: usize, remaining: usize) {
    assert_eq!(
        error,
        ZstdError::UnexpectedEof {
            offset,
            needed,
            remaining,
        }
    );
}
