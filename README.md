# Lasada (ラサダ)

**Lasada** は、Rust で開発された次世代の AI エージェント・インタープリターです。  
Open-Interpreter の設計思想を継承しつつ、Rust の持つ圧倒的なパフォーマンスとメモリ安全性、そして独自のシンボリック状態管理により、複雑なタスクを高速かつ堅牢に遂行します。

## 概要

Lasada は、単なる「チャットボット」ではなく、ユーザーの哲学と意図を理解し、OS 資源を直接操作して価値を創出する **「実行エンジン」** です。Python 依存の肥大化した環境から脱却し、シングルバイナリで動作する軽量かつ強力な Rust ベースのアーキテクチャを採用しています。

## 主な特徴

- **Rust-Powered High Performance**:  
  全機能を Rust で実装。高い実行速度とメモリ安全性を保証し、Open-Interpreter の高パフォーマンスな代替手段として機能します。
- **プラグイン・アーキテクチャ**:  
  トレイトベースの設計により、多様な実行エンジンをシームレスに統合。
  - **Bash**: 状態保持型シェル実行（システムの直接制御）。
  - **Python**: 隔離された環境でのスクリプト実行。
  - **Web**: 高速なウェブ検索とスクレイピング、情報のリアルタイム抽出。
  - **Computer (Computer Use)**: GUI 操作、画面情報の解析。
- **Vision Support (視覚支援)**:  
  画面解析機能を標準搭載。グリッドオーバーレイ（Visual Grid）により、GUI 上の座標を直感的に認識し、精密な操作を実現します。
- **Local RAG (長期記憶)**:  
  `fastembed` を活用したローカル・ベクトルデータベースを内蔵。過去の対話履歴や外部ドキュメントを自動的にインデックス化し、文脈に応じた最適な情報を抽出します。
- **Symbolic Context (.vlog)**:  
  高密度な状態管理フォーマット `.vlog` を採用。AI の振る舞いを「指示」ではなく「制約と状態遷移」として定義し、一貫性のある高度な推論を可能にします。

## アーキテクチャ

1.  **Core Interpreter**: 全体のワークフロー（対話、RAG、状態管理）を統合制御。
2.  **Plugin Dispatcher**: コマンドの種類に応じて最適なプラグイン（Executor）へ処理を委譲。
3.  **Context Manager**: L1（短期記憶）、L2（ベクトルDBによる長期記憶）、および `.vlog`（シンボリック状態）を同期。
4.  **LLM Connector**: OpenAI 互換 API などの各種バックエンドに対応。

## セットアップ

### 必要条件
- [Rust](https://www.rust-lang.org/) (Cargo, Edition 2024 以上)
- オプション: `xdotool`, `scrot` (Computer Use 用)

### インストール
```bash
git clone https://github.com/kubodad/lasada.git
cd lasada
./install.sh
```

## 使い方

```bash
# 基本起動
lasada

# デバッグモード（詳細ログ出力）
lasada --debug

# 自動実行モード（コマンド確認をスキップ）
lasada --auto-run
```

## 開発哲学 (System Philosophy)

Lasada は **HV-CAD (Human-Value Centric Autonomous Development)** の原則に基づき設計されています。
AI を「確率分布の操作対象」として定義し、人間が「価値判断の独占者」として介入することで、最小限の監視で最大の成果を生み出す「デジタルツイン」の構築を目指しています。

## ライセンス
Apache License 2.0
