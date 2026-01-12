//! Explorer関連の状態管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_dir;
use std::sync::Arc;
use sysinfo::Disks;
use tauri::State;
use tokio::sync::RwLock;

use super::types::{ActiveTab, AppState};
use crate::app::explorer_types::{SortConfig, SortField, SortOrder};

// ========================================
// 型定義
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Thumbnail {
    pub path: String,
    pub filename: String,
    pub thumbpath: String,
    pub modified_at: Option<u64>,
    pub created_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerTabState {
    pub title: String,
    pub key: String,
    pub path: Option<String>,
    pub transfer_path: Option<String>,
    pub page: usize,
    pub end: usize,
    pub folders: Vec<Thumbnail>,
    #[serde(default)]
    pub sort: SortConfig,
    #[serde(default)]
    pub search_query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerState {
    pub label: String,
    pub count: i32,
    pub active: Option<ActiveTab>,
    pub tabs: Vec<ExplorerTabState>,
}

// ========================================
// 状態管理関数
// ========================================

pub(crate) async fn add_explorer_state<'a>(state: &State<'a, AppState>) -> Result<String, String> {
    let mut explorers = state.explorers.lock().await;
    let label = format!("explorer-{}", *state.count.lock().await);
    (*explorers).push(ExplorerState {
        label: label.clone(),
        count: 0,
        active: None,
        tabs: vec![],
    });
    *state.count.lock().await += 1;
    Ok(label)
}

pub(crate) async fn remove_explorer_state<'a>(
    label: String,
    state: State<'a, AppState>,
) -> Result<(), String> {
    let mut explorers = state.explorers.lock().await;
    let index = (*explorers)
        .iter()
        .position(|w| w.label == label)
        .ok_or_else(|| "explorer not found".to_string())?;
    (*explorers).remove(index);
    Ok(())
}

pub(crate) async fn add_explorer_tab_state<'a>(
    label: &String,
    state: &State<'a, AppState>,
) -> Result<ExplorerState, String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "explorer not found".to_string())?;
    explorer_state.count += 1;
    let key = format!("tab-{}", explorer_state.count);
    let title = "Explorer".to_string();
    let tab = ExplorerTabState {
        title,
        key: key.clone(),
        path: None,
        transfer_path: None,
        page: 1,
        end: 1,
        folders: get_devices()?,
        sort: SortConfig::default(),
        search_query: None,
    };
    explorer_state.tabs.push(tab.clone());
    explorer_state.active = Some(ActiveTab { key: key.clone() });
    Ok(explorer_state.clone())
}

pub(crate) async fn reset_explorer_tab_state<'a>(
    label: &String,
    key: &String,
    state: &State<'a, AppState>,
) -> Result<ExplorerTabState, String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == *key)
        .ok_or_else(|| "tab not found".to_string())?;
    let tab = &mut explorer_state.tabs[index];
    tab.page = 1;
    tab.end = 1;
    tab.path = None;
    tab.folders = get_devices()?;
    tab.sort = SortConfig::default();
    tab.search_query = None;
    Ok(tab.clone())
}

pub(crate) async fn remove_explorer_tab_state(
    label: &String,
    key: &String,
    state: &State<'_, AppState>,
) -> Result<ExplorerState, String> {
    let mut explorers = state.explorers.lock().await;
    let explorer_state = (*explorers)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "explorer not found".to_string())?;
    let index = explorer_state
        .tabs
        .iter()
        .position(|t| t.key == *key)
        .ok_or_else(|| "tab not found".to_string())?;
    explorer_state.tabs.remove(index);
    if explorer_state.tabs.is_empty() {
        explorer_state.active = None;
    } else if explorer_state.active.is_some() && explorer_state.active.as_ref().unwrap().key == *key
    {
        let new_key = explorer_state.tabs[std::cmp::min(index, explorer_state.tabs.len() - 1)]
            .key
            .clone();
        explorer_state.active = Some(ActiveTab { key: new_key });
    }
    Ok(explorer_state.clone())
}

// ========================================
// ディレクトリ・サムネイル操作
// ========================================

const CATALOG_PER_PAGE: usize = 50;

/// 最初の画像ファイルを見つける (キャッシュなしの場合の処理)
fn find_first_image_in_folder(folder_path: &std::path::Path) -> String {
    let extensions = vec![
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

/// ディレクトリスキャン、ソート、ページネーション、サムネイル抽出を統合した最適化版
pub(crate) async fn explore_path_with_count(
    filepath: &str,
    page: usize,
    cache: Arc<RwLock<HashMap<String, String>>>,
    sort: &SortConfig,
    search_query: Option<&str>,
) -> Result<(Vec<Thumbnail>, usize), String> {
    use std::time::UNIX_EPOCH;

    // 1. 単一スキャンで全エントリを収集
    let dirs = read_dir(filepath).map_err(|_| "failed to open path")?;
    let mut entries: Vec<_> = dirs
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    // 2. 検索フィルタリング
    if let Some(query) = search_query {
        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            entries.retain(|e| {
                e.file_name()
                    .to_str()
                    .map(|name| name.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
            });
        }
    }

    // 3. メタデータ取得してソート
    let mut entries_with_meta: Vec<_> = entries
        .into_iter()
        .map(|e| {
            let metadata = e.metadata().ok();
            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let created = metadata
                .as_ref()
                .and_then(|m| m.created().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            (e, modified, created)
        })
        .collect();

    // ソート実行
    match (&sort.field, &sort.order) {
        (SortField::Name, SortOrder::Asc) => {
            entries_with_meta.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
        }
        (SortField::Name, SortOrder::Desc) => {
            entries_with_meta.sort_by(|a, b| b.0.file_name().cmp(&a.0.file_name()));
        }
        (SortField::DateModified, SortOrder::Asc) => {
            entries_with_meta.sort_by(|a, b| a.1.cmp(&b.1));
        }
        (SortField::DateModified, SortOrder::Desc) => {
            entries_with_meta.sort_by(|a, b| b.1.cmp(&a.1));
        }
        (SortField::DateCreated, SortOrder::Asc) => {
            entries_with_meta.sort_by(|a, b| a.2.cmp(&b.2));
        }
        (SortField::DateCreated, SortOrder::Desc) => {
            entries_with_meta.sort_by(|a, b| b.2.cmp(&a.2));
        }
    }

    // 4. 総ページ数を計算
    let total_count = entries_with_meta.len();
    let total_pages = if total_count == 0 {
        1
    } else {
        total_count.div_ceil(CATALOG_PER_PAGE)
    };

    // 5. ページネーション
    let start = (page.saturating_sub(1)) * CATALOG_PER_PAGE;
    let end = (start + CATALOG_PER_PAGE).min(total_count);

    if start >= total_count {
        return Ok((vec![], total_pages));
    }

    let page_entries = &entries_with_meta[start..end];

    // 6. サムネイル抽出 (並列処理)
    let tasks: Vec<_> = page_entries
        .iter()
        .map(|(entry, modified, created)| {
            let path = entry.path();
            let filename = entry.file_name().to_str().unwrap_or_default().to_string();
            let cache = cache.clone();
            let modified = *modified;
            let created = *created;

            tokio::spawn(async move {
                let path_str = path.to_str().unwrap_or_default().to_string();

                // キャッシュチェック
                {
                    let cache_read = cache.read().await;
                    if let Some(thumb) = cache_read.get(&path_str) {
                        return Thumbnail {
                            path: path_str,
                            filename,
                            thumbpath: thumb.clone(),
                            modified_at: modified,
                            created_at: created,
                        };
                    }
                }

                // キャッシュミス: ブロッキングI/Oで検索
                let path_clone = path.clone();
                let thumb =
                    tokio::task::spawn_blocking(move || find_first_image_in_folder(&path_clone))
                        .await
                        .unwrap_or_default();

                // キャッシュに保存
                {
                    let mut cache_write = cache.write().await;
                    cache_write.insert(path_str.clone(), thumb.clone());
                }

                Thumbnail {
                    path: path_str,
                    filename,
                    thumbpath: thumb,
                    modified_at: modified,
                    created_at: created,
                }
            })
        })
        .collect();

    // 全タスクの完了を待つ
    let mut thumbnails = Vec::new();
    for task in tasks {
        match task.await {
            Ok(thumb) => thumbnails.push(thumb),
            Err(_) => return Err("failed to extract thumbnails".to_string()),
        }
    }

    Ok((thumbnails, total_pages))
}

pub(crate) fn get_devices() -> Result<Vec<Thumbnail>, String> {
    let disks = Disks::new_with_refreshed_list();
    Ok(disks
        .iter()
        .map(|v| {
            let file = v.mount_point().to_str().unwrap_or_default().to_string();
            Thumbnail {
                path: file.clone(),
                filename: file,
                thumbpath: "".to_string(),
                modified_at: None,
                created_at: None,
            }
        })
        .collect())
}

/// 指定されたディレクトリのサムネイルキャッシュをクリア
pub(crate) async fn clear_thumbnail_cache_for_dir(
    dir_path: &str,
    cache: Arc<RwLock<HashMap<String, String>>>,
) {
    let mut cache_write = cache.write().await;
    cache_write.retain(|k, _| !k.starts_with(dir_path));
}
