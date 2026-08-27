use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC: u32 = 0x184D_2A55;
const REFERENCE_COMPRESSED_CHECKSUM: &str =
    include_str!("../../zstdscope/tests/fixtures/reference/compressed-checksum.zst.hex");

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[test]
fn json_v1_minimal_standard_frame_matches_schema_fixture() {
    let output = run_json(&minimal_standard_frame());

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let actual = parse_json(output.stdout);

    let expected = json!({
        "schema_version": 1,
        "input_size": "9",
        "frames": [
            {
                "index": 0,
                "span": {
                    "offset": "0",
                    "length": "9"
                },
                "kind": {
                    "type": "standard",
                    "data": {
                        "magic_span": {
                            "offset": "0",
                            "length": "4"
                        },
                        "header": {
                            "span": {
                                "offset": "4",
                                "length": "2"
                            },
                            "descriptor": 0,
                            "descriptor_span": {
                                "offset": "4",
                                "length": "1"
                            },
                            "window_descriptor_span": {
                                "offset": "5",
                                "length": "1"
                            },
                            "frame_content_size": null,
                            "dictionary_id": null,
                            "window_size": "1024",
                            "content_checksum_flag": false,
                            "single_segment": false,
                            "unused_bit": false
                        },
                        "blocks": [
                            {
                                "index": 0,
                                "header_span": {
                                    "offset": "6",
                                    "length": "3"
                                },
                                "content_span": {
                                    "offset": "9",
                                    "length": "0"
                                },
                                "block_type": "raw",
                                "declared_size": 0,
                                "encoded_content_size": 0,
                                "is_last": true
                            }
                        ],
                        "content_checksum": null
                    }
                }
            }
        ]
    });

    assert_eq!(actual, expected);
}

#[test]
fn json_standard_frame_has_explicit_v1_snake_case_shape() {
    let output = run_json(&decode_hex(REFERENCE_COMPRESSED_CHECKSUM));

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let json = parse_json(output.stdout);

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["input_size"], "63");
    assert!(json.get("inputSize").is_none());
    assert_eq!(json["frames"][0]["index"], 0);
    assert_eq!(json["frames"][0]["kind"]["type"], "standard");

    let standard = &json["frames"][0]["kind"]["data"];
    assert_eq!(standard["header"]["frame_content_size"]["value"], "10240");
    assert!(standard["header"].get("frameContentSize").is_none());
    assert_eq!(standard["blocks"][0]["block_type"], "compressed");
    assert_eq!(standard["blocks"][0]["declared_size"], 49);
    assert_eq!(standard["blocks"][0]["encoded_content_size"], 49);
    assert_eq!(standard["content_checksum"]["value"], 1_002_594_435_u64);
}

#[test]
fn json_preserves_dictionary_id_absent_vs_explicit_zero() {
    let absent = parse_json(run_json(&minimal_standard_frame()).stdout);
    let absent_header = &absent["frames"][0]["kind"]["data"]["header"];
    assert!(absent_header["dictionary_id"].is_null());

    let explicit_zero = parse_json(run_json(&explicit_zero_dictionary_id_frame()).stdout);
    let dictionary_id = &explicit_zero["frames"][0]["kind"]["data"]["header"]["dictionary_id"];
    assert_eq!(dictionary_id["encoded"], 0);
    assert_eq!(dictionary_id["span"]["offset"], "6");
    assert_eq!(dictionary_id["span"]["length"], "1");
}

#[test]
fn json_skippable_frame_uses_tagged_snake_case_shape() {
    let output = run_json(&skippable_frame(&[0xAA, 0xBB]));

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let json = parse_json(output.stdout);

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["frames"][0]["kind"]["type"], "skippable");
    let skippable = &json["frames"][0]["kind"]["data"];
    assert_eq!(skippable["variant"], 5);
    assert_eq!(skippable["declared_payload_size"], 2);
    assert_eq!(skippable["payload_span"]["offset"], "8");
    assert_eq!(skippable["payload_span"]["length"], "2");
}

#[test]
fn json_preserves_rle_declared_and_encoded_sizes() {
    let output = run_json(&rle_frame());

    assert!(output.status.success());
    let json = parse_json(output.stdout);
    let block = &json["frames"][0]["kind"]["data"]["blocks"][0];

    assert_eq!(block["block_type"], "rle");
    assert_eq!(block["declared_size"], 17);
    assert_eq!(block["encoded_content_size"], 1);
}

#[test]
fn json_parse_failure_returns_nonzero_without_partial_stdout() {
    let output = run_json(&0x1234_5678_u32.to_le_bytes());

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = utf8(output.stderr);
    assert!(stderr.contains("parse error:"));
    assert!(stderr.contains("invalid top-level magic 0x12345678 at byte offset 0"));
}

fn run_json(input: &[u8]) -> Output {
    let file = TempInput::new(input);
    Command::new(binary())
        .arg("inspect")
        .arg(&file.path)
        .arg("--json")
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

fn explicit_zero_dictionary_id_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0x00, 0x00]);
    frame
}

fn rle_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00, 0x8B, 0x00, 0x00, 0x7F]);
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

fn parse_json(bytes: Vec<u8>) -> Value {
    serde_json::from_slice(&bytes).expect("--json output must be valid JSON")
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("CLI output must be UTF-8")
}

fn unique_temp_path() -> PathBuf {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "zstdscope-json-{}-{sequence}.zst",
        std::process::id()
    ))
}

struct TempInput {
    path: PathBuf,
}

impl TempInput {
    fn new(bytes: &[u8]) -> Self {
        let path = unique_temp_path();
        fs::write(&path, bytes).expect("temporary fixture must be writable");
        Self { path }
    }
}

impl Drop for TempInput {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
