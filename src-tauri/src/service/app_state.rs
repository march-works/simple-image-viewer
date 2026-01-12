//! アプリケーション状態管理の共通ユーティリティ

use tauri_plugin_dialog::DialogExt;

use crate::utils::file_utils::get_any_extensions;

// types.rs からの再エクスポート（後方互換性）
pub use super::types::{ActiveTab, ActiveViewer, AppState};

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
