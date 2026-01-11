# GitHub Copilot Instructions for Simple Image Viewer

## プロジェクト概要

Simple Image Viewer は、画像・動画・ZIP 内画像を閲覧するためのデスクトップアプリケーションです。
Tauri + SolidJS + TypeScript + Rust で構築されています。

## 技術スタック

### フロントエンド

- **フレームワーク**: SolidJS (v1.7.3) - React に似たリアクティブ UI フレームワーク
- **ルーティング**: @solidjs/router (v0.8.2)
- **スタイリング**: TailwindCSS (v3.3.1)
- **ビルドツール**: Vite (v3.2.6) + vite-plugin-solid
- **言語**: TypeScript (v4.9.5)
- **パターンマッチング**: ts-pattern (v4.2.2)
- **動画再生**: video.js (v8.3.0)

### バックエンド (Tauri / Rust)

- **Tauri**: v1.6.2 (デスクトップアプリケーションフレームワーク)
- **Rust Edition**: 2021
- **gRPC**: tonic (v0.11.0) + prost (v0.12.4)
- **非同期ランタイム**: tokio (v1.37.0)
- **ZIP ファイル処理**: zip (v1.1.3)
- **ファイル監視**: notify (v6.1.1)
- **自然順ソート**: natord (v1.0.9)
- **状態永続化**: tauri-plugin-store

## プロジェクト構造

```
simple-image-viewer/
├── src/                          # フロントエンド (SolidJS)
│   ├── components/               # 共通コンポーネント
│   │   └── Pagination/           # ページネーションコンポーネント
│   ├── features/                 # 機能別モジュール
│   │   ├── DirectoryTree/        # ディレクトリツリー表示
│   │   │   ├── components/       # DirectoryNode, ImageNode, VideoNode, ZipNode
│   │   │   ├── routes/           # PathSelection
│   │   │   └── types/            # 型定義
│   │   ├── Folder/               # フォルダ関連機能
│   │   └── Image/                # 画像表示機能
│   │       └── ImageCanvas/      # 画像キャンバス (ズーム・パン機能)
│   ├── pages/                    # ページコンポーネント
│   │   ├── explorer/             # エクスプローラーウィンドウ
│   │   └── viewer/               # ビューアーウィンドウ
│   └── assets/                   # 静的アセット
├── src-tauri/                    # バックエンド (Rust)
│   ├── src/
│   │   ├── app/                  # アプリケーションロジック
│   │   │   ├── explorer.rs       # エクスプローラー機能
│   │   │   └── viewer.rs         # ビューアー機能
│   │   ├── grpc/                 # gRPC サーバー (プロセス間通信)
│   │   │   ├── add_tab.rs        # タブ追加サービス
│   │   │   ├── new_window.rs     # 新規ウィンドウサービス
│   │   │   └── server.rs         # gRPCサーバー起動
│   │   ├── service/              # サービス層
│   │   │   └── app_state.rs      # アプリケーション状態管理
│   │   └── utils/                # ユーティリティ
│   │       └── file_utils.rs     # ファイル操作ユーティリティ
│   └── proto/                    # Protocol Buffers定義
│       ├── add_tab.proto
│       └── new_window.proto
├── index.html                    # Viewerエントリーポイント
├── explorer.html                 # Explorerエントリーポイント
└── vite.config.ts                # Vite設定 (マルチページ設定)
```

## 主要な機能

### Viewer (メインウィンドウ)

- 画像/動画ファイルの閲覧
- タブによる複数ファイル管理
- ZIP ファイル内の画像閲覧
- ズーム・パン操作 (Ctrl+I/O, マウスホイール)
- キーボードナビゲーション

### Explorer (エクスプローラーウィンドウ)

- ディレクトリツリー表示
- サムネイル表示
- フォルダ間のファイル転送機能
- ページネーション

## コーディング規約

### TypeScript/SolidJS

- コンポーネントは関数コンポーネントで記述
- `createSignal`, `createEffect`, `createMemo`などの SolidJS プリミティブを使用
- パターンマッチングには`ts-pattern`ライブラリを使用
- Tauri との通信には`@tauri-apps/api`の invoke を使用

### Rust

- エラーハンドリングには`anyhow`を使用
- 非同期処理には`tokio`を使用
- Tauri コマンドは`#[tauri::command]`アトリビュートで定義

## ファイル形式のサポート

### 画像

- JPEG (jpg, jpeg, jpe, jfif, pjpeg, pjp)
- PNG
- GIF
- TIFF (tif, tiff)
- その他 (webp, bmp, ico, svg, avif)

### 動画

- MP4, WebM, OGG, MOV, AVI, MKV, WMV

### 圧縮ファイル

- ZIP

## 開発コマンド

```bash
# 開発サーバー起動
pnpm tauri dev

# ビルド
pnpm tauri build

# リント
pnpm lint

# フォーマット
pnpm format

# フロントエンドのみフォーマット
pnpm format-web

# バックエンドのみフォーマット
pnpm format-back
```

## Tauri IPC (invoke)

### Viewer 関連

- `open_new_viewer_tab` - 新規ビューアータブを開く
- `change_active_viewer_tab` - アクティブタブを変更
- `remove_viewer_tab` - タブを閉じる
- `change_viewing` - 表示中のファイルを変更
- `move_forward` / `move_backward` - 前後のファイルに移動
- `open_image_dialog` - 画像選択ダイアログを開く
- `read_image_in_zip` - ZIP 内の画像を読み込み
- `subscribe_dir_notification` - ディレクトリ変更を監視

### Explorer 関連

- `open_new_explorer` - 新規エクスプローラーを開く
- `open_new_explorer_tab` - 新規タブを追加
- `change_explorer_path` - パスを変更
- `transfer_folder` - フォルダを転送
- `move_explorer_forward` / `move_explorer_backward` - 履歴ナビゲーション

## 注意事項

- マルチウィンドウ対応: Viewer と Explorer は別ウィンドウで動作
- 状態管理: アプリケーション状態は Rust 側で管理し、tauri-plugin-store で永続化
- gRPC サーバー: ポート 50052 で起動、プロセス間通信に使用
- 自動アップデート: Tauri Updater を使用、GitHub Gist 経由で配信

## JSX の記法

SolidJS を使用しているため、以下の点に注意:

- `className`ではなく`class`を使用
- `onClick`などのイベントハンドラはキャメルケース
- 条件付きレンダリングには`<Show when={...}>`を使用
- リストレンダリングには`<For each={...}>`を使用
- パターンマッチングには`<Switch>/<Match>`または`ts-pattern`を使用
