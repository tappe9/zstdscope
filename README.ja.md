# ZstdScope

ZstdScope は、Zstandard圧縮データ形式の構造を解析・可視化するための **Pure Rust製Inspector/Parserツールキット**を目指すOSSプロジェクトです。

このプロジェクトの主目的は圧縮・展開ではなく、`.zst`などのZstandardストリームに含まれるFrame、Header、Block、offset、sizeなどの構造情報を安全に取得できるAPIを提供することです。

> 現在は設計フェーズです。Parser本体の実装はまだ含まれていません。

## 目標

ZstdScopeでは次を目指します。

- Zstandardデータを展開せずに構造を解析する
- Standard FrameとSkippable Frameを識別する
- Frame HeaderやBlock Headerの各フィールドを取得する
- 各構造のbyte offsetとencoded sizeを取得する
- 壊れた入力に対してpanicせず、位置情報付きの型付きエラーを返す
- CLIとParser coreを分離し、他のRustプログラムから再利用できるようにする
- 将来のWebAssembly / Web Inspectorに対応しやすい設計にする

## v0.1で予定している解析範囲

- Standard Zstandard magic number
- Skippable Frame magic number
- Frame Header Descriptor
- Window Size
- Frame Content Size
- Dictionary ID
- Content Checksumの有無と格納値
- Block Header
- Raw / RLE / Compressed Block
- Last Block Flag
- Frame / Blockのbyte offsetとencoded size
- 複数Frameの連結ストリーム

v0.1ではCompressed Block内部の以下は解析しません。

- Literals
- Sequences
- Huffman
- FSE

また、圧縮・展開機能はZstdScopeの目的外です。

## 想定構成

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust Parser Library
│   └── zstdscope-cli/   # CLI
├── docs/
└── ARCHITECTURE.md
```

Public APIは、まず次のようなシンプルな形から開始する案です。

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

APIは設計中であり、まだ確定していません。

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

## 開発方針

ZstdScopeでは任意の入力byte列を信用しません。

Parserでは特に以下を重視します。

- bounds check
- checked arithmetic
- malformed inputでpanicしない
- 不正なsizeによる過剰allocationを防ぐ
- v0.1ではproject codeに`unsafe`を使用しない
- Zstdの公開仕様を根拠に実装する

## ライセンス

プロジェクトのライセンスはまだ決定していません。最初の配布可能なリリースまでに明示的に決定します。
