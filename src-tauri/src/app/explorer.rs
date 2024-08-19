use fs_extra::dir::{move_dir, CopyOptions};
use tauri::{AppHandle, Manager, State};

#[allow(unused_imports)]
use tokio_stream::StreamExt;

use crate::service::app_state::{
    add_explorer_state, add_explorer_tab_state, explore_path, get_page_count,
    remove_explorer_tab_state, reset_explorer_tab_state, ActiveTab, AppState,
};

pub struct StreamActivation(bool);

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
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == explorer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let mut page = explorer_state.tabs[index].page;
    let end = get_page_count(&path).await?;
    if page > end {
        page = end;
    }
    let thumbnails = explore_path(&path, page)?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_explorer<'a>(
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let label = add_explorer_state(&state).await?;
    let explorer_state = add_explorer_tab_state(&label, &state).await?;
    tauri::WindowBuilder::new(&app, &label, tauri::WindowUrl::App("explorer.html".into()))
        .title("Image Explorer")
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
    let thumbnails = explore_path(&path, 1)?;
    let end = get_page_count(&path).await?;
    explorer_state.tabs[index].path = Some(path);
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
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
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let thumbnails = explore_path(&path, page)?;
    let end = get_page_count(&path).await?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
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
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == explorer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let page = explorer_state.tabs[index].page + 1;
    let end = get_page_count(&path).await?;
    if page > end {
        return Ok(());
    }

    let thumbnails = explore_path(&path, page)?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_backward(
    label: String,
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
        .position(|t| t.key == explorer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let page = explorer_state.tabs[index].page - 1;
    if page == 0 {
        return Ok(());
    }
    let thumbnails = explore_path(&path, page)?;
    let end = get_page_count(&path).await?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_end(
    label: String,
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
        .position(|t| t.key == explorer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let end = get_page_count(&path).await?;
    let page = end;
    let thumbnails = explore_path(&path, page)?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_explorer_to_start(
    label: String,
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
        .position(|t| t.key == explorer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let path = explorer_state.tabs[index].path.clone().unwrap_or_default();
    let page = 1;
    let end = get_page_count(&path).await?;
    let thumbnails = explore_path(&path, page)?;
    explorer_state.tabs[index].page = page;
    explorer_state.tabs[index].folders = thumbnails;
    explorer_state.tabs[index].end = end;
    app.emit_to(
        &label,
        "explorer-tab-state-changed",
        &explorer_state.tabs[index],
    )
    .map_err(|_| "failed to emit explorer state".to_string())?;
    Ok(())
}
