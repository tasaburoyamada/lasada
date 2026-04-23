# Lasada

Rust で実装された、堅牢かつ拡張性の高い AI インタープリター。
Open-Interpreter の設計思想を継承しつつ、Python への依存を排除し、高いパフォーマンスと安全な実行環境を提供することを目指しています。

## 特徴

- **純 Rust 実装**: 高い実行速度とメモリ安全性。
- **脱 Python**: システムの `bash` を直接制御。Python ランタイムは不要です。
- **プラグイン・アーキテクチャ**:
  - `LlmBackend`: OpenAI 互換 API や自社製 LLM、テスト用 Mock などに柔軟に対応。
  - `ExecutionEngine`: 現在は Bash をサポート。将来的に Wasm や Docker への拡張が可能。
- **インタラクティブな UI**: `colored` による色分けと `indicatif` によるプログレス表示。
- **永続的なセッション**: `BashExecutor` により、同一対話内でのディレクトリ移動 (`cd`) や変数の保持が可能。

## アーキテクチャ

システムは以下の 3 つのコアコンポーネントで構成されています。

1. **Core**: `Interpreter` が全体の流れを制御。LLM と実行エンジンの仲介を行います。
2. **Traits**: `LlmBackend` および `ExecutionEngine` を定義。
3. **Plugins**: 具体的な実装（`OpenAICompatibleLlm`, `BashExecutor`, `MockLlm`）。

## セットアップ

### 必要条件
- [Rust](https://www.rust-lang.org/) (Cargo)

### インストール
```bash
./install.sh
```
これにより、バイナリが `~/.local/bin/lasada` に、設定ファイルが `~/.config/lasada/config.toml` に配置されます。

## 設定方法

`config.toml` および環境変数で動作をカスタマイズできます。

### 1. config.toml
プロジェクトルートに `config.toml` を配置します。

```toml
[llm]
type = "openai_compatible" # または "mock"
model = "your-model-name"
base_url = "https://your-api-endpoint/v1"

[system]
prompt = "あなたはエンジニアを支援するエキスパートAIです..."
```

### 2. 環境変数
API キーなどの機密情報は環境変数または `.env` ファイルに記述します。

```bash
LLM_API_KEY=your_secret_key
```

## 使い方

```bash
cargo run
```

起動後、プロンプトに指示を入力してください。
例:
- `今いるディレクトリのファイル一覧を見せて`
- `現在の時刻を表示して`

## ライセンス
MIT License
