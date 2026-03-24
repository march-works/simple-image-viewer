# Tauri IPC Communication Patterns

このドキュメントは Tauri IPC 通信のパターンとコマンド一覧を定義します。

## 基本パターン

### Frontend → Backend (invoke)

```typescript
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// 基本的な呼び出し
await invoke('command_name', {
  label: appWindow.label,
  param1: value1,
});

// 戻り値を受け取る
const result = await invoke<ResultType>('get_data', { label: appWindow.label });
```

### Backend → Frontend (emit)

```rust
// 特定のウィンドウに送信
app.emit_to(&label, "event-name", &payload)?;

// 全ウィンドウに送信
app.emit("global-event", &payload)?;
```

### Frontend でのイベント受信

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// ウィンドウ固有のイベントを購読
const unlisten = await appWindow.listen<PayloadType>('event-name', (event) => {
  console.log(event.payload);
});

// クリーンアップ
unlisten();
```

## 状態同期フロー

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (SolidJS)                      │
│                                                              │
│  User Action → invoke('command', { label, ...params })       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                   Backend (Rust/Tauri)                       │
│                                                              │
│  #[tauri::command]                                           │
│  async fn command(...) {                                     │
│      service::update_state(&state)?;  // 状態変更            │
│      app.emit_to(&label, "state-changed", &new_state)?;     │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (SolidJS)                      │
│                                                              │
│  appWindow.listen('state-changed', (event) => {              │
│      setSignal(event.payload);  // UI 更新                   │
│  });                                                         │
└─────────────────────────────────────────────────────────────┘
```

## Viewer コマンド一覧

| コマンド                       | 説明                     | パラメータ                             |
| ------------------------------ | ------------------------ | -------------------------------------- |
| `open_new_viewer_tab`          | 新規タブでファイルを開く | `path: String`                         |
| `change_active_viewer_tab`     | アクティブタブを変更     | `label: String, index: usize`          |
| `remove_viewer_tab`            | タブを閉じる             | `label: String, index: usize`          |
| `change_viewing`               | 表示ファイルを変更       | `label: String, key: String`           |
| `move_forward`                 | 次のファイルへ           | `label: String`                        |
| `move_backward`                | 前のファイルへ           | `label: String`                        |
| `open_image_dialog`            | 画像選択ダイアログ       | なし                                   |
| `read_image_in_zip`            | ZIP 内の画像を読み込み   | `zip_path: String, entry_name: String` |
| `subscribe_dir_notification`   | ディレクトリ監視を開始   | `filepath: String, tab_key: String`    |
| `unsubscribe_dir_notification` | ディレクトリ監視を終了   | `filepath: String, tab_key: String`    |

### Viewer イベント

| イベント               | 説明                   | ペイロード      |
| ---------------------- | ---------------------- | --------------- |
| `viewer-state-changed` | Viewer 状態が変化      | `ViewerState`   |
| `directory-changed`    | 監視ディレクトリが変化 | `Vec<FileInfo>` |

## Explorer コマンド一覧

| コマンド                    | 説明                      | パラメータ                                |
| --------------------------- | ------------------------- | ----------------------------------------- |
| `open_new_explorer`         | 新規 Explorer を開く      | `path: Option<String>`                    |
| `open_new_explorer_tab`     | 新規タブを追加            | `label: String, path: Option<String>`     |
| `change_explorer_path`      | パスを変更                | `label: String, path: String`             |
| `move_explorer_forward`     | 履歴を進む                | `label: String`                           |
| `move_explorer_backward`    | 履歴を戻る                | `label: String`                           |
| `transfer_folder`           | フォルダを転送            | `from: String, to: String, label: String` |
| `change_explorer_sort`      | ソート設定を変更          | `label: String, sort: SortConfig`         |
| `get_thumbnails`            | サムネイルを取得          | `paths: Vec<String>, label: String`       |
| `get_recommendation_scores` | ML レコメンドスコアを取得 | `query: String, label: String`            |

### Explorer イベント

| イベント                          | 説明                                  | ペイロード           |
| --------------------------------- | ------------------------------------- | -------------------- |
| `explorer-state-changed`          | Explorer 状態が変化                   | `ExplorerState`      |
| `active-viewer-directory-changed` | Viewer のアクティブディレクトリが変化 | `String` (パス)      |
| `thumbnails-ready`                | サムネイル生成完了                    | `Vec<ThumbnailData>` |

## ウィンドウ識別子

### 命名規則

- Viewer: `viewer-{n}` (例: `viewer-0`, `viewer-1`)
- Explorer: `explorer-{n}` (例: `explorer-0`, `explorer-1`)

### ウィンドウラベルの使用

```typescript
// Frontend: 現在のウィンドウラベルを取得
const appWindow = getCurrentWindow();
const label = appWindow.label;  // "viewer-0" など

// Backend: ラベルで対象を特定
app.emit_to(&label, "event", payload);
```

## クロスウィンドウ通知

### Viewer → Explorer 通知

```rust
// Viewer のアクティブディレクトリ変更時、全 Explorer に通知
pub async fn notify_active_directory_changed(
    app: &AppHandle,
    state: &AppState,
    directory: &str,
) -> Result<(), String> {
    let explorers = state.explorers.lock().await;

    for explorer in explorers.values() {
        app.emit_to(&explorer.label, "active-viewer-directory-changed", directory)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

### Explorer → Viewer 通知

```rust
// Explorer からファイル選択時、Viewer に通知
app.emit_to("viewer-0", "open-file-request", &file_path)?;
```

## エラーハンドリング

### Frontend

```typescript
try {
  await invoke('command_name', { label, param });
} catch (error) {
  // エラーは文字列として返される
  console.error('Command failed:', error);
  setErrorMessage(error as string);
}
```

### Backend

```rust
#[tauri::command]
pub async fn my_command(...) -> Result<(), String> {
    // エラーを文字列に変換して返す
    operation()
        .map_err(|e| format!("Operation failed: {}", e))?;
    Ok(())
}
```

## ディレクトリ監視パターン

### 購読開始

```typescript
// タブオープン時
await invoke('subscribe_dir_notification', {
  filepath: directoryPath,
  tabKey: tab.key,
});
```

### 購読解除

```typescript
// タブクローズ時
await invoke('unsubscribe_dir_notification', {
  filepath: directoryPath,
  tabKey: tab.key,
});
```

### 変更通知の受信

```typescript
appWindow.listen<FileInfo[]>('directory-changed', (event) => {
  setFileList(event.payload);
});
```

## ベストプラクティス

1. **常に label を渡す**: マルチウィンドウ対応のため
2. **イベントリスナーは必ずクリーンアップ**: メモリリーク防止
3. **エラーはユーザーフレンドリーに**: バックエンドで適切なエラーメッセージを生成
4. **購読は参照カウント**: 同じリソースの重複監視を防止
5. **状態変更は必ず emit で通知**: Frontend と Backend の状態同期を維持
