use tauri::{AppHandle, Emitter, State};

use crate::app::explorer_types::SortConfig;
use crate::service::app_state::AppState;
use crate::service::explorer_state::Thumbnail;

/// タブ状態から (path, page, sort, search_query) を取得するタプル
pub type TabStateQuery = (String, usize, SortConfig, Option<String>);

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

/// アクティブタブの状態 (path, page, sort, search_query) を取得するヘルパー関数
pub(crate) async fn get_active_tab_state_query(
    label: &str,
    state: &State<'_, AppState>,
    page_modifier: impl FnOnce(usize) -> usize,
) -> Result<(usize, TabStateQuery), String> {
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
    let tab = &explorer_state.tabs[index];
    Ok((
        index,
        (
            tab.path.clone().unwrap_or_default(),
            page_modifier(tab.page),
            tab.sort.clone(),
            tab.search_query.clone(),
        ),
    ))
}

/// 指定されたキーのタブ状態 (path, page, sort, search_query) を取得するヘルパー関数
pub(crate) async fn get_tab_state_query_by_key(
    label: &str,
    key: &str,
    state: &State<'_, AppState>,
) -> Result<(usize, TabStateQuery), String> {
    let explorers = state.explorers.lock().await;
    let explorer_state = explorers
        .iter()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())?;
    let tab = &explorer_state.tabs[index];
    Ok((
        index,
        (
            tab.path.clone().unwrap_or_default(),
            tab.page,
            tab.sort.clone(),
            tab.search_query.clone(),
        ),
    ))
}

/// タブの状態を更新してイベントを発行するヘルパー関数
pub(crate) async fn update_tab_and_emit(
    label: &str,
    index: usize,
    page: usize,
    thumbnails: Vec<Thumbnail>,
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

/// 現在のタブ状態をそのままemitするヘルパー関数（ローディング解除用）
pub(crate) async fn emit_current_tab_state(
    label: &str,
    index: usize,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let tab_state = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        explorer_state.tabs[index].clone()
    };
    app.emit_to(label, "explorer-tab-state-changed", &tab_state)
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}
