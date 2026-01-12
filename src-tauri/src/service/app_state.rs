//! アプリケーション状態管理の共通部分

use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{Mutex, RwLock};

use crate::utils::file_utils::get_any_extensions;

use super::explorer_state::ExplorerState;
use super::viewer_state::ViewerState;

// ========================================
// 共通型定義
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveViewer {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTab {
    pub key: String,
}

// ========================================
// アプリケーション状態
// ========================================

pub struct AppState {
    pub count: Mutex<i32>,
    pub active: Mutex<ActiveViewer>,
    pub viewers: Mutex<Vec<ViewerState>>,
    pub explorers: Mutex<Vec<ExplorerState>>,
    /// ディレクトリ監視のwatcher管理 (path -> (watcher, 参照カウント))
    pub watchers: Mutex<HashMap<String, (RecommendedWatcher, usize)>>,
    /// サムネイルキャッシュ (folder_path -> thumbnail_path)
    pub thumbnail_cache: Arc<RwLock<HashMap<String, String>>>,
}

// ========================================
// 共通ユーティリティ
// ========================================

pub(crate) async fn open_file_pick_dialog(app: &tauri::AppHandle) -> Result<String, String> {
    use tokio::sync::oneshot;

    let extensions = get_any_extensions();
    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .add_filter(
            "File",
            &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    match rx.await {
        Ok(Some(path)) => Ok(path.to_string()),
        Ok(None) => Err("no file selected".to_string()),
        Err(_) => Err("dialog cancelled".to_string()),
    }
}
