# ZstdScope

[![Crates.io](https://img.shields.io/crates/v/zstdscope.svg)](https://crates.io/crates/zstdscope)
[![docs.rs](https://docs.rs/zstdscope/badge.svg)](https://docs.rs/zstdscope)
[![CI](https://github.com/tappe9/zstdscope/actions/workflows/ci.yml/badge.svg)](https://github.com/tappe9/zstdscope/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/zstdscope.svg)](https://github.com/tappe9/zstdscope#license)

ZstdScopeは、Zstandard圧縮データ形式の構造を解析する **Pure Rust製Inspector / Parserツールキット**です。

主目的は圧縮・展開ではなく、`.zst`などのZstandard streamに含まれるFrame、Header、Block、offset、sizeなどのencoded structureを、安全かつ再利用可能なRust APIとCLIから取得することです。Compressed Blockのpayload内部はopaqueとして扱います。

> **`zstdscope` libraryはv0.2.0を公開済みです。v0.3.0 project releaseでは、`zstdscope-cli` v0.3.0を初めてcrates.ioへ公開します。** Rust APIとCLI/JSONのcompatibility contractは別々に進化するため、package versionを独立して管理します。

**リンク:** [library crates.io](https://crates.io/crates/zstdscope) · [CLI crates.io](https://crates.io/crates/zstdscope-cli) · [docs.rs](https://docs.rs/zstdscope) · [Changelog](CHANGELOG.md) · [Releases](https://github.com/tappe9/zstdscope/releases)

## インストール

### Rust library

Rustプロジェクトへ追加する場合:

```bash
cargo add zstdscope
```

`Cargo.toml`へ直接追加する場合:

```toml
[dependencies]
zstdscope = "0.2"
```

Public inspection modelのSerde serializationを有効にする場合:

```bash
cargo add zstdscope --features serde
```

### CLI

CLI package名は`zstdscope-cli`、installされるbinary名は`zstdscope`です。

```bash
cargo install zstdscope-cli --version 0.3.0 --locked
```

crates.io releaseではなく現在のrepository revisionをinstallする場合:

```bash
cargo install --git https://github.com/tappe9/zstdscope zstdscope-cli --locked
```

正式なCLI release channelにはcrates.ioを採用します。GitHub Release向けprebuilt binary、署名、checksumは現在提供しません。これらは再現可能な自動release policyを別途定義した場合のみ追加します。

Source buildはGitHub-hosted runner上のUbuntu x86_64、Windows x86_64、macOS arm64で継続検証します。その他のRust対応targetはbest effortです。これはsource buildのsupport範囲であり、prebuilt binary提供を保証するものではありません。

LibraryとCLIは独立したpackage versionを使用します。v0.3 project releaseではparserとPublic Rust APIに変更がないため、`zstdscope-cli 0.3.0`は公開済みの`zstdscope 0.2.0` APIへ依存します。

## 解析する範囲

現在のstructural scopeでは以下を扱います。

- Standard Frameと全16種類のSkippable Frame magic
- 複数Frameの連結streamと正確なFrame境界
- Frame Header Descriptorとderived Window Size
- 全encoded widthのFrame Content Size
- Block-level decoded-size boundsから検証可能なFrame Content Size矛盾
- Dictionary IDのfieldなし・明示的な`0`・非0の区別
- Raw / RLE / Compressed Block Headerとencoded content span
- RLEのdeclared sizeと1 byteのencoded content sizeの区別
- 必須outer section headerを格納できないCompressed Blockのstructural rejection
- Content Checksumの格納値とspan（checksum validationは行わない）
- major fieldのzero-based source span
- malformed / truncated inputに対するlocation-aware typed error

Literals、Sequences、Huffman、FSEなどCompressed Block内部は解析しません。Parserは最低1つのcomplete Frameを要求し、入力全体をconsumeします。空入力、未知のtop-level magic、reserved encoding、不可能なstructural size、検証可能な矛盾、末尾のpartial Frameはerrorです。

## Rust library

主なAPIは次のとおりです。

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

使用例:

```rust
use zstdscope::{FrameKind, inspect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("sample.zst")?;
    let file = inspect(&bytes)?;

    for frame in &file.frames {
        match &frame.kind {
            FrameKind::Standard(standard) => {
                println!(
                    "frame #{}: standard, {} blocks, offset={}, size={}",
                    frame.index,
                    standard.blocks.len(),
                    frame.span.offset,
                    frame.span.length
                );
            }
            FrameKind::Skippable(skippable) => {
                println!(
                    "frame #{}: skippable variant {}, payload={} bytes",
                    frame.index,
                    skippable.variant,
                    skippable.declared_payload_size
                );
            }
        }
    }

    Ok(())
}
```

Public offsetはencoded input先頭からのzero-based byte offsetです。Opaque BlockとSkippable payloadは解析結果へcopyせず、spanで表現します。

### Resource limits

`inspect()`はFrame/Block件数に上限を設けない簡潔なAPIです。外部入力などにmetadata件数のbudgetを設ける場合は`inspect_with_limits()`を利用します。

```rust
use zstdscope::{InspectionLimits, inspect_with_limits};

let limits = InspectionLimits {
    max_frames: 1_024,
    max_blocks_per_frame: 2_048,
    max_total_blocks: 100_000,
};

let file = inspect_with_limits(&bytes, limits)?;
```

上記は例であり、すべての用途に対する安全なdefault値ではありません。設定値ちょうどまでは許可され、さらに1件を解析しようとした時点で、対象構造の開始offsetを持つ`ZstdError::ResourceLimitExceeded`を返します。

この制限はFrame/Block metadataの件数を制御するもので、callerが保持する入力sliceのbyte数を制限したりstreaming化したりするものではありません。

### Optional serialization

Core crateのSerdeはoptionalです。

```toml
[features]
default = []
serde = ["dep:serde"]
```

このfeatureで得られるPublic Rust modelのserialization表現と、CLI JSON wire contractは独立した契約です。Parseのみを行うconsumerにSerde dependencyは不要です。

## CLI

Human-readable output:

```text
zstdscope inspect sample.zst
```

Repository checkoutから実行する場合:

```text
cargo run -p zstdscope-cli -- inspect sample.zst
```

Frame typeと境界、Header metadata、Block type/size、Skippable payload metadata、格納されているChecksum metadataを表示します。

### 大きな入力ファイル

CLIはin-memoryのlibrary APIを利用するため、受け入れた入力ファイルは解析中にメモリ上へ保持されます。defaultのCLI動作をboundedにするため、`zstdscope inspect`は **268,435,456 bytes（256 MiB）** を超えるencoded inputをparse前に拒否します。

上限を明示的に変更する場合:

```text
zstdscope inspect large.zst --max-input-bytes 1073741824
```

上限を引き上げるとencoded input bufferへ許容する最大メモリ量も増えます。CLIは可能な場合は全体read前にfile sizeを確認し、実際のread自体にも上限を適用するため、read中にファイルが増加しても設定上限をsilently bypassしません。

CLIのbyte上限と、libraryの`inspect_with_limits()`によるmetadata件数上限は別のresource policyです。どちらもstreaming化するものではありません。

### Versioned JSON output

Machine-readable output:

```text
zstdscope inspect sample.zst --json
```

CLIはPublic Rust modelを直接serializeせず、専用JSON DTOを利用します。現在のwire contractは次のとおりです。

- top-levelに`"schema_version": 1`
- field名とenum値は明示的な`snake_case`
- Frame kindは`type` / `data`のtagged representation
- Rustの`u64`値（offset、length、input size、window size、Frame Content Sizeなど）はJavaScriptでinteger precisionを失わないdecimal string
- boundedな`u8`、`u32`、`usize`はJSON number
- Dictionary IDのfieldなしと明示的な`0`を保持
- RLEのdeclared sizeとencoded sizeを保持

1.0より前でも、既存consumerと互換なadditive fieldはschema version 1内で追加できます。fieldの削除・rename・type変更、enum/tag representation変更などのbreaking changeでは`schema_version`を更新し、release noteへ記録します。Human-readable outputは別契約であり、JSON DTO refactorの影響を受けません。

I/O error、parse error、input size limit超過は非0で終了し、diagnosticをstderrへ出力します。Partial-success JSONは出力しません。Output write failureでpanicせず、downstream processが通常どおりpipeを閉じた場合は正常終了として扱います。

詳細は[ADR 0005](docs/adr/0005-versioned-cli-json.md)、[ADR 0006](docs/adr/0006-cli-distribution.md)、[ADR 0007](docs/adr/0007-independent-package-versioning.md)を参照してください。

## 構成

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust Parser Library
│   └── zstdscope-cli/   # Public library APIを利用するCLI
├── docs/
└── ARCHITECTURE.md
```

Public API方針は[Public API設計](docs/API-DESIGN.md)で管理しています。

## ドキュメント

- [要件定義](docs/REQUIREMENTS.md)
- [アーキテクチャ](ARCHITECTURE.md)
- [Zstandardフォーマットメモ](docs/ZSTD-FORMAT.md)
- [Public API設計](docs/API-DESIGN.md)
- [Changelog](CHANGELOG.md)
- [Release手順](docs/RELEASING.md)
- [Supply-chain policy](docs/SUPPLY-CHAIN.md)
- [Fuzzing guide](FUZZING.md)
- [Roadmap](ROADMAP.md)
- [ADR](docs/adr/)

設計ドキュメントは英語を正式版として管理し、このREADMEは日本語で概要を確認するためのものとします。

## 開発・安全性方針

ZstdScopeは任意の入力byte列を信用しません。

- bounds-checked read/skip
- checked arithmetic
- malformed inputでparser panicしない
- opaque payloadを結果へcopyしない
- core crateでproject-authored `unsafe`を禁止
- Zstandard公開仕様を根拠に実装
- configurable metadata count limits
- CLI default 256 MiB encoded-input guard

Public parser APIは入力全体をメモリ上で扱います。`inspect_with_limits()`はmetadata件数を制限し、CLIは別途encoded-input byte上限を持ちます。`--max-input-bytes`で上限を変更できますが、受け入れた入力は依然として全体がメモリ上に存在します。Streaming/file-backed APIは後続milestoneです。

CIではformat、Clippy、test、rustdoc、MSRV、Ubuntu/Windows/macOS、package/publish dry-run、packaged CLI smoke、WebAssembly compile、advisory/license/source policyを検証します。詳細は[Supply-chain policy](docs/SUPPLY-CHAIN.md)と[SECURITY.md](SECURITY.md)を参照してください。

## ライセンス

ZstdScopeは以下のデュアルライセンスです。

- Apache License 2.0 (`LICENSE-APACHE`)
- MIT License (`LICENSE-MIT`)

利用者はどちらか一方を選択して利用・再配布できます。
