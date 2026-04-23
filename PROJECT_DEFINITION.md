# Project: my-interpreter

Open-Interpreter を参考に、よりシンプルで使いやすいインタープリターを再構築するプロジェクト。

## 1. 目的
- Open-Interpreter の複雑な依存関係や設定を排し、必要最小限の機能を持つ軽量なインタープリターを構築する。
- 実行環境の安全性と制御性を高める。

## 2. 作業工程 (Phase)
### Phase 1: 基礎調査 & 構造設計
- [ ] `references/open-interpreter` のコアロジック（コード実行、LLM連携、ストリーミング）の抽出
- [ ] `my-interpreter` の全体アーキテクチャの定義

### Phase 2: MVP (Minimum Viable Product) 実装
- [ ] LLM インターフェースの実装
- [ ] コード実行エンジン（Python等）の実装
- [ ] 基本的な CLI / インタラクティブ・ループの実装

### Phase 3: 機能拡張 & 洗練
- [ ] エラーハンドリングと自動修正機能
- [ ] ファイルシステム操作の安全性強化
- [ ] 出力フォーマットの最適化

## 3. 技術スタック案
- 言語: Python (or Rust if performance is critical)
- LLM API: OpenAI / Anthropic / Gemini (Google)

## 4. 進捗管理
- 完了したタスクは [x] でマークする。
