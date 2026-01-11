# メモリリーク・リソース管理問題の修正計画

作成日: 2026-01-12

## 概要

アプリケーションで以下の症状が報告されている:
- 長時間起動しているとき強制終了する
- Viewerでタブを多く開いているとき強制終了する
- Explorerのファイル読み込みなどで重い処理中にViewerを動かしたとき強制終了する

調査の結果、以下の問題が特定された。

## 発見された問題一覧

| # | 影響度 | 問題 | ファイル |
|---|--------|------|----------|
| 1 | 🔴 高 | ファイル監視(notify)のリソースリーク | `src-tauri/src/app/viewer.rs` |
| 2 | 🔴 高 | ZIP読み込み時のメモリ爆発 | `src-tauri/src/app/viewer.rs` |
| 3 | 🔴 高 | get_compressed_file_treeでのZIP全体読み込み | `src-tauri/src/app/explorer.rs` |
| 4 | 🟡 中 | ViewerTabのイベントリスナー解除漏れ | `src/pages/viewer/ViewerTab.tsx` |
| 5 | 🟡 中 | ExplorerTabのイベントリスナー解除漏れ | `src/pages/explorer/ExplorerTab.tsx` |
| 6 | 🟡 中 | タブごとのFileTree重複保持 | `src-tauri/src/service/app_state.rs` |
| 7 | 🟡 中 | video.jsのdispose処理欠如 | `src/pages/viewer/ViewerTab.tsx` |
| 8 | 🟢 低 | 非同期ロックの長時間保持 | 複数ファイル |

---

## 問題詳細と修正方針

### 問題1: ファイル監視(notify)のリソースリーク 🔴

**現状のコード:**
```rust
#[tauri::command]
pub(crate) fn subscribe_dir_notification(filepath: String, app: AppHandle) {
    let path_inner = filepath.clone();
    recommended_watcher(move |res| match res {
        // ...
    })
    .map_or_else(
        |_| (),
        |mut watcher| {
            watcher
                .watch(Path::new(&filepath), RecursiveMode::Recursive)
                .unwrap_or(())
        },
    );
    // ← watcher がここでスコープを抜けてドロップされるが、内部スレッドは停止しない！
}
```

**問題点:**
1. `watcher`が関数終了時にスコープを抜けてドロップされるが、notifyの内部監視スレッドは停止しない
2. タブを開くたびに`subscribe_dir_notification`が呼ばれ、監視スレッドが累積
3. タブを閉じても監視は解除されない
4. 長時間使用で数百〜数千の監視スレッドが蓄積し、メモリ・リソース枯渇

**修正方針:**
- `AppState`に`HashMap<String, RecommendedWatcher>`を追加
- パスごとにwatcherを管理し、参照カウントで共有
- タブ削除時に対応するwatcherを適切に`drop`

**修正ファイル:**
- `src-tauri/src/service/app_state.rs`
- `src-tauri/src/app/viewer.rs`

---

### 問題2, 3: ZIP読み込み時のメモリ爆発 🔴

**現状のコード:**
```rust
// viewer.rs - read_image_in_zip
let file = std::fs::read(path).unwrap_or_default(); // ZIPファイル全体をメモリに読み込み
let zip = zip::ZipArchive::new(std::io::Cursor::new(file));

// explorer.rs - get_compressed_file_tree
let file = std::fs::read(filepath).unwrap_or_default(); // 同様
```

**問題点:**
1. `std::fs::read`でZIPファイル全体(数百MB〜数GB)をメモリに読み込む
2. 展開した画像データもメモリに保持
3. Base64エンコードでさらにメモリ使用量が約1.33倍に
4. 大きなZIPファイルで画像を連続で見ると急激にメモリ増加

**修正方針:**
- `std::fs::File`と`BufReader`を使用してストリーミング読み込みに変更
- ZIPアーカイブを直接ファイルから開く

**修正後のコード:**
```rust
use std::fs::File;
use std::io::BufReader;

let file = File::open(&path).map_err(|e| e.to_string())?;
let reader = BufReader::new(file);
let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;
```

**修正ファイル:**
- `src-tauri/src/app/viewer.rs`
- `src-tauri/src/app/explorer.rs`

---

### 問題4, 5: イベントリスナー解除漏れ 🟡

**現状のコード (ViewerTab.tsx):**
```tsx
appWindow
  .listen('viewer-tab-state-changed', (event) => {
    // ...
  })
  .then((unListen) => (unListenRef = unListen));

onCleanup(() => {
  unListenRef && unListenRef();
});
```

**問題点:**
1. `listen()`は非同期で、Promiseが解決する前にタブが閉じられると`unListenRef`が`undefined`のまま
2. 結果としてイベントリスナーが解除されない
3. タブの素早い開閉で累積

**修正方針:**
- `onMount`内で`await`を使用してリスナー登録を待つ
- または解除関数の配列を使用して確実にクリーンアップ

**修正後のコード:**
```tsx
let unListenRef: UnlistenFn | undefined;

onMount(async () => {
  unListenRef = await appWindow.listen('viewer-tab-state-changed', (event) => {
    // ...
  });
});

onCleanup(() => {
  unListenRef?.();
});
```

**修正ファイル:**
- `src/pages/viewer/ViewerTab.tsx`
- `src/pages/explorer/ExplorerTab.tsx`

---

### 問題6: タブごとのFileTree重複保持 🟡

**現状のコード:**
```rust
pub struct ViewerTabState {
    pub tree: Vec<FileTree>, // ディレクトリ全体のツリー構造を各タブで保持
}
```

**問題点:**
- 各タブが`tree`全体をメモリに保持
- 大きなディレクトリ（数千〜数万ファイル）を開くと、各タブで大量のメモリを消費
- 同じディレクトリを複数タブで開くとデータが重複

**修正方針:**
- 今回は影響度を考慮し、優先度を下げる
- 将来的には`Arc<Vec<FileTree>>`による共有、またはディレクトリパスをキーとしたキャッシュを検討

**対応:** 今回は見送り（将来課題として記録）

---

### 問題7: video.jsのdispose処理欠如 🟡

**現状のコード:**
```tsx
<Match when={props.viewing?.file_type === 'Video'}>
  <video
    class="video-js vjs-theme-fantasy w-full h-full object-contain"
    controls
    preload="auto"
    src={data()}
  />
</Match>
```

**問題点:**
1. video.jsのCSSをインポートしているが、実際には`videojs()`での初期化がない
2. 素のHTML5 `<video>`要素を使用している
3. 動画切り替え時にブラウザの動画バッファがメモリに残る可能性

**修正方針:**
- video.jsを正しく初期化するか、video.jsを削除して軽量化するか選択
- 今回は軽量化を選択し、video.jsのCSSのみ使用を継続
- `<video>`要素に`ref`を設定し、ソース変更時に明示的に`load()`を呼ぶ

**修正ファイル:**
- `src/pages/viewer/ViewerTab.tsx`

---

### 問題8: 非同期ロックの長時間保持 🟢

**現状のコード:**
```rust
pub(crate) async fn move_explorer_forward(...) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await; // ロック取得
    // ...
    let thumbnails = explore_path(&path, page)?; // I/O処理（ロック保持中）
    let end = get_page_count(&path).await?; // 非同期I/O（ロック保持中）
    // ...
}
```

**問題点:**
1. ファイルI/O処理中にMutexロックを保持し続ける
2. 重い処理中に他のコマンドがブロックされる

**修正方針:**
- 今回は影響度を考慮し、優先度を下げる
- 将来的にはロックを最小限に保持するパターンに変更

**対応:** 今回は見送り（将来課題として記録）

---

## 実装順序

1. **問題1**: watcher管理をAppStateに移行（最優先・クラッシュの主要因）
2. **問題2, 3**: ZIP読み込みをストリーミング化
3. **問題4, 5**: イベントリスナー解除を確実に
4. **問題7**: video要素のリソース管理改善

---

## 将来の改善課題

- [ ] FileTreeの共有参照化（問題6）
- [ ] Mutexロック保持時間の最適化（問題8）
- [ ] ZIPファイルのLRUキャッシュ導入
- [ ] 画像データのメモリキャッシュ上限設定
