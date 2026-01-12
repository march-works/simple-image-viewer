use fs_extra::dir::{move_dir, CopyOptions};
use notify::RecursiveMode;
use tauri::{AppHandle, Emitter, State, WebviewUrl, WebviewWindowBuilder};

use crate::service::app_state::{
    add_explorer_state, add_explorer_tab_state, explore_path_with_count, remove_explorer_tab_state,
    reset_explorer_tab_state, ActiveTab, AppState,
};
use crate::utils::watcher_utils::{
    create_explorer_watcher_callback, subscribe_directory, unsubscribe_directory,
};

use super::explorer_helpers::{
    emit_current_tab_state, get_active_tab_state_query, get_tab_index_by_key,
    get_tab_state_query_by_key, update_tab_and_emit,
};
use super::explorer_types::SortConfig;

#[tauri::command]
pub(crate) async fn transfer_folder(
    from: String,
    to: String,
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let options = CopyOptions::new();
    move_dir(from, to, &options).map_err(|_| "failed to move folder")?;

    // フォルダ移動した結果の表示更新
    let (index, (path, mut page, sort, search_query)) =
        get_active_tab_state_query(&label, &state, |p| p).await?;

    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    if page > total_pages {
        page = total_pages;
    }

    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_explorer<'a>(
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let label = add_explorer_state(&state).await?;
    let explorer_state = add_explorer_tab_state(&label, &state).await?;
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("explorer.html".into()))
        .title(super::get_explorer_title())
        .build()
        .map_err(|_| "system unavailable".to_string())?;
    app.emit_to(&label, "explorer-state-changed", &explorer_state)
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_explorer_tab<'a>(
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let explorer_state = add_explorer_tab_state(&label, &state).await?;
    app.emit_to(&label, "explorer-state-changed", &explorer_state)
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn remove_explorer_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let explorer_state = remove_explorer_tab_state(&label, &key, &state).await?;
    app.emit_to(&label, "explorer-state-changed", explorer_state.clone())
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_active_explorer_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    explorer_state.active = Some(ActiveTab { key: key.clone() });
    app.emit_to(&label, "explorer-state-changed", explorer_state.clone())
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_explorer_state<'a>(
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    app.emit_to(&label, "explorer-state-changed", explorer_state.clone())
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_explorer_tab_state<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let tab_state = explorer_state
        .tabs
        .iter_mut()
        .find(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())?;
    app.emit_to(&label, "explorer-tab-state-changed", tab_state.clone())
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_explorer_path(
    path: String,
    label: String,
    key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Get sort/search from current tab state
    let (sort, search_query) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        let tab = explorer_state
            .tabs
            .iter()
            .find(|t| t.key == key)
            .ok_or_else(|| "tab not found".to_string())?;
        (tab.sort.clone(), tab.search_query.clone())
    };

    // 1. ロック外でI/O実行
    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        1,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;

    // 2. ロック取得して状態更新のみ
    let tab_state = {
        let mut explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter_mut()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        let index = explorer_state
            .tabs
            .iter()
            .position(|t| t.key == key)
            .ok_or_else(|| "tab not found".to_string())?;
        explorer_state.tabs[index].path = Some(path);
        explorer_state.tabs[index].folders = thumbnails;
        explorer_state.tabs[index].end = total_pages;
        explorer_state.tabs[index].page = 1;
        explorer_state.tabs[index].clone()
    };

    app.emit_to(&label, "explorer-tab-state-changed", &tab_state)
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_explorer_transfer_path(
    transfer_path: String,
    label: String,
    key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())?;
    explorer_state.tabs[index].transfer_path = Some(transfer_path);
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_explorer_page(
    page: usize,
    label: String,
    key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, _, sort, search_query)) =
        get_tab_state_query_by_key(&label, &key, &state).await?;

    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_explorer_tab(
    label: String,
    key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let explorer_state = reset_explorer_tab_state(&label, &key, &state).await?;
    app.emit_to(&label, "explorer-tab-state-changed", explorer_state.clone())
        .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_forward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, page, sort, search_query)) =
        get_active_tab_state_query(&label, &state, |p| p + 1).await?;

    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    if page > total_pages {
        // 範囲外の場合は現在のページ状態をemitしてローディングを解除
        emit_current_tab_state(&label, index, &state, &app).await?;
        return Ok(());
    }

    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_backward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, page, sort, search_query)) =
        get_active_tab_state_query(&label, &state, |p| p.saturating_sub(1)).await?;

    if page == 0 {
        // 範囲外の場合は現在のページ状態をemitしてローディングを解除
        emit_current_tab_state(&label, index, &state, &app).await?;
        return Ok(());
    }

    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_end(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, _, sort, search_query)) =
        get_active_tab_state_query(&label, &state, |_| 1).await?;

    let (_, total_pages) = explore_path_with_count(
        &path,
        1,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    let page = total_pages;
    let (thumbnails, _) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;

    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_start(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, _, sort, search_query)) =
        get_active_tab_state_query(&label, &state, |_| 1).await?;

    let page = 1;
    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

/// Explorerのディレクトリ監視を開始する
/// watcherはAppStateで管理し、同じパスへの監視は参照カウントで共有する
#[tauri::command]
pub(crate) async fn subscribe_explorer_dir_notification(
    dir_path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let callback =
        create_explorer_watcher_callback(app, dir_path.clone(), state.thumbnail_cache.clone());

    subscribe_directory(dir_path, &state, RecursiveMode::NonRecursive, callback).await
}

/// Explorerのディレクトリ監視を解除する
/// 参照カウントが0になった場合のみwatcherを削除
#[tauri::command]
pub(crate) async fn unsubscribe_explorer_dir_notification(
    dir_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    unsubscribe_directory(dir_path, &state).await
}

/// ディレクトリ変更通知を受けてExplorerタブの内容を再読み込みする
/// フロントエンドから explorer-directory-changed イベントを受けた際に呼び出される
#[tauri::command]
pub(crate) async fn refresh_explorer_tab(
    label: String,
    key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, (path, page, sort, search_query)) =
        get_tab_state_query_by_key(&label, &key, &state).await?;

    let (thumbnails, total_pages) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
    )
    .await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

/// ソート設定を変更する
#[tauri::command]
pub(crate) async fn change_explorer_sort(
    label: String,
    key: String,
    sort: SortConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let index = get_tab_index_by_key(&label, &key, &state).await?;
    let (path, search_query) = {
        let mut explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter_mut()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        explorer_state.tabs[index].sort = sort.clone();
        (
            explorer_state.tabs[index].path.clone(),
            explorer_state.tabs[index].search_query.clone(),
        )
    };

    // パスがない場合（デバイス一覧）はソートしない
    if let Some(path) = path {
        let (thumbnails, total_pages) = explore_path_with_count(
            &path,
            1,
            state.thumbnail_cache.clone(),
            &sort,
            search_query.as_deref(),
        )
        .await?;
        update_tab_and_emit(&label, index, 1, thumbnails, total_pages, &state, &app).await?;
    } else {
        // デバイス一覧の場合は状態のみ更新
        emit_current_tab_state(&label, index, &state, &app).await?;
    }
    Ok(())
}

/// 検索クエリを変更する
#[tauri::command]
pub(crate) async fn change_explorer_search(
    label: String,
    key: String,
    query: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let index = get_tab_index_by_key(&label, &key, &state).await?;
    let (path, sort) = {
        let mut explorers = state.explorers.lock().await;
        let explorer_state = explorers
            .iter_mut()
            .find(|w| w.label == label)
            .ok_or_else(|| "explorer not found".to_string())?;
        explorer_state.tabs[index].search_query = query.clone();
        (
            explorer_state.tabs[index].path.clone(),
            explorer_state.tabs[index].sort.clone(),
        )
    };

    // パスがない場合（デバイス一覧）は検索しない
    if let Some(path) = path {
        let (thumbnails, total_pages) = explore_path_with_count(
            &path,
            1,
            state.thumbnail_cache.clone(),
            &sort,
            query.as_deref(),
        )
        .await?;
        update_tab_and_emit(&label, index, 1, thumbnails, total_pages, &state, &app).await?;
    } else {
        // デバイス一覧の場合は状態のみ更新
        emit_current_tab_state(&label, index, &state, &app).await?;
    }
    Ok(())
}
