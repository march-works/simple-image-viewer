# 開発環境と本番環境の分離 実行計画

## 背景・課題

アプリを実用しながら開発を行う際、以下の問題が発生していた：

1. **シングルインスタンス制約**: 本番版が起動中だと開発版が起動できない（逆も同様）
2. **state.json の共有**: 開発版と本番版で状態ファイルが共有され、開発中の変更が本番環境に影響する

## 解決方針

Tauri の設定マージ機能（`--config` フラグ）と Rust の `cfg!(debug_assertions)` を組み合わせて、開発環境と本番環境を完全に分離する。

### 分離される項目

| 項目 | 開発 (`tauri dev`) | 本番 (リリースビルド) |
|------|-------------------|----------------------|
| App Identifier | `com.simple-image-viewer.march.dev` | `com.simple-image-viewer.march` |
| シングルインスタンス | 開発版同士で共有 | 本番版同士で共有 |
| Viewer タイトル | `Simple Image Viewer [DEV]` | `Simple Image Viewer` |
| Explorer タイトル | `Image Explorer [DEV]` | `Image Explorer` |
| 状態ファイル | `{AppData}/simple-image-viewer-dev/state.json` | `{AppData}/simple-image-viewer/state.json` |

## 実装計画

### Step 1: 開発用設定ファイルの作成

**ファイル**: `src-tauri/tauri.dev.conf.json`

```json
{
  "identifier": "com.simple-image-viewer.march.dev",
  "productName": "simple-image-viewer-dev",
  "app": {
    "windows": [
      {
        "label": "viewer-0",
        "title": "Simple Image Viewer [DEV]"
      }
    ]
  }
}
```

**効果**:
- 開発用の異なる `identifier` により、シングルインスタンスのミューテックス/ソケット名が本番と分離される
- 初期ウィンドウのタイトルが `[DEV]` 付きになる

### Step 2: npm scripts の更新

**ファイル**: `package.json`

```json
{
  "scripts": {
    "tauri:dev": "tauri dev --config src-tauri/tauri.dev.conf.json"
  }
}
```

**使い方**:
- 開発時: `pnpm tauri:dev`（開発用識別子を使用）
- 従来通り: `pnpm tauri dev`（本番用識別子を使用、テスト用途）

### Step 3: 状態ファイルパスの分離

**ファイル**: `src-tauri/src/app/mod.rs`

ヘルパー関数を追加:

```rust
/// Returns the app directory name based on build mode
fn get_app_dir_name() -> &'static str {
    if cfg!(debug_assertions) {
        "simple-image-viewer-dev"
    } else {
        "simple-image-viewer"
    }
}
```

状態ファイルの読み込み・保存箇所で使用:

```rust
// 読み込み時
let save_path = save_dir.join(get_app_dir_name()).join("state.json");

// 保存時
let app_dir = dir.join(get_app_dir_name());
```

### Step 4: ウィンドウタイトルの分岐

**ファイル**: `src-tauri/src/app/mod.rs`

ヘルパー関数を追加:

```rust
/// Returns the viewer window title based on build mode
fn get_viewer_title() -> &'static str {
    if cfg!(debug_assertions) {
        "Simple Image Viewer [DEV]"
    } else {
        "Simple Image Viewer"
    }
}

/// Returns the explorer window title based on build mode
fn get_explorer_title() -> &'static str {
    if cfg!(debug_assertions) {
        "Image Explorer [DEV]"
    } else {
        "Image Explorer"
    }
}
```

動的に作成されるウィンドウで使用:

```rust
// Viewer ウィンドウ
.title(get_viewer_title())

// Explorer ウィンドウ
.title(get_explorer_title())
```

## 変更対象ファイル一覧

1. `src-tauri/tauri.dev.conf.json` - 新規作成
2. `package.json` - scripts に `tauri:dev` を追加
3. `src-tauri/src/app/mod.rs` - ヘルパー関数追加、パス・タイトルの分岐

## 動作確認手順

1. 本番版をインストール・起動
2. `pnpm tauri:dev` で開発版を起動
3. 両方が同時に起動できることを確認
4. 開発版のウィンドウタイトルに `[DEV]` が付いていることを確認
5. 開発版で状態を変更し、本番版に影響がないことを確認
6. `%APPDATA%/simple-image-viewer-dev/state.json` が作成されることを確認

## 注意事項

- `pnpm tauri dev`（従来コマンド）は本番用識別子を使用するため、本番版と競合する
- 開発時は `pnpm tauri:dev` を使用すること
- state.json のスキーマが変わった場合、開発用・本番用それぞれで初期化が必要
