use fs_extra::dir::{move_dir, CopyOptions};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use std::path::Path;
use tauri::{AppHandle, Emitter, State, WebviewUrl, WebviewWindowBuilder};

use crate::service::app_state::{
    add_explorer_state, add_explorer_tab_state, clear_thumbnail_cache_for_dir,
    explore_path_with_count, remove_explorer_tab_state, reset_explorer_tab_state, ActiveTab,
    AppState,
};

/// アクティブタブのインデックスを取得するヘルパー関数
async fn get_active_tab_index(
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
async fn get_tab_index_by_key(
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
async fn update_tab_and_emit(
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
    let (index, _) = get_active_tab_index(&label, &state).await?;
    let (path, mut page) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        (
            explorer_state.tabs[index].path.clone().unwrap_or_default(),
            explorer_state.tabs[index].page,
        )
    };

    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
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
    // 1. ロック外でI/O実行
    let (thumbnails, total_pages) =
        explore_path_with_count(&path, 1, state.thumbnail_cache.clone()).await?;

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
    let index = get_tab_index_by_key(&label, &key, &state).await?;
    let path = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        explorer_state.tabs[index].path.clone().unwrap_or_default()
    };

    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
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
    let (index, _) = get_active_tab_index(&label, &state).await?;
    let (path, page) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        (
            explorer_state.tabs[index].path.clone().unwrap_or_default(),
            explorer_state.tabs[index].page + 1,
        )
    };

    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
    if page > total_pages {
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
    let (index, _) = get_active_tab_index(&label, &state).await?;
    let (path, page) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        (
            explorer_state.tabs[index].path.clone().unwrap_or_default(),
            explorer_state.tabs[index].page.saturating_sub(1),
        )
    };

    if page == 0 {
        return Ok(());
    }

    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_end(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, _) = get_active_tab_index(&label, &state).await?;
    let path = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        explorer_state.tabs[index].path.clone().unwrap_or_default()
    };

    let (_, total_pages) = explore_path_with_count(&path, 1, state.thumbnail_cache.clone()).await?;
    let page = total_pages;
    let (thumbnails, _) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;

    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_start(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (index, _) = get_active_tab_index(&label, &state).await?;
    let path = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        explorer_state.tabs[index].path.clone().unwrap_or_default()
    };

    let page = 1;
    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
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
    let mut watchers = state.watchers.lock().await;

    // 既に同じパスの監視がある場合は参照カウントを増やすだけ
    if let Some((_, ref_count)) = watchers.get_mut(&dir_path) {
        *ref_count += 1;
        return Ok(());
    }

    // 新しいwatcherを作成
    let path_inner = dir_path.clone();
    let cache = state.thumbnail_cache.clone();
    let watcher = recommended_watcher(move |res| match res {
        Ok(_) => {
            // ディレクトリ変更時にキャッシュをクリア
            let cache_clone = cache.clone();
            let path_clone = path_inner.clone();
            tokio::spawn(async move {
                clear_thumbnail_cache_for_dir(&path_clone, cache_clone).await;
            });

            // フロントエンドに通知
            app.emit("explorer-directory-changed", &path_inner)
                .unwrap_or_default();
        }
        Err(_) => {
            app.emit(
                "explorer-directory-watch-error",
                "Error occurred while directory watching",
            )
            .unwrap_or_default();
        }
    })
    .map_err(|e| format!("failed to create watcher: {}", e))?;

    let mut watcher = watcher;
    watcher
        .watch(Path::new(&dir_path), RecursiveMode::NonRecursive)
        .map_err(|e| format!("failed to watch directory: {}", e))?;

    watchers.insert(dir_path, (watcher, 1));
    Ok(())
}

/// Explorerのディレクトリ監視を解除する
/// 参照カウントが0になった場合のみwatcherを削除
#[tauri::command]
pub(crate) async fn unsubscribe_explorer_dir_notification(
    dir_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers = state.watchers.lock().await;

    if let Some((_, ref_count)) = watchers.get_mut(&dir_path) {
        *ref_count -= 1;
        if *ref_count == 0 {
            watchers.remove(&dir_path);
        }
    }
    Ok(())
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
    let index = get_tab_index_by_key(&label, &key, &state).await?;
    let (path, page) = {
        let explorers = state.explorers.lock().await;
        let explorer_state = explorers.iter().find(|w| w.label == label).unwrap();
        (
            explorer_state.tabs[index].path.clone().unwrap_or_default(),
            explorer_state.tabs[index].page,
        )
    };

    let (thumbnails, total_pages) =
        explore_path_with_count(&path, page, state.thumbnail_cache.clone()).await?;
    update_tab_and_emit(&label, index, page, thumbnails, total_pages, &state, &app).await?;
    Ok(())
}
