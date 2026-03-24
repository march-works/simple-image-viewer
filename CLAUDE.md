# CLAUDE.md - Simple Image Viewer

このファイルは Claude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供します。

## プロジェクト概要

Simple Image Viewer は、画像・動画・ZIP 内画像を閲覧するためのデスクトップアプリケーションです。

- **フレームワーク**: Tauri v2 (Rust) + SolidJS (TypeScript)
- **対象 OS**: Windows / macOS / Linux
- **アーキテクチャ**: マルチウィンドウ (Viewer + Explorer)

## 開発コマンド

```bash
# 開発サーバー起動（開発用識別子、本番版と分離）
pnpm tauri:dev

# 本番ビルド
pnpm tauri build

# リント
pnpm lint

# フォーマット（フロントエンド + バックエンド）
pnpm format

# フロントエンドのみフォーマット
pnpm format-web

# バックエンドのみフォーマット
pnpm format-back
```

## プロジェクト構造

```
simple-image-viewer/
├── src/                    # フロントエンド (SolidJS + TypeScript)
│   ├── pages/              # ページコンポーネント (Viewer, Explorer)
│   ├── features/           # 機能別モジュール
│   └── components/         # 共通UIコンポーネント
├── src-tauri/              # バックエンド (Rust)
│   └── src/
│       ├── app/            # Tauriコマンド層
│       ├── service/        # 状態管理層
│       └── utils/          # ユーティリティ層
└── docs/                   # 技術ドキュメント
```

## アーキテクチャ原則

### バックエンド 3 層構造

| レイヤー | 責務 | 禁止事項 |
|---------|------|----------|
| `app/` | `#[tauri::command]` エントリーポイント、`AppHandle` 操作、イベント emit | 直接の状態変更 |
| `service/` | `AppState` の操作、ビジネスロジック、状態のロック管理 | `AppHandle` 操作、emit 処理 |
| `utils/` | 汎用ヘルパー関数 | 状態操作、ウィンドウ操作 |

### フロントエンド構造

- **pages/**: ルートレベルのページコンポーネント（タブ管理含む）
- **features/**: 機能単位のモジュール（DirectoryTree, Image, Explorer, Folder）
- **components/**: 再利用可能な共通 UI コンポーネント

## 重要な規約

詳細なコーディング規約は以下を参照:

- [.claude/rules/architecture.md](.claude/rules/architecture.md) - レイヤー分離原則
- [.claude/rules/rust-backend.md](.claude/rules/rust-backend.md) - Rust コーディング規約
- [.claude/rules/solidjs-frontend.md](.claude/rules/solidjs-frontend.md) - SolidJS/TypeScript 規約
- [.claude/rules/tauri-ipc.md](.claude/rules/tauri-ipc.md) - Tauri IPC 通信パターン

既存の GitHub Copilot 向けドキュメント:
- [.github/copilot-instructions.md](.github/copilot-instructions.md)

## 技術スタック

### フロントエンド
- SolidJS v1.9.x + @solidjs/router
- TailwindCSS v4.x
- Vite v7.x + vite-plugin-solid
- TypeScript v5.x
- ts-pattern (パターンマッチング)
- video.js (動画再生)

### バックエンド
- Rust 2021 Edition
- Tauri v2.x
- tokio (非同期ランタイム)
- zip (ZIPファイル処理)
- notify (ファイル監視)
- tauri-plugin-store (状態永続化)
- tauri-plugin-single-instance (シングルインスタンス)

## サポートするファイル形式

### 画像
JPEG, PNG, GIF, TIFF, WebP, BMP, ICO, SVG, AVIF

### 動画
MP4, WebM, OGG, MOV, AVI, MKV, WMV

### 圧縮ファイル
ZIP

## 開発環境と本番環境の分離

| 項目 | 開発 (`pnpm tauri:dev`) | 本番 |
|------|------------------------|------|
| App Identifier | `com.simple-image-viewer.march.dev` | `com.simple-image-viewer.march` |
| ウィンドウタイトル | `[DEV]` 付き | 通常 |
| 状態ファイル | `simple-image-viewer-dev/` | `simple-image-viewer/` |

詳細: [docs/dev-prod-environment-separation.md](docs/dev-prod-environment-separation.md)
