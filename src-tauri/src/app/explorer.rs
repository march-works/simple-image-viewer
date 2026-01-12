use fs_extra::dir::{move_dir, CopyOptions};
use notify::RecursiveMode;
use tauri::{AppHandle, Emitter, State, WebviewUrl, WebviewWindowBuilder};

use crate::service::app_state::AppState;
use crate::service::explorer_state::{
    add_explorer_state, add_explorer_tab_state, explore_path_with_count,
    get_active_tab_state_query, get_tab_index_by_key, get_tab_state_by_index,
    get_tab_state_query_by_key, remove_explorer_tab_state, reset_explorer_tab_state,
    update_tab_state, Thumbnail,
};
use crate::service::explorer_types::SortConfig;
use crate::service::types::ActiveTab;
use crate::utils::watcher_utils::{
    create_explorer_watcher_callback, subscribe_directory, unsubscribe_directory,
};

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
        Some(&state.db),
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
        Some(&state.db),
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

// ========================================
// イベント発行ヘルパー（app レイヤー専用）
// ========================================

use crate::service::explorer_state::ExplorerTabState;

/// タブ状態変更イベントを発行する
fn emit_tab_state(
    label: &str,
    tab_state: &ExplorerTabState,
    app: &AppHandle,
) -> Result<(), String> {
    app.emit_to(label, "explorer-tab-state-changed", tab_state)
        .map_err(|_| "failed to emit explorer state".to_string())
}

/// タブの状態を更新してイベントを発行する
async fn update_tab_and_emit(
    label: &str,
    index: usize,
    page: usize,
    thumbnails: Vec<Thumbnail>,
    total_pages: usize,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let tab_state = update_tab_state(label, index, page, thumbnails, total_pages, state).await?;
    emit_tab_state(label, &tab_state, app)
}

/// 現在のタブ状態をそのまま emit する（ローディング解除用）
async fn emit_current_tab_state(
    label: &str,
    index: usize,
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let tab_state = get_tab_state_by_index(label, index, state).await?;
    emit_tab_state(label, &tab_state, app)
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
        Some(&state.db),
    )
    .await?;
    let tab_state = update_tab_state(&label, index, page, thumbnails, total_pages, &state).await?;
    emit_tab_state(&label, &tab_state, &app)?;
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
        Some(&state.db),
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
        Some(&state.db),
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
        Some(&state.db),
    )
    .await?;
    let page = total_pages;
    let (thumbnails, _) = explore_path_with_count(
        &path,
        page,
        state.thumbnail_cache.clone(),
        &sort,
        search_query.as_deref(),
        Some(&state.db),
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
        Some(&state.db),
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
        Some(&state.db),
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
            Some(&state.db),
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
            Some(&state.db),
        )
        .await?;
        update_tab_and_emit(&label, index, 1, thumbnails, total_pages, &state, &app).await?;
    } else {
        // デバイス一覧の場合は状態のみ更新
        emit_current_tab_state(&label, index, &state, &app).await?;
    }
    Ok(())
}

/// リコメンドを再構築する（バックグラウンド処理）
/// 指定ディレクトリ配下のすべてのフォルダのサムネイルから埋め込みを生成する
/// force_rebuild=false の場合、フォルダの更新日時が変わっていないものはスキップする
#[tauri::command]
pub(crate) async fn rebuild_recommendations(
    directory_path: String,
    force_rebuild: Option<bool>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    use crate::service::embedding_service::{embedding_to_bytes, EMBEDDING_VERSION};
    use std::fs::read_dir;
    use std::time::UNIX_EPOCH;

    let force = force_rebuild.unwrap_or(false);

    let embedding_service = state
        .embedding_service
        .as_ref()
        .ok_or_else(|| "Embedding service not available".to_string())?;

    // 既に処理中の場合はエラー
    if embedding_service.is_processing().await {
        return Err("Recommendation rebuild already in progress".to_string());
    }

    // ディレクトリ配下のフォルダ一覧を取得（更新日時付き）
    let dirs = read_dir(&directory_path).map_err(|_| "failed to open directory")?;
    let folder_entries: Vec<(String, i64)> = dirs
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let path = e.path().to_str()?.to_string();
            let modified = e
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs() as i64;
            Some((path, modified))
        })
        .collect();

    if folder_entries.is_empty() {
        return Err("No folders found in directory".to_string());
    }

    // 差分チェック用にDB内の folder_modified_at を一括取得
    let folder_paths: Vec<String> = folder_entries.iter().map(|(p, _)| p.clone()).collect();
    let db_modified_map = state
        .db
        .get_folder_modified_at_batch(&folder_paths)
        .map_err(|e| e.to_string())?;

    // 処理開始
    embedding_service.set_processing(true).await;

    // 処理開始を通知
    let _ = app.emit("rebuild-recommendations-started", ());

    // バックグラウンドで処理
    let db = state.db.clone();
    let service = embedding_service.clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        let result = async {
            let total = folder_entries.len();
            eprintln!(
                "[rebuild_recommendations] Processing {} folders in {} (force={})",
                total, directory_path, force
            );

            let mut processed = 0;
            let mut skipped_unchanged = 0;
            let mut skipped_error = 0;

            for (folder_path, folder_modified) in folder_entries {
                // 差分チェック: DBに保存された更新日時と比較
                if !force {
                    if let Some(&db_modified) = db_modified_map.get(&folder_path) {
                        if db_modified >= folder_modified {
                            skipped_unchanged += 1;
                            continue;
                        }
                    }
                }

                // フォルダの最初の画像を取得
                let thumbnail_path = find_first_image_in_folder(std::path::Path::new(&folder_path));

                if thumbnail_path.is_empty() {
                    skipped_error += 1;
                    continue;
                }

                // 画像を読み込み
                let thumbnail_data = match tokio::fs::read(&thumbnail_path).await {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!(
                            "[rebuild_recommendations] Failed to read {}: {}",
                            thumbnail_path, e
                        );
                        skipped_error += 1;
                        continue;
                    }
                };

                // 画像埋め込みを生成
                let image_embedding = match service.generate_image_embedding(&thumbnail_data).await
                {
                    Ok(emb) => emb,
                    Err(e) => {
                        eprintln!(
                            "[rebuild_recommendations] Failed to generate embedding for {}: {}",
                            folder_path, e
                        );
                        skipped_error += 1;
                        continue;
                    }
                };

                // DB に保存（サムネイルとフォルダ更新日時も一緒に保存）
                if let Err(e) = db.upsert_folder_embedding(
                    &folder_path,
                    Some(&thumbnail_data),
                    &embedding_to_bytes(&image_embedding),
                    EMBEDDING_VERSION,
                    folder_modified,
                ) {
                    eprintln!(
                        "[rebuild_recommendations] Failed to save embedding for {}: {}",
                        folder_path, e
                    );
                    skipped_error += 1;
                    continue;
                }

                processed += 1;

                // 進捗を通知（10件ごと）
                if processed % 10 == 0 || (processed + skipped_unchanged) % 100 == 0 {
                    let _ = app_handle.emit(
                        "rebuild-recommendations-progress",
                        serde_json::json!({
                            "processed": processed,
                            "total": total,
                            "skipped_unchanged": skipped_unchanged,
                            "skipped_error": skipped_error
                        }),
                    );
                }
            }

            eprintln!(
                "[rebuild_recommendations] Completed: {} processed, {} unchanged, {} errors",
                processed, skipped_unchanged, skipped_error
            );
            Ok::<_, String>((processed, skipped_unchanged, skipped_error))
        }
        .await;

        // 処理完了
        service.set_processing(false).await;

        match result {
            Ok((count, skipped_unchanged, skipped_error)) => {
                let _ = app_handle.emit(
                    "rebuild-recommendations-completed",
                    serde_json::json!({
                        "processed": count,
                        "skipped_unchanged": skipped_unchanged,
                        "skipped_error": skipped_error
                    }),
                );
            }
            Err(e) => {
                let _ = app_handle.emit("rebuild-recommendations-error", e);
            }
        }
    });

    Ok(())
}

/// フォルダの最初の画像ファイルを見つける
fn find_first_image_in_folder(folder_path: &std::path::Path) -> String {
    use std::fs::read_dir;

    let extensions = [
        "jpg", "jpeg", "JPG", "JPEG", "jpe", "jfif", "pjpeg", "pjp", "png", "PNG", "gif", "tif",
        "tiff", "bmp", "dib", "webp",
    ];

    if let Ok(inner_file) = read_dir(folder_path) {
        for entry in inner_file.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if extensions.contains(&ext) {
                return path.to_str().unwrap_or_default().to_string();
            }
        }
    }
    String::new()
}

/// リコメンド再構築が処理中かどうかを取得
#[tauri::command]
pub(crate) async fn is_rebuilding_recommendations(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    match &state.embedding_service {
        Some(service) => Ok(service.is_processing().await),
        None => Ok(false),
    }
}

/// 指定フォルダ群のリコメンドスコアを取得
#[tauri::command]
pub(crate) async fn get_recommendation_scores(
    folder_paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, f64>, String> {
    use crate::service::embedding_service::{
        average_embeddings, cosine_similarity, embedding_from_bytes,
    };

    const IMAGE_WEIGHT: f64 = 0.8;
    const PATH_WEIGHT: f64 = 0.2;
    const RECENT_LIMIT: usize = 20;

    let mut scores = std::collections::HashMap::new();

    // 埋め込みサービスがない場合はすべて 0 スコア
    if state.embedding_service.is_none() {
        for path in &folder_paths {
            scores.insert(path.clone(), 0.0);
        }
        return Ok(scores);
    }

    // 直近閲覧したフォルダの埋め込みを取得
    let recent_records = state
        .db
        .get_recent_viewed_records_with_embeddings(RECENT_LIMIT)
        .map_err(|e| e.to_string())?;

    if recent_records.is_empty() {
        // 履歴がない場合はすべて 0 スコア
        for path in &folder_paths {
            scores.insert(path.clone(), 0.0);
        }
        return Ok(scores);
    }

    // 履歴の埋め込みを平均
    let recent_image_embeddings: Vec<Vec<f32>> = recent_records
        .iter()
        .filter_map(|r| r.image_embedding.as_ref().map(|e| embedding_from_bytes(e)))
        .collect();
    let recent_path_embeddings: Vec<Vec<f32>> = recent_records
        .iter()
        .filter_map(|r| r.path_embedding.as_ref().map(|e| embedding_from_bytes(e)))
        .collect();

    let avg_image_embedding = average_embeddings(&recent_image_embeddings);
    let avg_path_embedding = average_embeddings(&recent_path_embeddings);

    // 対象フォルダのレコードを取得
    let target_records = state
        .db
        .get_folder_records_by_paths(&folder_paths)
        .map_err(|e| e.to_string())?;

    // スコアを計算
    for record in target_records {
        let mut score = 0.0f64;

        if let Some(image_emb) = &record.image_embedding {
            let emb = embedding_from_bytes(image_emb);
            let sim = cosine_similarity(&emb, &avg_image_embedding);
            score += IMAGE_WEIGHT * sim as f64;
        }

        if let Some(path_emb) = &record.path_embedding {
            let emb = embedding_from_bytes(path_emb);
            let sim = cosine_similarity(&emb, &avg_path_embedding);
            score += PATH_WEIGHT * sim as f64;
        }

        scores.insert(record.path.clone(), score);
    }

    // 埋め込みがないフォルダは 0 スコア
    for path in &folder_paths {
        scores.entry(path.clone()).or_insert(0.0);
    }

    Ok(scores)
}
