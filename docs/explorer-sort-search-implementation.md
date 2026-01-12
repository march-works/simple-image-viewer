# Explorer ソート・検索機能実装計画

## 概要

Explorer に日付ソート（昇順/降順）と名前検索機能を追加する。将来的なリコメンド機能・機械学習拡張を見据えた設計とする。

## ロードマップ

| フェーズ | 機能 | 概要 | 状態 |
|----------|------|------|------|
| **Phase 1** | 日付ソート・名前検索 | ソート・検索 UI | ✅ 完了 |
| **Phase 2** | SQLite + サムネイルデータ蓄積 | リコメンドの基盤データ収集 | 📋 計画 |
| **Phase 3** | ~~ルールベースリコメンド~~ | ~~頻度・傾向に基づくスコアリング~~ | ❌ 廃止 |
| **Phase 4** | 機械学習リコメンド (CLIP) | サムネイル埋め込み + 類似度計算 | 📋 計画 |

---

## Phase 1: 日付ソート・名前検索

### 要件

- **ソート機能**
  - ソートフィールド: 名前 / 更新日 / 作成日
  - ソート順: 昇順 / 降順
  - デフォルト: 更新日降順
  - タブ単位で状態を保持・永続化

- **検索機能**
  - 現在のディレクトリ直下のフォルダ名でフィルタ
  - バックエンド側でフィルタリング（ページネーションとの整合性のため）
  - 大文字小文字を区別しない
  - デバウンス 300ms

- **メタデータ**
  - `Thumbnail` 構造体に `modified_at`, `created_at` を追加
  - インメモリキャッシュ（永続化は Phase 2 の SQLite 導入時に検討）

- **UI**
  - ソート・検索コントロールはタブ内上部に常時表示

### 設計

#### Rust 型定義 (`src-tauri/src/app/explorer_types.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SortField {
    Name,
    #[default]
    DateModified,
    DateCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SortConfig {
    pub field: SortField,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExplorerQuery {
    pub sort: SortConfig,
    pub search: Option<String>,
}
```

#### Thumbnail 構造体拡張 (`src-tauri/src/app/explorer_helpers.rs`)

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Thumbnail {
    pub path: String,
    pub filename: String,
    pub thumbpath: String,
    pub modified_at: Option<u64>,  // Unix timestamp (seconds)
    pub created_at: Option<u64>,   // Unix timestamp (seconds)
}
```

#### ExplorerTabState 拡張 (`src-tauri/src/service/app_state.rs`)

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ExplorerTabState {
    pub title: String,
    pub key: String,
    pub path: Option<String>,
    pub transfer_path: Option<String>,
    pub page: usize,
    pub end: usize,
    pub folders: Vec<Thumbnail>,
    pub sort: SortConfig,           // 追加
    pub search_query: Option<String>, // 追加
}
```

#### Tauri コマンド (`src-tauri/src/app/explorer.rs`)

- `change_explorer_sort(label: String, tab_key: String, sort: SortConfig)` - ソート変更
- `change_explorer_search(label: String, tab_key: String, query: Option<String>)` - 検索クエリ変更

#### TypeScript 型定義

```typescript
// src/features/Explorer/types/ExplorerQuery.ts
export type SortField = 'Name' | 'DateModified' | 'DateCreated';
export type SortOrder = 'Asc' | 'Desc';

export type SortConfig = {
  field: SortField;
  order: SortOrder;
};

// src/features/Folder/types/Thumbnail.ts
export type Thumbnail = {
  path: string;
  filename: string;
  thumbpath: string;
  modified_at?: number;
  created_at?: number;
};
```

#### UI コンポーネント (`src/pages/explorer/ExplorerTab.tsx`)

- ソートドロップダウン（6択: 名前↑↓, 更新日↑↓, 作成日↑↓）
- 検索入力フィールド（デバウンス 300ms）
- Tailwind CSS でスタイリング

---

## Phase 2: SQLite + サムネイルデータ蓄積

### 概要

リコメンド機能 (Phase 4) の基盤として、SQLite にサムネイル画像データと閲覧履歴を蓄積する。

### 要件

- **SQLite データベース**
  - `rusqlite` クレート (bundled) を導入
  - DB ファイル配置:
    - 開発: `{AppData}/simple-image-viewer-dev/data.db`
    - 本番: `{AppData}/simple-image-viewer/data.db`

- **サムネイルデータ**
  - 224×224 にリサイズした RGB 画像データを BLOB で保存
  - 変更検知用のハッシュ値を保持
  - `image` クレートでリサイズ処理

- **閲覧履歴**
  - Viewer でフォルダ内ファイルを開いた際に記録
  - `last_viewed_at` (Unix timestamp) と `view_count` を更新

### SQLite スキーマ

```sql
CREATE TABLE folder_records (
    path TEXT PRIMARY KEY,
    thumbnail_blob BLOB,           -- 224×224 RGB bytes (JPEG encoded)
    thumbnail_hash TEXT,           -- SHA256 for change detection
    last_viewed_at INTEGER,        -- Unix timestamp (nullable, 未閲覧なら NULL)
    view_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL    -- レコード作成日時
);
CREATE INDEX idx_last_viewed ON folder_records(last_viewed_at DESC);
```

### 追加 Rust クレート

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
image = "0.25"
sha2 = "0.10"
```

### 実装詳細

#### DB 初期化 (`src-tauri/src/service/database.rs`)

```rust
use rusqlite::Connection;
use std::path::PathBuf;

pub fn init_database(app_data_dir: &PathBuf) -> Result<Connection, rusqlite::Error> {
    let db_path = app_data_dir.join("data.db");
    let conn = Connection::open(&db_path)?;
    
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS folder_records (
            path TEXT PRIMARY KEY,
            thumbnail_blob BLOB,
            thumbnail_hash TEXT,
            last_viewed_at INTEGER,
            view_count INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_last_viewed ON folder_records(last_viewed_at DESC);
    "#)?;
    
    Ok(conn)
}
```

#### サムネイル生成 (`src-tauri/src/utils/thumbnail_utils.rs`)

```rust
use image::{DynamicImage, ImageFormat};
use sha2::{Sha256, Digest};
use std::io::Cursor;

pub struct ThumbnailData {
    pub blob: Vec<u8>,
    pub hash: String,
}

pub fn generate_thumbnail_data(image_path: &str) -> Result<ThumbnailData, anyhow::Error> {
    let img = image::open(image_path)?;
    let resized = img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
    
    let mut buf = Cursor::new(Vec::new());
    resized.write_to(&mut buf, ImageFormat::Jpeg)?;
    let blob = buf.into_inner();
    
    let hash = format!("{:x}", Sha256::digest(&blob));
    
    Ok(ThumbnailData { blob, hash })
}
```

#### AppState 拡張 (`src-tauri/src/service/app_state.rs`)

```rust
use rusqlite::Connection;
use std::sync::Mutex;

pub struct AppState {
    // ... 既存フィールド
    pub db: Mutex<Connection>,
}
```

#### 閲覧記録 Tauri コマンド (`src-tauri/src/app/viewer.rs`)

```rust
#[tauri::command]
pub async fn record_folder_view(
    folder_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // folder_path からサムネイル取得 → DB upsert
}
```

### 実装ステップ

1. `rusqlite`, `image`, `sha2` クレートを Cargo.toml に追加
2. `src-tauri/src/service/database.rs` 作成、DB 初期化・マイグレーション
3. `src-tauri/src/utils/thumbnail_utils.rs` 作成、サムネイル生成関数
4. `AppState` に DB 接続を追加、アプリ起動時に初期化
5. `record_folder_view` コマンド追加、Viewer から呼び出し
6. 動作確認

---

## Phase 3: ルールベースリコメンド（廃止）

> **Note**: Phase 3 は廃止。同一アイテムを繰り返し閲覧するユースケースを想定しないため、
> 頻度ベースのルールは不要。Phase 4 のサムネイル類似度ベースに直接移行する。

---

## Phase 4: 機械学習リコメンド (CLIP)

### 概要

CLIP (ViT-B/32) モデルを使用し、サムネイル画像の埋め込みベクトルから類似フォルダを推薦する。
サムネイル（画像）をメイン、パス/タイトル（テキスト）をサブとした重み付けスコアリング。

### 要件

- **埋め込みモデル**
  - CLIP ViT-B/32 (ONNX 形式)
  - 画像エンコーダ: ~350MB
  - テキストエンコーダ: ~65MB
  - アプリに同梱 (`src-tauri/resources/`)

- **埋め込み生成**
  - サムネイル画像 → 512 次元ベクトル (画像エンコーダ)
  - フォルダパス → 512 次元ベクトル (テキストエンコーダ)
  - バックグラウンドで非同期生成

- **リコメンドロジック**
  - 直近閲覧 N 件（ディレクトリ問わず）の埋め込み平均を基準
  - 現在ディレクトリ内のフォルダとのコサイン類似度を計算
  - スコア = `0.8 × cosine_sim(image) + 0.2 × cosine_sim(path)`
  - Explorer に「おすすめ順」ソートオプション追加

- **UI**
  - 「リコメンドを再構築」ボタンを Explorer に配置
  - 再構築中はボタンをグローバルに非活性化
  - 処理完了後に通知

### スキーマ拡張

```sql
ALTER TABLE folder_records ADD COLUMN image_embedding BLOB;  -- 512 * 4 bytes (f32)
ALTER TABLE folder_records ADD COLUMN path_embedding BLOB;   -- 512 * 4 bytes (f32)
ALTER TABLE folder_records ADD COLUMN embedding_version INTEGER DEFAULT 0;  -- モデル更新時の再計算用
```

### 追加 Rust クレート

```toml
ort = "2.0"       # ONNX Runtime
ndarray = "0.16"  # 多次元配列
```

### リソース配置

```
src-tauri/
├── resources/
│   ├── clip-vit-b32-image.onnx   # ~350MB
│   └── clip-vit-b32-text.onnx    # ~65MB
└── tauri.conf.json  # resources に追加
```

### バックグラウンド処理フロー

```
「リコメンドを再構築」ボタン押下
    ↓
グローバルで処理中フラグを ON（ボタン非活性化）
    ↓
DB 内の全フォルダを取得
    ↓
tokio::spawn で非同期処理
    ├─ 埋め込み未生成: サムネイル取得 → CLIP 推論 → UPDATE
    ├─ サムネイル変更: 再生成 → CLIP 推論 → UPDATE
    └─ パス変更検知: テキスト埋め込み再生成 → UPDATE
    ↓
処理完了 → フロントエンドに emit
    ↓
グローバルで処理中フラグを OFF（ボタン活性化）
```

### 差分検知

| 検知対象 | 方法 |
|----------|------|
| フォルダ追加 | DB に path なし → INSERT |
| フォルダ削除 | DB に path あるがディスク上にない → 削除 or 無視 |
| サムネイル変更 | `thumbnail_hash` 比較 → 埋め込み再生成 |
| モデル更新 | `embedding_version` 比較 → 全件再生成 |

### リコメンドソート実装

```rust
pub fn calculate_recommendation_scores(
    current_dir_folders: &[FolderRecord],
    recent_views: &[FolderRecord],  // 直近 N 件（ディレクトリ問わず）
) -> Vec<(String, f32)> {
    // 1. recent_views の image_embedding 平均ベクトル算出
    // 2. recent_views の path_embedding 平均ベクトル算出
    // 3. current_dir_folders 各フォルダとのコサイン類似度計算
    // 4. スコア = 0.8 * image_sim + 0.2 * path_sim
    // 5. (path, score) のリストを返却
}
```

### 実装ステップ

1. `ort`, `ndarray` クレートを Cargo.toml に追加
2. CLIP ONNX モデルを `src-tauri/resources/` に配置
3. `tauri.conf.json` の `resources` にモデルファイルを追加
4. `src-tauri/src/service/embedding_service.rs` 作成
   - モデル初期化
   - 画像埋め込み生成
   - テキスト埋め込み生成
5. `folder_records` テーブルに埋め込みカラム追加（マイグレーション）
6. バックグラウンド埋め込み生成処理
7. `rebuild_recommendations` コマンド追加
8. リコメンドスコア計算ロジック
9. Explorer に「おすすめ順」ソートオプション追加
10. 「リコメンドを再構築」ボタン UI
11. 動作確認・調整

---

## 実装ステップ（Phase 1）✅ 完了

1. ✅ 実装計画ドキュメント作成
2. ✅ Rust: `explorer_types.rs` にソート・検索型定義を追加
3. ✅ Rust: `Thumbnail` 構造体を拡張
4. ✅ Rust: `AppState` / `ExplorerTabState` を拡張
5. ✅ Rust: `explore_path_with_count` にソート・フィルタロジック追加
6. ✅ Rust: Tauri コマンド `change_explorer_sort`, `change_explorer_search` を追加
7. ✅ Rust: 既存コマンドでソート・検索状態を考慮
8. ✅ TypeScript: 型定義を追加
9. ✅ TypeScript: `ExplorerTab.tsx` に UI コンポーネントを追加
10. ✅ 動作確認・調整

---

## 実装ステップ（Phase 2）

1. `rusqlite`, `image`, `sha2` クレートを Cargo.toml に追加
2. `src-tauri/src/service/database.rs` 作成、DB 初期化・マイグレーション
3. `src-tauri/src/utils/thumbnail_utils.rs` 作成、サムネイル生成関数
4. `AppState` に DB 接続を追加、アプリ起動時に初期化
5. `record_folder_view` コマンド追加、Viewer から呼び出し
6. 動作確認

---

## 実装ステップ（Phase 4）

1. `ort`, `ndarray` クレートを Cargo.toml に追加
2. CLIP ONNX モデルを `src-tauri/resources/` に配置
3. `tauri.conf.json` の `resources` にモデルファイルを追加
4. `src-tauri/src/service/embedding_service.rs` 作成
5. `folder_records` テーブルに埋め込みカラム追加（マイグレーション）
6. バックグラウンド埋め込み生成処理
7. `rebuild_recommendations` コマンド追加
8. リコメンドスコア計算ロジック
9. Explorer に「おすすめ順」ソートオプション追加
10. 「リコメンドを再構築」ボタン UI
11. 動作確認・調整
