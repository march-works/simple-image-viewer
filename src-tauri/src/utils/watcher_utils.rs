use notify::{recommended_watcher, Event, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

use crate::service::app_state::AppState;

/// ディレクトリ監視を開始するヘルパー関数
///
/// # Arguments
/// * `dir_path` - 監視するディレクトリパス
/// * `state` - AppState
/// * `recursive_mode` - 監視モード (Recursive または NonRecursive)
/// * `on_change` - ファイル変更時のコールバック
pub async fn subscribe_directory<F>(
    dir_path: String,
    state: &State<'_, AppState>,
    recursive_mode: RecursiveMode,
    on_change: F,
) -> Result<(), String>
where
    F: Fn(NotifyResult<Event>) + Send + 'static,
{
    let mut watchers_guard = state.watchers.lock().await;

    // 既に同じパスの監視がある場合は参照カウントを増やすだけ
    if let Some((_, ref_count)) = watchers_guard.get_mut(&dir_path) {
        *ref_count += 1;
        return Ok(());
    }

    // 新しいwatcherを作成
    let watcher =
        recommended_watcher(on_change).map_err(|e| format!("failed to create watcher: {}", e))?;

    let mut watcher = watcher;
    watcher
        .watch(Path::new(&dir_path), recursive_mode)
        .map_err(|e| format!("failed to watch directory: {}", e))?;

    watchers_guard.insert(dir_path, (watcher, 1));
    Ok(())
}

/// ディレクトリ監視を解除するヘルパー関数
/// 参照カウントが0になった場合のみwatcherを削除
pub async fn unsubscribe_directory(
    dir_path: String,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers_guard = state.watchers.lock().await;

    if let Some((_, ref_count)) = watchers_guard.get_mut(&dir_path) {
        *ref_count -= 1;
        if *ref_count == 0 {
            watchers_guard.remove(&dir_path);
        }
    }
    Ok(())
}

/// Explorerタイプのwatcherコールバックを生成
pub fn create_explorer_watcher_callback(
    app: AppHandle,
    path: String,
    cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
) -> impl Fn(NotifyResult<Event>) + Send + 'static {
    move |res| match res {
        Ok(_) => {
            // ディレクトリ変更時にキャッシュをクリア
            let cache_clone = cache.clone();
            let path_clone = path.clone();
            tokio::spawn(async move {
                crate::service::explorer_state::clear_thumbnail_cache_for_dir(
                    &path_clone,
                    cache_clone,
                )
                .await;
            });

            // フロントエンドに通知
            app.emit("explorer-directory-changed", &path)
                .unwrap_or_default();
        }
        Err(_) => {
            app.emit(
                "explorer-directory-watch-error",
                "Error occurred while directory watching",
            )
            .unwrap_or_default();
        }
    }
}

/// Viewerタイプのwatcherコールバックを生成
pub fn create_viewer_watcher_callback(
    app: AppHandle,
    path: String,
) -> impl Fn(NotifyResult<Event>) + Send + 'static {
    move |res| match res {
        Ok(_) => {
            app.emit("directory-tree-changed", &path)
                .unwrap_or_default();
        }
        Err(_) => {
            app.emit(
                "directory-watch-error",
                "Error occured while directory watching",
            )
            .unwrap_or_default();
        }
    }
}
