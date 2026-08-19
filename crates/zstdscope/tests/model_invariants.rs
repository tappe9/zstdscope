mod support;

use support::assert_model_invariants;
use zstdscope::inspect;

const STANDARD_MAGIC: u32 = 0xFD2F_B528;

#[test]
fn successful_parse_satisfies_structural_model_invariants() {
    let input = [minimal_standard_frame(), minimal_standard_frame()].concat();
    let file = inspect(&input).expect("fixture must parse");

    assert_model_invariants(&input, &file);
}

fn minimal_standard_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00]);
    frame.extend_from_slice(&[0x01, 0x00, 0x00]);
    frame
}
