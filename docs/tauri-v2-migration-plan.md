# Tauri v2 マイグレーション計画

## 概要

Simple Image ViewerをTauri v1.6.2からTauri v2.xへアップデートする計画書です。

## 現状のバージョン

### フロントエンド (package.json)

| パッケージ | 現在のバージョン |
|---|---|
| @tauri-apps/api | ^1.2.0 |
| @tauri-apps/cli | ^1.2.3 |
| solid-js | ^1.7.3 |
| @solidjs/router | ^0.8.2 |
| vite | ^3.2.6 |
| typescript | ^4.9.5 |
| tailwindcss | ^3.3.1 |
| ts-pattern | ^4.2.2 |
| video.js | ^8.3.0 |
| tauri-plugin-store-api | github:tauri-apps/tauri-plugin-store (v1) |

### バックエンド (Cargo.toml)

| クレート | 現在のバージョン |
|---|---|
| tauri | 1.6.2 |
| tauri-build | 1.5.1 |
| tauri-plugin-store | git (v1 branch) |
| tokio | 1.37.0 |
| tonic | 0.11.0 |
| prost | 0.12.4 |

## アップデート方針

### 決定事項

1. **Updater形式**: v2形式に完全移行（v1互換モードは使用しない）
2. **LocalStorage/IndexedDB**: 現在使用していないため、origin URL変更への対応（useHttpsScheme）は不要
3. **段階的アップデート**: Tauri v2移行を先行し、Vite/TypeScriptのアップデートは後続フェーズで実施

---

## Phase 1: Tauri v2 マイグレーション

### 1.1 事前準備

- [ ] 作業ブランチを作成 (`feature/tauri-v2-migration`)
- [ ] 現在の動作確認

### 1.2 自動マイグレーションの実行

```bash
# CLIを最新版に更新
pnpm add -D @tauri-apps/cli@latest

# 自動マイグレーション実行
pnpm tauri migrate
```

このコマンドで以下が自動的に更新される:
- `tauri.conf.json` の構造変更
- `Cargo.toml` の依存関係更新
- `package.json` の依存関係更新
- `src-tauri/capabilities/` ディレクトリの生成

### 1.3 設定ファイルの変更点

#### tauri.conf.json の主な変更

| v1 | v2 |
|---|---|
| `package.productName` | `productName` (トップレベル) |
| `package.version` | `version` (トップレベル) |
| `tauri.bundle` | `bundle` (トップレベル) |
| `tauri.windows` | `app.windows` |
| `tauri.allowlist` | 削除 → `capabilities/` に移行 |
| `tauri.updater` | `plugins.updater` |
| `tauri.cli` | `plugins.cli` |

#### Permissions (ACL) システム

v1の`allowlist`は廃止され、`src-tauri/capabilities/`ディレクトリでの宣言的なPermissionsシステムに移行:

```json
// src-tauri/capabilities/default.json (例)
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["*"],
  "permissions": [
    "core:default",
    "fs:default",
    "dialog:default",
    "cli:default",
    "store:default"
  ]
}
```

### 1.4 プラグインの更新

#### tauri-plugin-store

**フロントエンド:**
```diff
- "tauri-plugin-store-api": "github:tauri-apps/tauri-plugin-store"
+ "@tauri-apps/plugin-store": "^2.0.0"
```

**バックエンド (Cargo.toml):**
```diff
- tauri-plugin-store = { git = "https://github.com/tauri-apps/plugins-workspace", branch = "v1" }
+ tauri-plugin-store = "2"
```

**Rustコード:**
```diff
- use tauri_plugin_store::Store;
+ use tauri_plugin_store::StoreExt;
```

#### CLI Plugin (新規追加が必要な場合)

**バックエンド (Cargo.toml):**
```toml
tauri-plugin-cli = "2"
```

### 1.5 JavaScript APIの変更

| v1 | v2 |
|---|---|
| `@tauri-apps/api/tauri` | `@tauri-apps/api/core` |
| `import { invoke } from '@tauri-apps/api'` | `import { invoke } from '@tauri-apps/api/core'` |
| `@tauri-apps/api/window` | `@tauri-apps/api/webviewWindow` |
| `appWindow` | `getCurrentWebviewWindow()` |

#### 主な修正箇所

**src/pages/viewer/Viewer.tsx:**
```diff
- import { appWindow } from '@tauri-apps/api/window';
- import { invoke } from '@tauri-apps/api';
+ import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
+ import { invoke } from '@tauri-apps/api/core';

+ const appWindow = getCurrentWebviewWindow();
```

**src/features/Image/ImageCanvas/index.tsx:**
```diff
- import { convertFileSrc, invoke } from '@tauri-apps/api/tauri';
+ import { convertFileSrc, invoke } from '@tauri-apps/api/core';
```

### 1.6 Rust側の変更

| v1 | v2 |
|---|---|
| `tauri::Window` | `tauri::WebviewWindow` |
| `Manager::get_window()` | `Manager::get_webview_window()` |
| `WindowBuilder` | `WebviewWindowBuilder` |

#### Cargo.toml features の変更

```diff
[dependencies]
- tauri = { version = "1.6.2", features = ["api-all", "cli", "updater"] }
+ tauri = { version = "2", features = [] }
+ tauri-plugin-cli = "2"
+ tauri-plugin-store = "2"

[build-dependencies]
- tauri-build = { version = "1.5.1", features = [] }
+ tauri-build = { version = "2", features = [] }
```

### 1.7 Updater の移行

v2ではUpdaterがプラグインに移行:

**Cargo.toml:**
```toml
tauri-plugin-updater = "2"
```

**main.rs / lib.rs:**
```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        // ...
}
```

**tauri.conf.json:**
```json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://gist.githubusercontent.com/..."],
      "pubkey": "..."
    }
  },
  "bundle": {
    "createUpdaterArtifacts": true
  }
}
```

### 1.8 動作確認チェックリスト

- [ ] `pnpm tauri dev` でビルドが通る
- [ ] Viewerウィンドウが正常に起動する
- [ ] Explorerウィンドウが正常に起動する
- [ ] 画像の表示が正常に動作する
- [ ] 動画の再生が正常に動作する
- [ ] ZIPファイル内の画像表示が動作する
- [ ] タブの追加・削除が動作する
- [ ] ファイルダイアログが動作する
- [ ] ディレクトリツリーが表示される
- [ ] フォルダ転送機能が動作する
- [ ] 状態の永続化 (store) が動作する
- [ ] `pnpm tauri build` でプロダクションビルドが成功する

---

## Phase 2: Vite & TypeScript アップデート (後続フェーズ)

Phase 1完了後、別途実施予定:

| パッケージ | 現在 | 目標 |
|---|---|---|
| vite | 3.2.6 | 5.x |
| typescript | 4.9.5 | 5.x |
| prettier | 2.8.7 | 3.x |

---

## Phase 3: その他ライブラリの更新 (後続フェーズ)

Phase 2完了後、必要に応じて実施:

- solid-js / @solidjs/router の最新版
- eslint の最新版
- tailwindcss の最新版

---

## リスクと対策

### リスク1: gRPCサーバー (tonic) との互換性

- **影響**: Tauri v2でのtokioランタイム管理方法が変わる可能性
- **対策**: マイグレーション後にgRPC通信の動作確認を重点的に実施

### リスク2: マルチウィンドウの動作

- **影響**: `Window` → `WebviewWindow` の変更でウィンドウ管理ロジックに影響
- **対策**: Viewer/Explorer両ウィンドウの連携動作を入念にテスト

### リスク3: Updater署名の互換性

- **影響**: v2形式への完全移行でリリースフローの変更が必要
- **対策**: GitHub Gistのフォーマットを更新、新しい署名形式に対応

---

## 参考リンク

- [Tauri v2 マイグレーションガイド](https://v2.tauri.app/start/migrate/from-tauri-1/)
- [Tauri v2 プラグイン一覧](https://v2.tauri.app/plugin/)
- [tauri-plugin-store](https://v2.tauri.app/plugin/store/)
- [tauri-plugin-updater](https://v2.tauri.app/plugin/updater/)

---

## 更新履歴

| 日付 | 内容 |
|---|---|
| 2026-01-12 | 初版作成 |
