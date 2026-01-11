use base64::{engine::general_purpose, Engine as _};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use std::fs::File as StdFile;
use std::io::{BufReader, Read};
use std::path::Path;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::service::app_state::{
    add_viewer_state, add_viewer_tab_state, find_key_in_tree, get_next_in_tree, get_prev_in_tree,
    open_file_pick_dialog, remove_viewer_tab_state, ActiveTab, ActiveViewer, AppState, File,
};

/// ディレクトリ監視を開始する
/// watcherはAppStateで管理し、同じパスへの監視は参照カウントで共有する
#[tauri::command]
pub(crate) async fn subscribe_dir_notification(
    filepath: String,
    _tab_key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut watchers = state.watchers.lock().await;
    
    // 既に同じパスの監視がある場合は参照カウントを増やすだけ
    if let Some((_, ref_count)) = watchers.get_mut(&filepath) {
        *ref_count += 1;
        return Ok(());
    }
    
    // 新しいwatcherを作成
    let path_inner = filepath.clone();
    let watcher = recommended_watcher(move |res| match res {
        Ok(_) => {
            app.emit("directory-tree-changed", &path_inner)
                .unwrap_or_default();
        }
        Err(_) => {
            app.emit(
                "directory-watch-error",
                "Error occured while directory watching",
            )
            .unwrap_or_default();
        }
    })
    .map_err(|e| format!("failed to create watcher: {}", e))?;
    
    let mut watcher = watcher;
    watcher
        .watch(Path::new(&filepath), RecursiveMode::Recursive)
        .map_err(|e| format!("failed to watch directory: {}", e))?;
    
    watchers.insert(filepath, (watcher, 1));
    Ok(())
}

/// ディレクトリ監視を解除する
/// 参照カウントが0になった場合のみwatcherを削除
#[tauri::command]
pub(crate) async fn unsubscribe_dir_notification(
    filepath: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers = state.watchers.lock().await;
    
    if let Some((_, ref_count)) = watchers.get_mut(&filepath) {
        *ref_count -= 1;
        if *ref_count == 0 {
            watchers.remove(&filepath);
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn open_file_image(filepath: String) -> Result<String, String> {
    let img = std::fs::read(&filepath).map_err(|e| format!("failed to read image: {}", e))?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(img))
}

/// ZIPファイル内のファイル名一覧を取得（ストリーミング読み込み）
#[tauri::command]
pub(crate) fn get_filenames_inner_zip(filepath: String) -> Result<Vec<String>, String> {
    let file = StdFile::open(&filepath).map_err(|e| format!("failed to open zip: {}", e))?;
    let reader = BufReader::new(file);
    let zip = zip::ZipArchive::new(reader).map_err(|e| format!("failed to read zip: {}", e))?;
    let mut files: Vec<String> = zip.file_names().map(|s| s.into()).collect();
    files.sort();
    Ok(files)
}

/// ZIP内の画像をBase64で読み込み（ストリーミング読み込み）
#[tauri::command]
pub(crate) fn read_image_in_zip(path: String, filename: String) -> Result<String, String> {
    let file = StdFile::open(&path).map_err(|e| format!("failed to open zip: {}", e))?;
    let reader = BufReader::new(file);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("failed to read zip: {}", e))?;
    
    let mut inner = zip.by_name(&filename).map_err(|e| format!("file not found in zip: {}", e))?;
    let mut buf = Vec::with_capacity(inner.size() as usize);
    inner.read_to_end(&mut buf).map_err(|e| format!("failed to read file: {}", e))?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(&buf))
}

#[tauri::command]
pub(crate) async fn change_active_viewer<'a>(
    window: WebviewWindow,
    state: State<'a, AppState>,
) -> Result<(), String> {
    let mut label = state.active.lock().await;
    *label = ActiveViewer {
        label: window.label().to_string(),
    };
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_viewer<'a>(
    path: Option<String>,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let label = add_viewer_state(&state).await?;
    if let Some(path) = &path {
        let viewer_state = add_viewer_tab_state(path, &label, &state).await?;
        app.emit_to(&label, "viewer-state-changed", &viewer_state)
            .map_err(|_| "failed to emit viewer state".to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_new_viewer_tab<'a>(
    path: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let active = state.active.lock().await;
    let viewer_state = add_viewer_tab_state(&path, &active.label, &state).await?;
    app.emit_to(&active.label, "viewer-state-changed", &viewer_state)
        .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn remove_viewer_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let viewer_state = remove_viewer_tab_state(&label, &key, &state).await?;
    app.emit_to(&label, "viewer-state-changed", viewer_state.clone())
        .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_active_viewer_tab<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    viewer_state.active = Some(ActiveTab { key: key.clone() });
    app.emit_to(&label, "viewer-state-changed", viewer_state.clone())
        .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn open_image_dialog<'a>(
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let filepath = open_file_pick_dialog(&app).await?;
    let label = state.active.lock().await.label.clone();
    let viewer_state = add_viewer_tab_state(&filepath, &label, &state).await?;
    app.emit_to(&label, "viewer-state-changed", &viewer_state)
        .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_viewer_state<'a>(
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    app.emit_to(&label, "viewer-state-changed", viewer_state.clone())
        .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn request_restore_viewer_tab_state<'a>(
    key: String,
    label: String,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    let tab_state = viewer_state
        .tabs
        .iter_mut()
        .find(|t| t.key == key)
        .ok_or_else(|| "tab not found".to_string())?;
    app.emit_to(&label, "viewer-tab-state-changed", tab_state.clone())
        .map_err(|_| "failed to emit viewer state".to_string())?;
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
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    let index = viewer_state
        .tabs
        .iter()
        .position(|t| t.key == tab_key)
        .ok_or_else(|| "tab not found".to_string())?;
    let tree = &viewer_state.tabs[index].tree;
    let viewing = find_key_in_tree(tree, &key);
    viewer_state.tabs[index].viewing = viewing;
    app.emit_to(
        &label,
        "viewer-tab-state-changed",
        viewer_state.tabs[index].clone(),
    )
    .map_err(|_| "failed to emit viewer state".to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_forward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    let tab_state = viewer_state
        .tabs
        .iter_mut()
        .find(|t| t.key == viewer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let old_viewing = tab_state.viewing.clone();
    let viewing = if let Some(File { key, .. }) = old_viewing {
        get_next_in_tree(&key, &tab_state.tree)
    } else {
        None
    };
    if viewing.is_some() {
        tab_state.viewing = viewing;
        app.emit_to(&label, "viewer-tab-state-changed", tab_state.clone())
            .map_err(|_| "failed to emit viewer state".to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn move_backward(
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    let tab_state = viewer_state
        .tabs
        .iter_mut()
        .find(|t| t.key == viewer_state.active.as_ref().unwrap().key)
        .ok_or_else(|| "tab not found".to_string())?;
    let old_viewing = tab_state.viewing.clone();
    let viewing = if let Some(File { key, .. }) = old_viewing {
        get_prev_in_tree(&key, &tab_state.tree)
    } else {
        None
    };
    if viewing.is_some() {
        tab_state.viewing = viewing;
        app.emit_to(&label, "viewer-tab-state-changed", tab_state.clone())
            .map_err(|_| "failed to emit viewer state".to_string())?;
    }
    Ok(())
}
