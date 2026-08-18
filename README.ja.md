# ZstdScope

ZstdScope は、Zstandard圧縮データ形式の構造を解析・可視化するための **Pure Rust製Inspector / Parserツールキット**を目指すOSSプロジェクトです。

このプロジェクトの主目的は圧縮・展開ではなく、`.zst`などのZstandardストリームに含まれるFrame、Header、Block、offset、sizeなどの構造情報を安全に取得できるAPIを提供することです。

> 現在、v0.1の要件・アーキテクチャ・Public API方針は設計確定済みです。Parser本体の実装はこれから開始します。

## 目標

ZstdScopeでは次を目指します。

- Zstandardデータを展開せずに構造を解析する
- Standard FrameとSkippable Frameを識別する
- Frame HeaderやBlock Headerの各フィールドを取得する
- 各構造・フィールドのbyte offset / spanとencoded sizeを取得する
- Inspectorとして意味のあるbyte表現上の違いを保持する
- 壊れた入力に対して位置情報付きの型付きエラーを返す
- CLIとParser coreを分離し、他のRustプログラムから再利用できるようにする
- 将来のWebAssembly / Web Inspectorに対応しやすい設計にする

## v0.1で予定している解析範囲

- Standard Zstandard magic number
- Skippable Frame magic number
- Frame Header Descriptor
- Window Size
- Frame Content Size
- Dictionary ID
  - フィールドなし
  - 明示的に`0`が格納されている
  - 非0のID
  を区別する
- Content Checksumの有無と格納値
- Block Header
- Raw / RLE / Compressed Block
- Last Block Flag
- Frame / Header field / Blockのsource spanとencoded size
- 複数Frameの連結ストリーム

v0.1ではCompressed Block内部の以下は解析しません。

- Literals
- Sequences
- Huffman
- FSE

また、圧縮・展開機能はZstdScopeの目的外です。

v0.1はstrict parserとし、空入力・途中で切れたデータ・未知のtop-level magic・末尾の不完全なFrameはエラーとします。

## 想定構成

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust Parser Library
│   └── zstdscope-cli/   # CLI
├── docs/
└── ARCHITECTURE.md
```

Public APIは、まず次の形から開始します。

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

v0.1のAPI方針は[Public API設計](docs/API-DESIGN.md)で確定しています。ただしプロジェクトはpre-1.0のため、今後も意図的なbreaking changeが発生する可能性があります。

## JSON

CLIでは`--json`によるmachine-readableな解析結果を提供します。

JSONのfield名とenum値は`snake_case`を使用し、serialization libraryのdefault挙動に偶然依存しないよう、表現を明示してテストします。

## ドキュメント

- [要件定義](docs/REQUIREMENTS.md)
- [アーキテクチャ](ARCHITECTURE.md)
- [Zstandardフォーマットメモ](docs/ZSTD-FORMAT.md)
- [Public API設計](docs/API-DESIGN.md)
- [Roadmap](ROADMAP.md)
- [ADR](docs/adr/)

設計ドキュメントは英語を正式版として管理し、このREADMEは日本語で概要を確認するためのものとします。

## 仕様の参照先

- [RFC 8878](https://www.rfc-editor.org/rfc/rfc8878.html)
- [Zstandard reference format specification](https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md)
- [Zstandard reference implementation](https://github.com/facebook/zstd)

## 開発・安全性方針

ZstdScopeでは任意の入力byte列を信用しません。

Parserでは特に以下を重視します。

- bounds check
- checked arithmetic
- malformed inputでparser panicしない
- 不正なsizeを理由にopaque payloadをコピーしない
- v0.1ではproject codeに`unsafe`を使用しない
- Zstdの公開仕様を根拠に実装する

## ライセンス

ZstdScopeは以下のデュアルライセンスです。

- Apache License 2.0 (`LICENSE-APACHE`)
- MIT License (`LICENSE-MIT`)

利用者はどちらか一方を選択して利用・再配布できます。
