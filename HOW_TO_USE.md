# My-Interpreter 使用ガイド

このドキュメントでは、`my-interpreter` の導入、設定、および拡張方法について説明します。

## 1. クイックスタート

### ビルド
まずはプロジェクトをコンパイルします。
```bash
cargo build --release
```

### 実行
```bash
./target/release/my_interpreter
```
※デフォルトでは `MockLlm` が起動します。実際の LLM と連携するには下記の設定を行ってください。

## 2. 設定ガイド

### LLM の切り替え
`config.toml` を編集して、使用する LLM バックエンドを選択します。

```toml
[llm]
# "mock" または "openai_compatible" を指定
type = "openai_compatible"
model = "your-model-name"
base_url = "https://your-api-endpoint/v1"
```

### API キーの設定
`.env` ファイルを作成し、API キーを記述します。
```bash
LLM_API_KEY=sk-xxxx...
```

## 3. 基本的な操作
起動すると `User >` プロンプトが表示されます。

- **自然言語での指示**: 「カレントディレクトリのファイルサイズを合計して」のように入力します。
- **自動実行**: AI が `bash` コマンドを生成すると、自動的に実行され、その結果が AI にフィードバックされます。
- **終了**: `exit` または `quit` と入力します。

## 4. 開発者向け：機能の拡張

### 新しい LLM バックエンドの追加
`src/core/traits.rs` の `LlmBackend` トレイトを実装した新しい構造体を `src/plugins/` に作成してください。

```rust
#[async_trait]
impl LlmBackend for MyNewLlm {
    fn name(&self) -> &'static str { "MyNewLlm" }
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream, String> {
        // 実装...
    }
}
```

### 新しい実行エンジンの追加
`src/core/traits.rs` の `ExecutionEngine` トレイトを実装することで、Bash 以外の環境（Pythonインタープリタの直接呼び出し、Docker コンテナ、Wasm 実行環境など）をサポートできます。

```rust
#[async_trait]
impl ExecutionEngine for MyCustomEngine {
    // 実装...
}
```

## 5. 設計思想の核
このツールは **「AI に人間（あなた）の判断基準を学習させる」** ことを最終目標としています。
日々の対話を通じて、AI があなたの好むコードスタイルや安全基準を学習し、最終的には最小限の確認だけで高度なタスクを完遂する「デジタルツイン」としての動作を目指しています。
