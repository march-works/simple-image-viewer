use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_dir;
use std::sync::Arc;
use sysinfo::Disks;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{Mutex, RwLock};

use crate::utils::file_utils::{
    get_any_extensions, get_filename_without_extension, get_parent_dir, get_parent_dir_name,
    is_compressed_file, is_executable_file, is_image_file, is_video_file,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveViewer {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTab {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    pub path: String,
    pub name: String,
    pub children: Vec<FileTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub key: String,
    pub file_type: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTree {
    Directory(Directory),
    File(File),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerTabState {
    pub title: String,
    pub key: String,
    pub path: String,
    pub viewing: Option<File>,
    pub tree: Vec<FileTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerState {
    pub label: String,
    pub count: i32,
    pub active: Option<ActiveTab>,
    pub tabs: Vec<ViewerTabState>,
}

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
    pub sort: crate::app::explorer_types::SortConfig,
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

pub struct AppState {
    pub count: Mutex<i32>,
    pub active: Mutex<ActiveViewer>,
    pub viewers: Mutex<Vec<ViewerState>>,
    pub explorers: Mutex<Vec<ExplorerState>>,
    /// ディレクトリ監視のwatcher管理 (path -> (watcher, 参照カウント))
    pub watchers: Mutex<HashMap<String, (RecommendedWatcher, usize)>>,
    /// サムネイルキャッシュ (folder_path -> thumbnail_path)
    pub thumbnail_cache: Arc<RwLock<HashMap<String, String>>>,
}

pub(crate) async fn add_viewer_state<'a>(state: &State<'a, AppState>) -> Result<String, String> {
    let mut viewers = state.viewers.lock().await;
    let label = format!("viewer-{}", *state.count.lock().await);
    (*viewers).push(ViewerState {
        label: label.clone(),
        count: 0,
        active: None,
        tabs: vec![],
    });
    *state.count.lock().await += 1;
    Ok(label)
}

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

pub(crate) async fn remove_viewer_state<'a>(
    label: String,
    state: State<'a, AppState>,
) -> Result<(), String> {
    let mut viewers = state.viewers.lock().await;
    let index = (*viewers)
        .iter()
        .position(|w| w.label == label)
        .ok_or_else(|| "viewer not found".to_string())?;
    (*viewers).remove(index);
    Ok(())
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

pub(crate) async fn add_viewer_tab_state<'a>(
    path: &String,
    label: &String,
    state: &State<'a, AppState>,
) -> Result<ViewerState, String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "viewer not found".to_string())?;
    viewer_state.count += 1;
    let key = format!("tab-{}", viewer_state.count);
    let title = if is_executable_file(path) {
        get_parent_dir_name(path)
    } else {
        get_filename_without_extension(path)
    };
    let new_path = if is_compressed_file(path) {
        path.clone()
    } else {
        get_parent_dir(path)
    };
    let tree = if is_compressed_file(path) {
        get_compressed_file_tree(&new_path)
    } else {
        let mut key_count = 0;
        get_file_tree(&new_path, &mut key_count)
    };
    let viewing = if is_compressed_file(path) {
        find_first_file(&tree)
    } else {
        find_path_in_tree(&tree, path)
    };
    let tab = ViewerTabState {
        title,
        key: key.clone(),
        path: new_path,
        viewing,
        tree,
    };
    viewer_state.tabs.push(tab.clone());
    viewer_state.active = Some(ActiveTab { key: key.clone() });
    Ok(viewer_state.clone())
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
        sort: crate::app::explorer_types::SortConfig::default(),
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
    tab.sort = crate::app::explorer_types::SortConfig::default();
    tab.search_query = None;
    Ok(tab.clone())
}

pub(crate) async fn remove_viewer_tab_state(
    label: &String,
    key: &String,
    state: &State<'_, AppState>,
) -> Result<ViewerState, String> {
    let mut viewers = state.viewers.lock().await;
    let viewer_state = (*viewers)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "viewer not found".to_string())?;
    let index = viewer_state
        .tabs
        .iter()
        .position(|t| t.key == *key)
        .ok_or_else(|| "tab not found".to_string())?;
    viewer_state.tabs.remove(index);
    if viewer_state.tabs.is_empty() {
        viewer_state.active = None;
    } else if viewer_state.active.is_some() && viewer_state.active.as_ref().unwrap().key == *key {
        let new_key = viewer_state.tabs[std::cmp::min(index, viewer_state.tabs.len() - 1)]
            .key
            .clone();
        viewer_state.active = Some(ActiveTab { key: new_key });
    }
    Ok(viewer_state.clone())
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

pub(crate) async fn open_file_pick_dialog(app: &tauri::AppHandle) -> Result<String, String> {
    use tokio::sync::oneshot;

    let extensions = get_any_extensions();
    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .add_filter(
            "File",
            &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    match rx.await {
        Ok(Some(path)) => Ok(path.to_string()),
        Ok(None) => Err("no file selected".to_string()),
        Err(_) => Err("dialog cancelled".to_string()),
    }
}

/// ディレクトリのファイルツリーを再構築する
/// ディレクトリ変更通知を受けた際にフロントエンドから呼び出される
pub(crate) fn rebuild_file_tree(path: &str, is_compressed: bool) -> Vec<FileTree> {
    if is_compressed {
        get_compressed_file_tree(&path.to_string())
    } else {
        let mut key_count = 0;
        get_file_tree(&path.to_string(), &mut key_count)
    }
}

fn get_file_tree(path: &String, key_count: &mut i32) -> Vec<FileTree> {
    let dirs = match std::fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let mut files = dirs
        .map(|f| {
            let filepath = f.unwrap().path();
            if filepath.is_dir() {
                FileTree::Directory(Directory {
                    path: filepath.to_str().unwrap_or_default().to_string(),
                    name: filepath.file_name().unwrap().to_str().unwrap().to_string(),
                    children: get_file_tree(
                        &filepath.to_str().unwrap_or_default().to_string(),
                        key_count,
                    ),
                })
            } else {
                *key_count += 1;
                let key = format!("file-{}", key_count);
                let filepath_str = filepath.to_str().unwrap_or_default();
                FileTree::File(File {
                    key,
                    file_type: if is_image_file(filepath_str) {
                        "Image".to_string()
                    } else if is_video_file(filepath_str) {
                        "Video".to_string()
                    } else {
                        "File".to_string()
                    },
                    path: filepath_str.to_string(),
                    name: filepath.file_name().unwrap().to_str().unwrap().to_string(),
                })
            }
        })
        .filter(|f| match f {
            FileTree::Directory(d) => !d.children.is_empty(),
            FileTree::File(file) => is_executable_file(&file.path),
        })
        .collect::<Vec<FileTree>>();
    files.sort_by(|a, b| match (a, b) {
        (FileTree::Directory(_), FileTree::File(_)) => std::cmp::Ordering::Less,
        (FileTree::File(_), FileTree::Directory(_)) => std::cmp::Ordering::Greater,
        (FileTree::Directory(a), FileTree::Directory(b)) => a.path.cmp(&b.path),
        (FileTree::File(a), FileTree::File(b)) => natord::compare(&a.name, &b.name),
    });
    files
}

/// ZIPファイル内のファイルツリーを取得（ストリーミング読み込み）\n
fn get_compressed_file_tree(filepath: &String) -> Vec<FileTree> {
    let mut key_count = 0;
    let file = match std::fs::File::open(filepath) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = std::io::BufReader::new(file);
    let zip = match zip::ZipArchive::new(reader) {
        Ok(z) => z,
        Err(_) => return vec![],
    };
    let mut files: Vec<String> = zip.file_names().map(|s| s.into()).collect();
    files.sort();
    files
        .iter()
        .map(|f| {
            key_count += 1;
            let key = format!("file-{}", key_count);
            FileTree::File(File {
                key,
                file_type: "Zip".to_string(),
                path: filepath.clone(),
                name: f.clone(),
            })
        })
        .collect()
}

pub(crate) fn find_first_file(tree: &Vec<FileTree>) -> Option<File> {
    for t in tree {
        match t {
            FileTree::Directory(d) => {
                if let Some(f) = find_first_file(&d.children) {
                    return Some(f);
                }
            }
            FileTree::File(f) => return Some(f.clone()),
        }
    }
    None
}

pub(crate) fn find_key_in_tree(tree: &Vec<FileTree>, key: &String) -> Option<File> {
    for file in tree {
        match file {
            FileTree::File(file) => {
                if file.key == *key {
                    return Some(file.clone());
                }
            }
            FileTree::Directory(Directory {
                path: _,
                name: _,
                children,
            }) => {
                let file = find_key_in_tree(children, key);
                if file.is_some() {
                    return file;
                }
            }
        }
    }
    None
}

pub(crate) fn find_path_in_tree(tree: &Vec<FileTree>, path: &String) -> Option<File> {
    for file in tree {
        match file {
            FileTree::File(file) => {
                if file.path == *path {
                    return Some(file.clone());
                }
            }
            FileTree::Directory(Directory {
                path: _,
                name: _,
                children,
            }) => {
                let file = find_key_in_tree(children, path);
                if file.is_some() {
                    return file;
                }
            }
        }
    }
    None
}

pub(crate) fn get_next_in_tree(viewing: &String, tree: &[FileTree]) -> Option<File> {
    let (files, dirs): (Vec<_>, Vec<_>) = tree.iter().partition(|v| matches!(v, FileTree::File(_)));
    let files: Vec<_> = files
        .iter()
        .map(|v| match v {
            FileTree::File(file) => file.clone(),
            _ => File {
                key: "".to_string(),
                file_type: "".to_string(),
                path: "".to_string(),
                name: "".to_string(),
            },
        })
        .collect();
    let dirs: Vec<_> = dirs
        .iter()
        .map(|v| match v {
            FileTree::Directory(dir) => dir.clone(),
            _ => Directory {
                path: "".to_string(),
                name: "".to_string(),
                children: vec![],
            },
        })
        .collect();
    let idx = files.iter().position(|v| v.key == *viewing);
    let length = files.len();
    if let Some(idx) = idx {
        let next_idx = (idx + 1) % length;
        return files.get(next_idx).cloned();
    }

    for dir in dirs {
        let file = get_next_in_tree(viewing, &dir.children);
        if file.is_some() {
            return file;
        }
    }
    None
}

pub(crate) fn get_prev_in_tree(viewing: &String, tree: &[FileTree]) -> Option<File> {
    let (files, dirs): (Vec<_>, Vec<_>) = tree.iter().partition(|v| matches!(v, FileTree::File(_)));
    let files: Vec<_> = files
        .iter()
        .map(|v| match v {
            FileTree::File(file) => file.clone(),
            _ => File {
                key: "".to_string(),
                file_type: "".to_string(),
                path: "".to_string(),
                name: "".to_string(),
            },
        })
        .collect();
    let dirs: Vec<_> = dirs
        .iter()
        .map(|v| match v {
            FileTree::Directory(dir) => dir.clone(),
            _ => Directory {
                path: "".to_string(),
                name: "".to_string(),
                children: vec![],
            },
        })
        .collect();
    let idx = files.iter().position(|v| v.key == *viewing);
    let length = files.len();
    if let Some(idx) = idx {
        let next_idx = (idx + length - 1) % length;
        return files.get(next_idx).cloned();
    }

    for dir in dirs {
        let file = get_prev_in_tree(viewing, &dir.children);
        if file.is_some() {
            return file;
        }
    }
    None
}

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
    sort: &crate::app::explorer_types::SortConfig,
    search_query: Option<&str>,
) -> Result<(Vec<Thumbnail>, usize), String> {
    use crate::app::explorer_types::{SortField, SortOrder};
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

// 旧関数は互換性のため残す (内部的には新関数を呼ぶ)
pub(crate) fn explore_path(filepath: &str, page: usize) -> Result<Vec<Thumbnail>, String> {
    use std::time::UNIX_EPOCH;

    // 同期版 - キャッシュなしで動作
    let dirs = read_dir(filepath).map_err(|_| "failed to open path")?;
    let mut entries: Vec<_> = dirs
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by_key(|a| a.file_name());

    let total_count = entries.len();
    let start = (page.saturating_sub(1)) * CATALOG_PER_PAGE;
    let end = (start + CATALOG_PER_PAGE).min(total_count);

    if start >= total_count {
        return Ok(vec![]);
    }

    let page_entries = &entries[start..end];

    let mut thumbs = vec![];
    for entry in page_entries {
        let thumbpath = find_first_image_in_folder(&entry.path());
        let metadata = entry.metadata().ok();
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
        thumbs.push(Thumbnail {
            path: entry.path().to_str().unwrap_or_default().to_string(),
            filename: entry.file_name().to_str().unwrap_or_default().to_string(),
            thumbpath,
            modified_at: modified,
            created_at: created,
        });
    }

    Ok(thumbs)
}

pub(crate) async fn get_page_count(filepath: &str) -> Result<usize, String> {
    let dirs = read_dir(filepath).map_err(|_| "failed to open inner path")?;
    let count = dirs
        .filter(|e| e.as_ref().map(|e| e.path().is_dir()).unwrap_or(false))
        .count();
    if count == 0 {
        return Ok(1);
    }
    Ok(count.div_ceil(CATALOG_PER_PAGE))
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
