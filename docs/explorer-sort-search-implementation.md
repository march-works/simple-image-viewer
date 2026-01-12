# Explorer ソート・検索機能実装計画

## 概要

Explorer に日付ソート（昇順/降順）と名前検索機能を追加する。将来的なリコメンド機能・機械学習拡張を見据えた設計とする。

## ロードマップ

| フェーズ | 機能 | 概要 | 状態 |
|----------|------|------|------|
| **Phase 1** | 日付ソート・名前検索 | 今回のスコープ | 🚧 実装中 |
| **Phase 2** | 閲覧履歴記録 + SQLite 導入 | リコメンドの基盤データ収集 | 📋 計画 |
| **Phase 3** | リコメンド（ルールベース） | 頻度・傾向に基づくスコアリング | 📋 計画 |
| **Phase 4** | 機械学習リコメンド | 埋め込みベクトル + 類似度計算 | 📋 計画 |

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

## Phase 2: 閲覧履歴記録 + SQLite 導入（将来計画）

### 概要

リコメンド機能の基盤として、閲覧履歴を SQLite に記録する。

### 主要コンポーネント

- `rusqlite` クレートを導入
- `ViewHistory` テーブル: `path`, `opened_at`, `duration_seconds`
- `FolderMetadata` テーブル: `path`, `modified_at`, `created_at`, `tags`
- Viewer でファイルを開いた際に履歴を記録

---

## Phase 3: リコメンド（ルールベース）（将来計画）

### 概要

閲覧履歴からルールベースでスコアリングし、おすすめ順ソートを実現。

### スコアリング要素

- 直近の閲覧頻度
- 類似フォルダ名の閲覧傾向
- 更新日の新しさ

---

## Phase 4: 機械学習リコメンド（将来計画）

### 概要

フォルダ名・タグから埋め込みベクトルを生成し、類似度計算でリコメンド。

### 技術選定候補

- ベクトル DB: `qdrant`（ローカル）または `sqlite-vss`
- 埋め込みモデル: ローカル推論（ONNX Runtime）

---

## 実装ステップ（Phase 1）

1. ✅ 実装計画ドキュメント作成
2. Rust: `explorer_types.rs` にソート・検索型定義を追加
3. Rust: `Thumbnail` 構造体を拡張
4. Rust: `AppState` / `ExplorerTabState` を拡張
5. Rust: `explore_path_with_count` にソート・フィルタロジック追加
6. Rust: Tauri コマンド `change_explorer_sort`, `change_explorer_search` を追加
7. Rust: 既存コマンドでソート・検索状態を考慮
8. TypeScript: 型定義を追加
9. TypeScript: `ExplorerTab.tsx` に UI コンポーネントを追加
10. 動作確認・調整
