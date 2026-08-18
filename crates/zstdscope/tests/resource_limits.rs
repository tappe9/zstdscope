use zstdscope::{InspectionLimits, ResourceLimitKind, ZstdError, inspect, inspect_with_limits};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;

#[test]
fn existing_inspect_remains_unlimited_by_default() {
    let input = [minimal_standard_frame(), minimal_standard_frame()].concat();

    let file = inspect(&input).expect("legacy inspect API must remain unbounded");

    assert_eq!(file.frames.len(), 2);
}

#[test]
fn frame_limit_accepts_exact_limit_and_rejects_next_frame_at_its_offset() {
    let input = [minimal_standard_frame(), minimal_standard_frame()].concat();
    let exact_limits = limits(2, usize::MAX, usize::MAX);

    let file =
        inspect_with_limits(&input, exact_limits).expect("two frames must fit a limit of two");
    assert_eq!(file.frames.len(), 2);

    let error = inspect_with_limits(&input, limits(1, usize::MAX, usize::MAX)).unwrap_err();
    assert_eq!(
        error,
        ZstdError::ResourceLimitExceeded {
            offset: 9,
            resource: ResourceLimitKind::Frames,
            limit: 1,
        }
    );
}

#[test]
fn per_frame_block_limit_accepts_exact_limit_and_rejects_next_block_header() {
    let input = standard_frame_with_empty_raw_blocks(2);

    let file = inspect_with_limits(&input, limits(1, 2, usize::MAX))
        .expect("two blocks must fit a per-frame limit of two");
    assert_eq!(file.frames.len(), 1);

    let error = inspect_with_limits(&input, limits(1, 1, usize::MAX)).unwrap_err();
    assert_eq!(
        error,
        ZstdError::ResourceLimitExceeded {
            offset: 9,
            resource: ResourceLimitKind::BlocksPerFrame,
            limit: 1,
        }
    );
}

#[test]
fn total_block_limit_is_enforced_across_concatenated_frames() {
    let input = [
        standard_frame_with_empty_raw_blocks(1),
        standard_frame_with_empty_raw_blocks(2),
    ]
    .concat();

    let file = inspect_with_limits(&input, limits(2, 2, 3))
        .expect("three total blocks must fit a total limit of three");
    assert_eq!(file.frames.len(), 2);

    let error = inspect_with_limits(&input, limits(2, 2, 2)).unwrap_err();
    assert_eq!(
        error,
        ZstdError::ResourceLimitExceeded {
            offset: 18,
            resource: ResourceLimitKind::TotalBlocks,
            limit: 2,
        }
    );
}

#[test]
fn the_more_specific_per_frame_limit_wins_when_both_block_limits_are_exhausted() {
    let input = standard_frame_with_empty_raw_blocks(2);

    let error = inspect_with_limits(&input, limits(1, 1, 1)).unwrap_err();

    assert_eq!(
        error,
        ZstdError::ResourceLimitExceeded {
            offset: 9,
            resource: ResourceLimitKind::BlocksPerFrame,
            limit: 1,
        }
    );
}

fn limits(
    max_frames: usize,
    max_blocks_per_frame: usize,
    max_total_blocks: usize,
) -> InspectionLimits {
    InspectionLimits {
        max_frames,
        max_blocks_per_frame,
        max_total_blocks,
    }
}

fn minimal_standard_frame() -> Vec<u8> {
    standard_frame_with_empty_raw_blocks(1)
}

fn standard_frame_with_empty_raw_blocks(count: usize) -> Vec<u8> {
    assert!(count > 0);

    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00]);

    for index in 0..count {
        let is_last = index + 1 == count;
        let header = u32::from(u8::from(is_last));
        frame.extend_from_slice(&header.to_le_bytes()[..3]);
    }

    frame
}
