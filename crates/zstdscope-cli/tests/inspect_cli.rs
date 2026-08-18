use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC: u32 = 0x184D_2A55;
const REFERENCE_COMPRESSED_CHECKSUM: &str =
    include_str!("../../zstdscope/tests/fixtures/reference/compressed-checksum.zst.hex");

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[test]
fn inspect_prints_a_readable_standard_frame_summary() {
    let output = run_inspect(&decode_hex(REFERENCE_COMPRESSED_CHECKSUM));

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = utf8(output.stdout);
    assert!(stdout.contains("Frame #0 Standard  offset=0  size=63"));
    assert!(stdout.contains("Header  offset=4  size=3"));
    assert!(stdout.contains("window_size=10240"));
    assert!(stdout.contains("Block #0 compressed"));
    assert!(stdout.contains("declared_size=49"));
    assert!(stdout.contains("encoded_size=49"));
    assert!(stdout.contains("Content checksum  stored=0x3BC26083"));
    assert!(stdout.contains("not verified"));
}

#[test]
fn inspect_preserves_rle_declared_and_encoded_sizes_in_text_output() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    input.extend_from_slice(&[0x00, 0x00, 0x8B, 0x00, 0x00, 0x7F]);

    let output = run_inspect(&input);

    assert!(output.status.success());
    let stdout = utf8(output.stdout);
    assert!(stdout.contains("Block #0 rle"));
    assert!(stdout.contains("declared_size=17"));
    assert!(stdout.contains("encoded_size=1"));
}

#[test]
fn inspect_prints_multiple_frames_in_input_order() {
    let standard = minimal_standard_frame();
    let skippable = skippable_frame(&[0xAA, 0xBB]);
    let input = [standard.clone(), skippable, standard].concat();

    let output = run_inspect(&input);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = utf8(output.stdout);
    assert!(stdout.contains("Frame #0 Standard  offset=0  size=9"));
    assert!(stdout.contains("Frame #1 Skippable  offset=9  size=10"));
    assert!(stdout.contains("variant=5"));
    assert!(stdout.contains("payload_size=2"));
    assert!(stdout.contains("Frame #2 Standard  offset=19  size=9"));
}

#[test]
fn malformed_input_returns_nonzero_and_writes_only_to_stderr() {
    let output = run_inspect(&0x1234_5678_u32.to_le_bytes());

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = utf8(output.stderr);
    assert!(stderr.contains("parse error:"));
    assert!(stderr.contains("invalid top-level magic 0x12345678 at byte offset 0"));
}

#[test]
fn missing_file_returns_nonzero_and_writes_only_to_stderr() {
    let path = unique_temp_path("missing");
    let output = Command::new(binary())
        .arg("inspect")
        .arg(&path)
        .output()
        .expect("zstdscope binary must start");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = utf8(output.stderr);
    assert!(stderr.contains("I/O error:"));
    assert!(stderr.contains(path.to_string_lossy().as_ref()));
}

fn run_inspect(input: &[u8]) -> Output {
    let file = TempInput::new(input);
    Command::new(binary())
        .arg("inspect")
        .arg(&file.path)
        .output()
        .expect("zstdscope binary must start")
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_zstdscope")
}

fn minimal_standard_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x00]);
    frame
}

fn skippable_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = SKIPPABLE_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn decode_hex(source: &str) -> Vec<u8> {
    let compact: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert_eq!(compact.len() % 2, 0);
    (0..compact.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&compact[index..index + 2], 16).unwrap())
        .collect()
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("CLI output must be UTF-8")
}

fn unique_temp_path(label: &str) -> PathBuf {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "zstdscope-cli-{label}-{}-{sequence}.zst",
        std::process::id()
    ))
}

struct TempInput {
    path: PathBuf,
}

impl TempInput {
    fn new(bytes: &[u8]) -> Self {
        let path = unique_temp_path("input");
        fs::write(&path, bytes).expect("temporary fixture must be writable");
        Self { path }
    }
}

impl Drop for TempInput {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
