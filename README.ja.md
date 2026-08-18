# ZstdScope

ZstdScope は、Zstandard圧縮データ形式の構造を解析・可視化するための **Pure Rust製Inspector / Parserツールキット**を目指すOSSプロジェクトです。

このプロジェクトの主目的は圧縮・展開ではなく、`.zst`などのZstandardストリームに含まれるFrame、Header、Block、offset、sizeなどの構造情報を安全に取得できるAPIを提供することです。

> 現在、v0.1のStructural Inspector、human-readable CLI、JSON CLIまで実装済みです。pre-1.0のためPublic APIとJSON schemaは今後意図的に変更される可能性があります。

## v0.1で解析する範囲

- Standard Zstandard magic number
- 全16種類のSkippable Frame magic number
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

また、Content Checksumは格納値と位置を取得しますが、復元データに対する検証は行いません。圧縮・展開機能もZstdScopeの目的外です。

v0.1は**実装している構造境界についてstrict**です。空入力・途中で切れたデータ・未知のtop-level magic・reserved bit/type・不可能なblock size・検証可能なFrame Content Size矛盾・末尾の不完全なFrameはエラーとします。一方、opaqueとして扱うCompressed Block内部やContent Checksumの正当性まで検証済みであるとは扱いません。

## 構成

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust Parser Library
│   └── zstdscope-cli/   # CLI
├── docs/
└── ARCHITECTURE.md
```

Public APIは次の形です。

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

Public API方針は[Public API設計](docs/API-DESIGN.md)で管理しています。

## CLI

```text
zstdscope inspect sample.zst
zstdscope inspect sample.zst --json
```

I/O errorやparse errorは非0で終了し、diagnosticをstderrへ出力します。pipe先が通常どおり先に閉じた場合のbroken pipeはpanicさせず正常終了として扱います。

v0.1のcrate releaseでは再利用可能な`zstdscope` libraryをcrates.io向けpackage対象とします。`zstdscope-cli` package自体は現在`publish = false`で、CLI配布方法は将来のGitHub Release等で別途整備します。

## JSON

CLIでは`--json`によるmachine-readableな解析結果を提供します。

JSONのfield名とenum値は`snake_case`を使用し、serialization libraryのdefault挙動に偶然依存しないよう、表現を明示してテストしています。Dictionary IDの「fieldなし」と「明示的な0」、RLEのdeclared sizeとencoded sizeなど、Inspector固有の情報も保持します。

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

v0.1のCLIは入力ファイル全体をメモリへ読み込み、parserもframe/block metadataをeagerに保持します。非常に大きいファイルや大量metadataを含む入力向けのresource limit、streaming/file-backed APIは後続milestoneで整備します。

## ライセンス

ZstdScopeは以下のデュアルライセンスです。

- Apache License 2.0 (`LICENSE-APACHE`)
- MIT License (`LICENSE-MIT`)

利用者はどちらか一方を選択して利用・再配布できます。
