# Explorer パフォーマンス最適化計画

## 概要

数千フォルダを含むディレクトリを Explorer で開く際、ページ移動時に 2-5 秒の顕著なローディング時間が発生している。本ドキュメントは、現状分析と最適化実装計画を記載する。

## 現状の問題点

### 主要なボトルネック

1. **バックエンド: ソートされていないディレクトリ反復処理**
   - `std::fs::read_dir` が返すイテレータは順序保証なし
   - `skip().take()` を適用しても、ファイルシステム依存の順序となり一貫性がない
   - 5,000 フォルダの場合、全エントリを収集してソートしないと正確なページネーション不可

2. **バックエンド: 冗長なディレクトリスキャン**
   - `explore_path()`: ディレクトリスキャン + 50 フォルダのサムネイル検索
   - `get_page_count()`: 同じディレクトリを再スキャンしてカウント
   - **結果**: 1 回のページ移動で 2 回の親ディレクトリスキャン + 50 回のサブディレクトリスキャン

3. **バックエンド: 入れ子 I/O によるサムネイル抽出**
   ```rust
   for entry in files.flatten() {
       let inner = read_dir(entry.path());  // ← 50回呼ばれる
       for inn_v in inner_file.by_ref() {   // ← 最初の画像まで線形探索
           if extensions.iter().any(|v| *v == ext) {
               thumbpath = ...;
               break;
           }
       }
   }
   ```
   - 各ページで 50 回の `read_dir` システムコール
   - フォルダ内に多数のファイルがある場合、最初の画像を見つけるまで全探索
   - **キャッシュなし**: 同じページを再訪問しても同じ処理を繰り返す

4. **バックエンド: async コンテキストでのブロッキング I/O**
   - `explore_path()` は同期関数だが、`async fn` から呼ばれる
   - Tokio エグゼキュータのスレッドをブロックする可能性
   - HDD や ネットワークドライブでは特に問題

5. **バックエンド: 長時間の state ロック保持**
   - `explorers.lock().await` を取得してから I/O 完了まで保持
   - 複数の Explorer ウィンドウが同時操作できない
   - ロック競合が発生する

6. **フロントエンド: 積極的なサムネイル読み込み**
   - 各 `Folder` コンポーネントが `createResource` で即座にサムネイル画像を要求
   - 50 個のサムネイルを並行して読み込み
   - ディスク I/O が飽和する可能性（特に HDD）
   - `loading="lazy"` 属性なし

### パフォーマンスシナリオ（5,000 フォルダの場合）

**現状の処理フロー:**
1. ユーザーがページ変更をクリック
2. `change_explorer_page` コマンド実行
3. `explorers.lock().await` 取得
4. `explore_path()` 呼び出し
   - 親ディレクトリスキャン（5,000 エントリ読み込み）
   - ソートなしで `skip(0).take(50)`
   - 50 フォルダに対して `read_dir()` → サムネイル検索
5. `get_page_count()` 呼び出し
   - 親ディレクトリを再スキャン（5,000 エントリ再読み込み）
6. ロック解放、IPC で結果送信
7. フロントエンドで 50 個の `createResource` 発動
8. 50 個の画像ファイルを並行読み込み

**推定時間:** HDD で 2-5 秒、SSD で 500ms-1 秒

**ボトルネック内訳:**
- 40% - サムネイル抽出（50 × `read_dir`）
- 30% - 未ソートディレクトリの収集
- 20% - 冗長な `get_page_count` スキャン
- 10% - フロントエンドのサムネイル読み込み

## 最適化戦略

### 設計方針

1. **メモリキャッシュのみ使用**
   - フォルダは流動的で、同じページを何度も表示する要件は少ない
   - アプリケーション実行中のみキャッシュを保持
   - 永続化不要

2. **ソート方法は既存踏襲**
   - 将来的にソート方法の追加を想定
   - 現状はファイルシステム順（自然順ソート未適用）

3. **サムネイル戦略は既存踏襲**
   - フォルダ内最初の画像ファイルを使用
   - 特定ファイル名の優先処理なし

### 最適化アプローチ

#### 1. 単一ディレクトリスキャン + ソート + ページネーション

**現在:**
```rust
pub(crate) fn explore_path(filepath: &str, page: usize) -> Result<Vec<Thumbnail>, String> {
    let dirs = read_dir(filepath)?;
    let files = dirs.skip((page - 1) * 50).take(50);  // ← 未ソート
    // サムネイル抽出...
}

pub(crate) async fn get_page_count(filepath: &str) -> Result<usize, String> {
    let dirs = read_dir(filepath)?;  // ← 2回目のスキャン
    Ok(dirs.count() / 50 + 1)
}
```

**改善後:**
```rust
pub(crate) fn explore_path_with_count(
    filepath: &str, 
    page: usize
) -> Result<(Vec<Thumbnail>, usize), String> {
    // 1. 単一スキャンで全エントリ収集
    let dirs = read_dir(filepath)?;
    let mut entries: Vec<_> = dirs
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    
    // 2. ソート（将来的にソート方法を追加可能な設計）
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    let total_count = entries.len();
    let total_pages = (total_count + 49) / 50;
    
    // 3. ページネーション
    let start = (page - 1) * 50;
    let end = (start + 50).min(total_count);
    let page_entries = &entries[start..end];
    
    // 4. サムネイル抽出（最適化後のロジック）
    let thumbnails = extract_thumbnails(page_entries)?;
    
    Ok((thumbnails, total_pages))
}
```

**効果:**
- 冗長なスキャンを削減: 2 回 → 1 回
- 一貫したページング: ソート済みエントリから確実に切り出し

#### 2. サムネイル抽出の並列化とキャッシュ

**現在:**
```rust
// 同期的に順次処理
for entry in files.flatten() {
    let inner = read_dir(entry.path())?;
    // 線形探索...
}
```

**改善後:**
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// AppState にキャッシュを追加
pub struct AppState {
    pub explorers: Arc<Mutex<Vec<ExplorerState>>>,
    pub thumbnail_cache: Arc<RwLock<HashMap<String, String>>>, // ← 追加
    // ...
}

// 並列処理でサムネイル抽出
async fn extract_thumbnails_parallel(
    entries: &[DirEntry],
    cache: Arc<RwLock<HashMap<String, String>>>
) -> Result<Vec<Thumbnail>, String> {
    let tasks: Vec<_> = entries
        .iter()
        .map(|entry| {
            let path = entry.path();
            let cache = cache.clone();
            
            tokio::spawn_blocking(move || {
                let path_str = path.to_str().unwrap().to_string();
                
                // キャッシュチェック
                let cached = tokio::runtime::Handle::current()
                    .block_on(cache.read())
                    .get(&path_str)
                    .cloned();
                    
                if let Some(thumb) = cached {
                    return Ok(Thumbnail {
                        path: path_str.clone(),
                        filename: path.file_name()...to_string(),
                        thumbpath: thumb,
                    });
                }
                
                // キャッシュミス: 検索
                let thumb = find_first_image(&path)?;
                
                // キャッシュに保存
                tokio::runtime::Handle::current()
                    .block_on(cache.write())
                    .insert(path_str.clone(), thumb.clone());
                
                Ok(Thumbnail {
                    path: path_str,
                    filename: ...,
                    thumbpath: thumb,
                })
            })
        })
        .collect();
    
    let results = futures::future::join_all(tasks).await;
    // エラーハンドリング...
}
```

**効果:**
- 並列処理: 50 フォルダを並行してスキャン（CPU コア数に応じた高速化）
- キャッシュヒット時: ディスク I/O なし（メモリから即座に取得）
- 非同期化: Tokio エグゼキュータをブロックしない

#### 3. ロックスコープの最適化

**現在:**
```rust
pub(crate) async fn change_explorer_page(...) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;  // ← ロック取得
    // ... パスを取得 ...
    let thumbnails = explore_path(&path, page)?;      // ← I/O実行（ロック保持中）
    let end = get_page_count(&path).await?;           // ← I/O実行（ロック保持中）
    // ... 状態更新 ...
    app.emit_to(...)?;
    Ok(())
}  // ← ロック解放
```

**改善後:**
```rust
pub(crate) async fn change_explorer_page(...) -> Result<(), String> {
    // 1. 必要なデータのみロックして取得
    let (label, path) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label)...;
        let tab = &explorer_state.tabs[index];
        (explorer_state.label.clone(), tab.path.clone().unwrap_or_default())
    };  // ← ロック即座に解放
    
    // 2. ロック外でI/O実行
    let (thumbnails, total_pages) = explore_path_with_count(&path, page).await?;
    
    // 3. 再度ロック取得して状態更新のみ
    {
        let mut explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter_mut().find(|w| w.label == label)...;
        let tab = &mut explorer_state.tabs[index];
        tab.page = page;
        tab.folders = thumbnails;
        tab.end = total_pages;
    }  // ← ロック解放
    
    // 4. イベント送信
    app.emit_to(...)?;
    Ok(())
}
```

**効果:**
- ロック保持時間を最小化: I/O 時間を除外
- 並行性向上: 複数の Explorer ウィンドウが独立して動作可能

#### 4. フロントエンド: 遅延画像読み込み

**現在:**
```tsx
// Folder.tsx
const [data] = createResource(
  () => props.thumb.thumbpath,
  () => props.thumb.thumbpath ? convertFileSrc(props.thumb.thumbpath) : fallback,
);

return <img src={data()} ... />;  // ← 即座に全画像読み込み
```

**改善後:**
```tsx
// Folder.tsx
const [data] = createResource(
  () => props.thumb.thumbpath,
  () => props.thumb.thumbpath ? convertFileSrc(props.thumb.thumbpath) : fallback,
);

return (
  <img 
    src={data()} 
    loading="lazy"  // ← ブラウザネイティブの遅延読み込み
    ... 
  />
);
```

**効果:**
- ビューポート外の画像は読み込まれない
- スクロール時に必要な画像のみ読み込み
- 初期レンダリング時の I/O 負荷を削減

#### 5. ディレクトリ監視によるキャッシュ無効化

**既存パターン（viewer.rs）:**
```rust
pub(crate) async fn subscribe_dir_notification(
    app: AppHandle,
    state: State<'_, AppState>,
    label: String,
    dir: String,
) -> Result<(), String> {
    // notify クレートでディレクトリ監視...
}
```

**Explorer への適用:**
```rust
pub(crate) async fn subscribe_explorer_dir_notification(
    app: AppHandle,
    state: State<'_, AppState>,
    label: String,
    dir: String,
) -> Result<(), String> {
    // ディレクトリ変更を検知
    // → thumbnail_cache から該当パスのエントリを削除
    // → フロントエンドにイベント送信して再読み込みをトリガー
}
```

**効果:**
- フォルダ追加/削除/名前変更時に自動でキャッシュ無効化
- 古いデータを表示し続けるリスクを低減

## 実装手順

### Phase 1: バックエンドの最適化

#### Step 1: `explore_path_with_count` の実装

**ファイル:** `src-tauri/src/app/explorer.rs`

1. 新関数 `explore_path_with_count` を追加
   - 単一の `read_dir` 呼び出しで全エントリ収集
   - `Vec<DirEntry>` に収集してソート
   - ページネーション適用
   - サムネイル抽出
   - `(Vec<Thumbnail>, usize)` を返す（サムネイル配列 + 総ページ数）

2. `explore_path` と `get_page_count` を `explore_path_with_count` に置き換え
   - 全呼び出し箇所を更新

#### Step 2: サムネイル抽出の並列化

**ファイル:** `src-tauri/src/app/explorer.rs`, `src-tauri/src/service/app_state.rs`

1. `AppState` に `thumbnail_cache: Arc<RwLock<HashMap<String, String>>>` を追加

2. `find_first_image` 関数を抽出（現在の入れ子ロジックを関数化）

3. `extract_thumbnails_parallel` 関数を実装
   - `tokio::spawn_blocking` で並列化
   - キャッシュチェック → ヒットなら即返却
   - ミスなら `find_first_image` → キャッシュに保存

4. `explore_path_with_count` から呼び出し

#### Step 3: ロックスコープの最適化

**ファイル:** `src-tauri/src/app/explorer.rs`

以下の関数を修正:
- `change_explorer_page`
- `move_explorer_forward`
- `move_explorer_backward`
- `change_explorer_path`
- `open_new_explorer_tab`
- その他全てのナビゲーションコマンド

**パターン:**
```rust
// 1. データ取得
let data = {
    let lock = state.explorers.lock().await;
    // 必要なデータをクローン
};

// 2. I/O実行（ロック外）
let result = expensive_io_operation(&data).await?;

// 3. 状態更新
{
    let mut lock = state.explorers.lock().await;
    // 状態更新のみ
};
```

### Phase 2: フロントエンドの最適化

#### Step 4: 遅延画像読み込み

**ファイル:** `src/features/Folder/routes/Folder.tsx`

`<img>` タグに `loading="lazy"` 属性を追加:

```tsx
<img
  class="block cursor-pointer w-40 h-40 object-contain"
  onClick={() => props.onClick(props.thumb)}
  src={data()}
  loading="lazy"  // ← 追加
  onError={(e) => (e.currentTarget.src = fallback)}
/>
```

### Phase 3: キャッシュ無効化

#### Step 5: ディレクトリ監視の実装

**ファイル:** `src-tauri/src/app/explorer.rs`

1. `subscribe_explorer_dir_notification` コマンドを追加
   - `notify` クレートで指定ディレクトリを監視
   - ファイル/フォルダの作成・削除・名前変更を検知

2. 変更検知時の処理:
   - `thumbnail_cache` から該当ディレクトリのエントリを削除
   - `app.emit_to()` でフロントエンドに通知

3. フロントエンド側で通知を受信してリフレッシュ
   - `ExplorerTab.tsx` でイベントリスナー追加

## 期待される効果

### パフォーマンス改善見込み

**5,000 フォルダのディレクトリ:**

| 項目 | 現状 | 改善後 | 改善率 |
|------|------|--------|--------|
| ディレクトリスキャン | 2 回 | 1 回 | **50% 削減** |
| サムネイル検索（初回） | 順次 50 回 | 並列 50 回 | **3-5倍高速化** |
| サムネイル検索（2回目以降） | 順次 50 回 | キャッシュヒット | **50-100倍高速化** |
| ロック競合 | 高 | 低 | **並行性向上** |
| フロントエンド画像読み込み | 50 並行 | ビューポート内のみ | **初期負荷 70% 削減** |

**推定総合改善:**
- HDD: 2-5 秒 → **500ms-1 秒**（4-5倍高速化）
- SSD: 500ms-1 秒 → **100-300ms**（3-5倍高速化）

### 特に改善されるシナリオ

1. **ページ再訪問**: キャッシュヒットにより劇的に高速化
2. **複数 Explorer ウィンドウ**: ロック競合が減少
3. **多数ファイルを含むフォルダ**: 並列処理で大幅改善
4. **低速ストレージ（HDD、ネットワークドライブ）**: I/O 回数削減の効果が顕著

## 今後の拡張性

### ソート方法の追加

現在の実装は将来的なソート機能追加を考慮した設計:

```rust
pub(crate) fn explore_path_with_count(
    filepath: &str, 
    page: usize,
    sort_by: Option<SortMethod>,  // ← 将来追加
) -> Result<(Vec<Thumbnail>, usize), String> {
    // ...
    match sort_by {
        Some(SortMethod::Name) => entries.sort_by(|a, b| a.file_name().cmp(&b.file_name())),
        Some(SortMethod::ModifiedDate) => entries.sort_by_key(|e| e.metadata().modified()),
        Some(SortMethod::Natural) => entries.sort_by(|a, b| natord::compare(...)),
        None => { /* 現在の動作 */ }
    }
    // ...
}
```

### サムネイル生成戦略の拡張

将来的な拡張例:
- 特定ファイル名優先（folder.jpg など）
- 複数画像のコラージュ
- 動画サムネイル（フレーム抽出）

現在の `find_first_image` を `ThumbnailStrategy` トレイトに抽象化することで対応可能。

## リスクと対策

### 1. メモリ使用量の増加

**リスク:** 大量のサムネイルパスをキャッシュするとメモリを消費

**対策:**
- LRU（Least Recently Used）キャッシュの実装
- キャッシュサイズ上限設定（例: 10,000 エントリ）
- 古いエントリの自動削除

### 2. キャッシュ整合性

**リスク:** ファイルシステム変更がキャッシュに反映されない

**対策:**
- ディレクトリ監視（notify）による自動無効化
- 手動リフレッシュ機能（ユーザー操作）
- タイムスタンプベースの無効化判定（オプション）

### 3. 並列処理のオーバーヘッド

**リスク:** 少数フォルダの場合、並列化のオーバーヘッドが逆効果

**対策:**
- 閾値による切り替え（例: 10 フォルダ未満は順次処理）
- `rayon` の代わりに `tokio::spawn_blocking` で調整

### 4. ソート処理の時間

**リスク:** 数万フォルダでソートに時間がかかる

**対策:**
- 初回スキャン時にソート済みキャッシュを保持
- ページ単位でのソート（全体ソート不要ならオプション化）

## 実装上の注意事項

### エラーハンドリング

- `tokio::spawn_blocking` の結果は `JoinError` を含む
- 並列処理で一部失敗した場合の処理を明確化
- ユーザーに適切なエラーメッセージを表示

### テスト

- 大量フォルダ（1,000+）でのパフォーマンステスト
- キャッシュヒット/ミスのシナリオテスト
- 並行アクセスのストレステスト
- ディレクトリ監視の動作確認

### 互換性

- 既存の IPC インターフェース（`ExplorerTabState`）は維持
- フロントエンドの変更は最小限に抑える
- 段階的なロールアウトが可能な設計

## まとめ

本最適化計画により、Explorer の応答性を 3-5 倍改善し、数千フォルダを含むディレクトリでも快適なユーザー体験を提供できる見込み。実装は段階的に行い、各フェーズで効果を検証しながら進める。
