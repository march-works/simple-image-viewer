use tauri::{AppHandle, Emitter, State};

use crate::service::app_state::AppState;

/// アクティブタブのインデックスを取得するヘルパー関数
pub(crate) async fn get_active_tab_index(
    label: &str,
    state: &State<'_, AppState>,
) -> Result<(usize, String), String> {
    let explorers = state.explorers.lock().await;
    let explorer_state = explorers
        .iter()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let active_key = explorer_state
        .active
        .as_ref()
        .ok_or_else(|| "no active tab".to_string())?
        .key
        .clone();
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == active_key)
        .ok_or_else(|| "tab not found".to_string())?;
    Ok((index, active_key))
}

/// 指定されたキーのタブインデックスを取得するヘルパー関数
pub(crate) async fn get_tab_index_by_key(
    label: &str,
    key: &str,
    state: &State<'_, AppState>,
) -> Result<usize, String> {
    let explorers = state.explorers.lock().await;
    let explorer_state = explorers
        .iter()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    explorer_state
        .tabs
        .iter()
        .position(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())
}

/// タブの状態を更新してイベントを発行するヘルパー関数
pub(crate) async fn update_tab_and_emit(
    label: &str,
    index: usize,
    page: usize,
    thumbnails: Vec<crate::service::app_state::Thumbnail>,
    total_pages: usize,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let tab_state = {
        let mut explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter_mut()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        explorer_state.tabs[index].page = page;
        explorer_state.tabs[index].folders = thumbnails;
        explorer_state.tabs[index].end = total_pages;
        explorer_state.tabs[index].clone()
    };

    app.emit_to(label, "explorer-tab-state-changed", &tab_state)
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}
