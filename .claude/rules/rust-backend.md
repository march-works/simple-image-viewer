# Rust Backend Coding Rules

このドキュメントは Rust バックエンドのコーディング規約を定義します。

## Tauri コマンド定義

### 基本パターン

```rust
#[tauri::command]
pub(crate) async fn command_name(
    app: AppHandle,
    state: State<'_, AppState>,
    label: String,
    // その他のパラメータ
) -> Result<ReturnType, String> {
    // 1. service 層を呼び出して状態操作
    let result = service::do_something(&state, &label).await?;

    // 2. 必要に応じてイベントを emit
    app.emit_to(&label, "event-name", &result)
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    Ok(result)
}
```

### コマンド登録

```rust
// lib.rs
.invoke_handler(tauri::generate_handler![
    app::viewer::command_name,
    app::explorer::another_command,
])
```

## エラーハンドリング

### Tauri コマンド (app/ レイヤー)

```rust
// Result<T, String> を使用
pub(crate) async fn my_command(...) -> Result<(), String> {
    operation
        .map_err(|e| format!("Operation failed: {}", e))?;
    Ok(())
}
```

### 内部ユーティリティ (utils/ レイヤー)

```rust
// anyhow::Result を使用
use anyhow::{Context, Result};

pub fn process_file(path: &str) -> Result<Data> {
    let content = std::fs::read(path)
        .context("Failed to read file")?;
    // ...
}
```

### エラー変換

```rust
// anyhow → String 変換
service::process(&state)
    .await
    .map_err(|e| e.to_string())?;
```

## 非同期・状態管理

### Mutex の使用

```rust
// 状態のロック取得
let mut viewers = state.viewers.lock().await;

// 必要な操作を行う
viewers.insert(label.clone(), viewer_state);

// ロックは drop 時に自動解放
```

### RwLock の使用

```rust
// 読み取り専用
let cache = state.thumbnail_cache.read().await;
if let Some(data) = cache.get(&key) {
    return Ok(data.clone());
}

// 書き込み
let mut cache = state.thumbnail_cache.write().await;
cache.insert(key, data);
```

### デッドロック防止

```rust
// ❌ 複数のロックを同時に保持しない
let viewers = state.viewers.lock().await;
let explorers = state.explorers.lock().await; // デッドロックの可能性

// ✅ 必要な操作を分離
{
    let mut viewers = state.viewers.lock().await;
    // viewers 操作
} // ロック解放

{
    let mut explorers = state.explorers.lock().await;
    // explorers 操作
}
```

## シリアライゼーション

### IPC 用の型

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerState {
    pub label: String,
    pub tabs: Vec<TabState>,
    pub active_tab_index: usize,
}

// snake_case → camelCase 変換
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendData {
    pub file_name: String,      // → fileName
    pub is_active: bool,        // → isActive
}
```

## ファイルウォッチャー管理

### 参照カウントパターン

```rust
// 購読開始
pub async fn subscribe(state: &AppState, path: &str, tab_key: &str) {
    let mut watchers = state.watchers.lock().await;

    if let Some((_, count)) = watchers.get_mut(path) {
        *count += 1;  // 既存の watcher の参照カウントを増加
    } else {
        let watcher = create_watcher(path)?;
        watchers.insert(path.to_string(), (watcher, 1));
    }
}

// 購読解除
pub async fn unsubscribe(state: &AppState, path: &str, tab_key: &str) {
    let mut watchers = state.watchers.lock().await;

    if let Some((_, count)) = watchers.get_mut(path) {
        *count -= 1;
        if *count == 0 {
            watchers.remove(path);  // 参照がなくなったら削除
        }
    }
}
```

## ファイル種別判定

```rust
use crate::utils::file_utils::{is_image_file, is_video_file, is_zip_file};

// 拡張子による判定
if is_image_file(&path) {
    // 画像処理
} else if is_video_file(&path) {
    // 動画処理
} else if is_zip_file(&path) {
    // ZIP 処理
}
```

## 命名規則

| 種類       | 規則                 | 例                    |
| ---------- | -------------------- | --------------------- |
| 関数       | snake_case           | `open_new_viewer_tab` |
| 構造体     | PascalCase           | `ViewerState`         |
| 定数       | SCREAMING_SNAKE_CASE | `MAX_THUMBNAIL_SIZE`  |
| モジュール | snake_case           | `viewer_state`        |

## テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert!(is_image_file("photo.jpg"));
        assert!(is_image_file("photo.PNG"));
        assert!(!is_image_file("video.mp4"));
    }

    #[tokio::test]
    async fn test_async_operation() {
        // 非同期テスト
    }
}
```

## マクロ使用

### cfg マクロ

```rust
// 開発・本番の分岐
let app_name = if cfg!(debug_assertions) {
    "Simple Image Viewer [DEV]"
} else {
    "Simple Image Viewer"
};

// OS 別処理
#[cfg(target_os = "windows")]
fn platform_specific() {
    // Windows 専用
}
```
