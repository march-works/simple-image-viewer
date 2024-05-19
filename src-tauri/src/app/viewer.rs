use base64::{engine::general_purpose, Engine as _};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use std::{io::Read, path::Path};
use tauri::{AppHandle, Manager, State, Window};

use crate::service::app_state::{
    add_tab_state, add_window_state, find_key_in_tree, get_next_in_tree, get_prev_in_tree,
    open_file_pick_dialog, remove_tab_state, ActiveTab, ActiveWindow, AppState, File,
};

#[tauri::command]
pub(crate) fn subscribe_dir_notification(filepath: String, app: AppHandle) {
    let path_inner = filepath.clone();
    recommended_watcher(move |res| match res {
        Ok(_) => {
            app.emit_all("directory-tree-changed", &path_inner)
                .unwrap_or_default();
        }
        Err(_) => {
            app.emit_all(
                "directory-watch-error",
                "Error occured while directory watching",
            )
            .unwrap_or_default();
        }
    })
    .map_or_else(
        |_| (),
        |mut watcher| {
            watcher
                .watch(Path::new(&filepath), RecursiveMode::Recursive)
                .unwrap_or(())
        },
    );
}

#[tauri::command]
pub(crate) fn open_file_image(filepath: String) -> String {
    let img = std::fs::read(filepath).unwrap_or_default();
    general_purpose::STANDARD_NO_PAD.encode(img)
}

#[tauri::command]
pub(crate) fn get_filenames_inner_zip(filepath: String) -> Vec<String> {
    let file = std::fs::read(filepath).unwrap_or_default();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(file));
    let mut files = zip
        .map(|f| f.file_names().map(|s| s.into()).collect::<Vec<String>>())
        .unwrap_or_default();
    files.sort();
    files
}

#[tauri::command]
pub(crate) fn read_image_in_zip(path: String, filename: String) -> String {
    let file = std::fs::read(path).unwrap_or_default();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(file));
    match zip {
        Ok(mut e) => {
            let inner = e.by_name(&filename);
            match inner {
                Ok(mut f) => {
                    let mut buf = vec![];
                    f.read_to_end(&mut buf).unwrap_or_default();
                    general_purpose::STANDARD_NO_PAD.encode(&buf)
                }
                Err(_) => "".into(),
            }
        }
        Err(_) => "".into(),
    }
}

#[tauri::command]
pub(crate) async fn change_active_window<'a>(
    window: Window,
    state: State<'a, AppState>,
) -> Result<(), String> {
    let mut label = state.active.lock().await;
    *label = ActiveWindow {
        label: window.label().to_string(),
    };
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_window<'a>(
    path: Option<String>,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let label = add_window_state(&state).await?;
    if let Some(path) = &path {
        let window_state = add_tab_state(path, &label, &state).await?;
        app.emit_to(&label, "window-state-changed", &window_state)
            .map_err(|_| "failed to emit window state".to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_tab<'a>(
    path: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let active = state.active.lock().await;
    let window_state = add_tab_state(&path, &active.label, &state).await?;
    app.emit_to(&active.label, "window-state-changed", &window_state)
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn remove_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let window_state = remove_tab_state(&label, &key, &state).await?;
    app.emit_to(&label, "window-state-changed", window_state.clone())
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_active_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    window_state.active = Some(ActiveTab { key: key.clone() });
    app.emit_to(&label, "window-state-changed", window_state.clone())
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_dialog<'a>(
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let filepath = open_file_pick_dialog()?;
    let label = state.active.lock().await.label.clone();
    let window_state = add_tab_state(&filepath, &label, &state).await?;
    app.emit_to(&label, "window-state-changed", &window_state)
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_state<'a>(
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    app.emit_to(&label, "window-state-changed", window_state.clone())
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_tab_state<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    let tab_state = (*window_state)
        .tabs
        .iter_mut()
        .find(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())?;
    app.emit_to(&label, "tab-state-changed", tab_state.clone())
        .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_viewing(
    tab_key: String,
    key: String,
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    let index = window_state
        .tabs
        .iter()
        .position(|t| t.key == tab_key)
        .ok_or_else(|| "tab not found".to_string())?;
    let tree = &window_state.tabs[index].tree;
    let viewing = find_key_in_tree(tree, &key);
    window_state.tabs[index].viewing = viewing;
    app.emit_to(
        &label,
        "tab-state-changed",
        window_state.tabs[index].clone(),
    )
    .map_err(|_| "failed to emit window state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_forward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    let tab_state = (*window_state)
        .tabs
        .iter_mut()
        .find(|t| t.key == window_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let old_viewing = tab_state.viewing.clone();
    let viewing = if let Some(File { key, .. }) = old_viewing {
        get_next_in_tree(&key, &tab_state.tree)
    } else {
        None
    };
    if viewing.is_some() {
        tab_state.viewing = viewing;
        app.emit_to(&label, "tab-state-changed", tab_state.clone())
            .map_err(|_| "failed to emit window state".to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_backward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    let tab_state = (*window_state)
        .tabs
        .iter_mut()
        .find(|t| t.key == window_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let old_viewing = tab_state.viewing.clone();
    let viewing = if let Some(File { key, .. }) = old_viewing {
        get_prev_in_tree(&key, &tab_state.tree)
    } else {
        None
    };
    if viewing.is_some() {
        tab_state.viewing = viewing;
        app.emit_to(&label, "tab-state-changed", tab_state.clone())
            .map_err(|_| "failed to emit window state".to_string())?;
    }
    Ok(())
}
