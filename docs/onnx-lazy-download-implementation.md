# ONNX モデル遅延ダウンロード実装計画

## 背景

CLIP モデル（vision_model.onnx: ~350MB, text_model.onnx: ~254MB）をアプリにバンドルすることで、以下の問題が発生:

- **バンドルサイズ爆発**: ~10MB → ~600MB
- **ビルド時間増大**: CI/CD ビルドに 30 分以上
- **Git LFS 問題**: リリースビルドで LFS ファイルがダウンロードされない

## 解決方針

**遅延ダウンロード方式**を採用:

1. ONNX ファイルをアプリバンドルから除外
2. 初回使用時に GitHub Releases から自動ダウンロード
3. ダウンロードしたファイルはアプリデータディレクトリにキャッシュ

## 実装状況

- [x] Step 1: tauri.conf.json から ONNX を除外
- [x] Step 2: Cargo.toml に依存関係追加 (reqwest, futures-util)
- [x] Step 3: model_downloader.rs 作成
- [x] Step 4: embedding_service 初期化の変更
- [x] Step 5: フロントエンド進捗表示コンポーネント作成
- [x] Step 6: GitHub Actions から LFS 手順を削除
- [ ] Step 7: GitHub Releases に models-v1 リリースを作成（手動作業）

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                        アプリケーション                           │
├─────────────────────────────────────────────────────────────────┤
│  アプリ起動時                                                     │
│              ↓                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  ModelDownloader (model_downloader.rs)                   │    │
│  │  1. キャッシュディレクトリを確認                           │    │
│  │  2. ファイルが存在しなければダウンロード                    │    │
│  │  3. ファイルサイズで整合性を検証                           │    │
│  │  4. EmbeddingService を初期化                             │    │
│  └─────────────────────────────────────────────────────────┘    │
│              ↓                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  フロントエンド (ModelDownloadProgress.tsx)              │    │
│  │  - model-download-progress イベントをリッスン            │    │
│  │  - 進捗バーを表示                                        │    │
│  └─────────────────────────────────────────────────────────┘    │
│              ↓                                                   │
│  リコメンド機能が利用可能に                                        │
└─────────────────────────────────────────────────────────────────┘
```

## ダウンロード元

GitHub Releases を使用:

```
https://github.com/march-works/simple-image-viewer/releases/download/models-v1/vision_model.onnx
https://github.com/march-works/simple-image-viewer/releases/download/models-v1/text_model.onnx
```

**注意**: `models-v1` タグでリリースを作成し、ONNX ファイルをアセットとしてアップロードが必要

## キャッシュディレクトリ

```
Windows: %APPDATA%/simple-image-viewer/models/
macOS:   ~/Library/Application Support/simple-image-viewer/models/
Linux:   ~/.local/share/simple-image-viewer/models/
```

開発モード:
```
Windows: %APPDATA%/simple-image-viewer-dev/models/
```

## 変更されたファイル

### Step 1: tauri.conf.json から ONNX を除外

```json
// 削除
"resources": [
  "resources/vision_model.onnx",
  "resources/text_model.onnx"
]
// 変更後
"resources": []
```

### Step 2: Cargo.toml に依存関係追加

```toml
reqwest = { version = "0.12", features = ["stream"] }
futures-util = "0.3"
```

### Step 3: model_downloader.rs

`src-tauri/src/service/model_downloader.rs` を新規作成

主要な機能:
- `ModelDownloader` 構造体: モデルのダウンロードとキャッシュを管理
- `ensure_models()` 関数: 両方のモデルが利用可能であることを保証
- 進捗イベント `model-download-progress` を emit

### Step 4: embedding_service 初期化の変更

`src-tauri/src/app/mod.rs` を修正:
- 開発モード: `src-tauri/resources` から直接ロード
- 本番モード: `model_downloader::ensure_models()` を使用

### Step 5: フロントエンド進捗表示

`src/components/ModelDownloadProgress/ModelDownloadProgress.tsx` を新規作成

- ダウンロード中に右下に進捗バーを表示
- 完了/失敗後は3秒で自動的に非表示

### Step 6: GitHub Actions

`.github/workflows/tauri_release.yml` から以下を削除:
- `lfs: true`
- `git lfs pull`

## モデルファイルのリリース手順（Step 7: 手動作業）

1. `models-v1` タグでリリースを作成
2. 以下のファイルをアセットとしてアップロード:
   - `vision_model.onnx` (~350MB)
   - `text_model.onnx` (~254MB)

## エラーハンドリング

| シナリオ | 対応 |
|---------|------|
| ネットワーク未接続 | 「モデルをダウンロードするにはインターネット接続が必要です」 |
| ダウンロード中断 | 部分ファイルを削除、再試行を促す |
| チェックサム不一致 | ファイルを削除、再ダウンロードを促す |
| ディスク容量不足 | 「600MB 以上の空き容量が必要です」 |

## 期待される効果

| 項目 | Before | After |
|------|--------|-------|
| インストーラサイズ | ~600MB | ~10MB |
| CI/CD ビルド時間 | ~30分 | ~5分 |
| 初回ダウンロード | 不要 | ~600MB（ユーザーが必要な場合のみ） |

## 実装順序

1. ✅ Step 1: tauri.conf.json から ONNX を除外
2. ✅ Step 2: Cargo.toml に依存関係追加
3. ✅ Step 3: model_downloader.rs の実装
4. ✅ Step 4: embedding_service 初期化ロジックの変更
5. ✅ Step 5: フロントエンドの進捗表示
6. ✅ Step 6: GitHub Actions の更新
7. ⬜ Step 7: モデル用リリースの作成（手動）
