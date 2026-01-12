use base64::{engine::general_purpose, Engine as _};
use notify::RecursiveMode;
use std::fs::File as StdFile;
use std::io::{BufReader, Read};
use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::service::app_state::{open_file_pick_dialog, ActiveTab, ActiveViewer, AppState};
use crate::service::viewer_state::{
    add_viewer_state, add_viewer_tab_state, find_key_in_tree, get_next_in_tree, get_prev_in_tree,
    rebuild_file_tree, remove_viewer_tab_state, File,
};

use crate::utils::file_utils::normalize_path;
use crate::utils::watcher_utils::{
    create_viewer_watcher_callback, subscribe_directory, unsubscribe_directory,
};

/// Viewer状態変更時に全Explorerにアクティブディレクトリを通知する共通関数
async fn notify_active_directory_to_explorers(
    viewer_state: &crate::service::viewer_state::ViewerState,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let active_dir = viewer_state.active.as_ref().and_then(|active_tab| {
        viewer_state
            .tabs
            .iter()
            .find(|tab| tab.key == active_tab.key)
            .map(|tab| normalize_path(&tab.path))
    });

    let explorers = state.explorers.lock().await;
    for explorer in explorers.iter() {
        let _ = app.emit_to(
            &explorer.label,
            "active-viewer-directory-changed",
            active_dir.clone(),
        );
    }

    Ok(())
}

/// ディレクトリ監視を開始する
/// watcherはAppStateで管理し、同じパスへの監視は参照カウントで共有する
#[tauri::command]
pub(crate) async fn subscribe_dir_notification(
    filepath: String,
    _tab_key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let callback = create_viewer_watcher_callback(app, filepath.clone());

    subscribe_directory(filepath, &state, RecursiveMode::Recursive, callback).await
}

/// ディレクトリ監視を解除する
/// 参照カウントが0になった場合のみwatcherを削除
#[tauri::command]
pub(crate) async fn unsubscribe_dir_notification(
    filepath: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    unsubscribe_directory(filepath, &state).await
}

/// ディレクトリ変更通知を受けてファイルツリーを再構築する
/// フロントエンドから directory-tree-changed イベントを受けた際に呼び出される
#[tauri::command]
pub(crate) async fn refresh_viewer_tab_tree(
    tab_key: String,
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
        .find(|t| t.key == tab_key)
        .ok_or_else(|| "tab not found".to_string())?;

    // ZIPファイルかどうかを判定（viewing.file_typeで判断）
    let is_compressed = tab_state
        .viewing
        .as_ref()
        .map(|v| v.file_type == "Zip")
        .unwrap_or(false);

    // ファイルツリーを再構築
    let new_tree = rebuild_file_tree(&tab_state.path, is_compressed);

    // 現在表示中のファイルがまだ存在するか確認
    let current_key = tab_state.viewing.as_ref().map(|v| v.key.clone());
    let new_viewing = if let Some(key) = current_key {
        find_key_in_tree(&new_tree, &key)
    } else {
        None
    };

    tab_state.tree = new_tree;
    tab_state.viewing = new_viewing;

    // フロントエンドに更新を通知
    app.emit_to(&label, "viewer-tab-state-changed", tab_state.clone())
        .map_err(|_| "failed to emit viewer state".to_string())?;

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

    let mut inner = zip
        .by_name(&filename)
        .map_err(|e| format!("file not found in zip: {}", e))?;
    let mut buf = Vec::with_capacity(inner.size() as usize);
    inner
        .read_to_end(&mut buf)
        .map_err(|e| format!("failed to read file: {}", e))?;
    Ok(general_purpose::STANDARD_NO_PAD.encode(&buf))
}

#[tauri::command]
pub(crate) async fn change_active_viewer<'a>(
    window: WebviewWindow,
    state: State<'a, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut label = state.active.lock().await;
    *label = ActiveViewer {
        label: window.label().to_string(),
    };

    // アクティブなViewerのディレクトリを全Explorerに通知
    let active_label = label.label.clone();
    drop(label);

    let viewers = state.viewers.lock().await;
    let viewer_state = viewers
        .iter()
        .find(|v| v.label == active_label)
        .ok_or_else(|| "active viewer not found".to_string())?;

    notify_active_directory_to_explorers(viewer_state, &state, &app).await?;

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

    // タブを閉じた後の新しいアクティブディレクトリを全Explorerに通知
    notify_active_directory_to_explorers(&viewer_state, &state, &app).await?;

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

    // アクティブなタブのディレクトリを全Explorerに通知
    notify_active_directory_to_explorers(viewer_state, &state, &app).await?;

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

/// アクティブなViewerのアクティブなタブで開いているディレクトリパスを取得
#[tauri::command]
pub(crate) async fn get_active_viewer_directory(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let active = state.active.lock().await;
    let viewers = state.viewers.lock().await;

    let viewer_state = viewers
        .iter()
        .find(|v| v.label == active.label)
        .ok_or_else(|| "active viewer not found".to_string())?;

    let directory = viewer_state.active.as_ref().and_then(|active_tab| {
        viewer_state
            .tabs
            .iter()
            .find(|tab| tab.key == active_tab.key)
            .map(|tab| normalize_path(&tab.path))
    });

    Ok(directory)
}

/// 指定されたディレクトリを開いている全てのViewerタブを閉じる
#[tauri::command]
pub(crate) async fn close_viewer_tabs_by_directory(
    directory: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let normalized_dir = normalize_path(&directory);
    let mut viewers = state.viewers.lock().await;

    // 全てのViewerウィンドウを走査
    for viewer_state in viewers.iter_mut() {
        // 閉じるべきタブを特定
        let tabs_to_close: Vec<String> = viewer_state
            .tabs
            .iter()
            .filter(|tab| normalize_path(&tab.path) == normalized_dir)
            .map(|tab| tab.key.clone())
            .collect();

        if tabs_to_close.is_empty() {
            continue;
        }

        // タブを閉じる（後ろから削除してインデックスの問題を回避）
        for key in tabs_to_close.iter() {
            if let Some(index) = viewer_state.tabs.iter().position(|t| &t.key == key) {
                viewer_state.tabs.remove(index);
            }
        }

        // アクティブなタブが閉じられた場合、新しいアクティブタブを設定
        if !viewer_state.tabs.is_empty() {
            if let Some(active_tab) = &viewer_state.active {
                if tabs_to_close.contains(&active_tab.key) {
                    let new_active_key = viewer_state.tabs[0].key.clone();
                    viewer_state.active = Some(ActiveTab {
                        key: new_active_key,
                    });
                }
            }
        } else {
            viewer_state.active = None;
        }

        let label = viewer_state.label.clone();
        let viewer_state_clone = viewer_state.clone();

        app.emit_to(&label, "viewer-state-changed", viewer_state_clone)
            .map_err(|_| "failed to emit viewer state".to_string())?;
    }

    Ok(())
}

/// フォルダの閲覧を記録する (Phase 2: リコメンド基盤)
/// Viewer でファイルを開いた際に呼び出し、閲覧履歴とサムネイルを DB に保存
#[tauri::command]
pub(crate) async fn record_folder_view(
    folder_path: String,
    thumbnail_image_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::utils::thumbnail_utils::generate_thumbnail_data;

    let db = state.db.clone();

    // バックグラウンドでサムネイル生成と DB 保存を行う
    tokio::task::spawn_blocking(move || {
        // サムネイル画像パスが指定されている場合、サムネイルを生成
        let (thumbnail_blob, thumbnail_hash) = if let Some(ref img_path) = thumbnail_image_path {
            // 既存のハッシュと比較して変更がある場合のみ再生成
            let existing_hash = db.get_thumbnail_hash(&folder_path).ok().flatten();
            let current_hash = crate::utils::thumbnail_utils::calculate_image_hash(img_path).ok();

            if existing_hash.as_ref() != current_hash.as_ref() {
                // サムネイルを生成
                match generate_thumbnail_data(img_path) {
                    Ok(data) => (Some(data.blob), Some(data.hash)),
                    Err(e) => {
                        eprintln!("Failed to generate thumbnail: {}", e);
                        (None, None)
                    }
                }
            } else {
                // 変更なし、サムネイルは更新しない
                (None, None)
            }
        } else {
            (None, None)
        };

        // DB に記録
        if let Err(e) = db.record_folder_view(
            &folder_path,
            thumbnail_blob.as_deref(),
            thumbnail_hash.as_deref(),
        ) {
            eprintln!("Failed to record folder view: {}", e);
        }
    })
    .await
    .map_err(|e| format!("Failed to spawn blocking task: {}", e))?;

    Ok(())
}
