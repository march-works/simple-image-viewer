use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_dir;
use sysinfo::Disks;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub path: String,
    pub filename: String,
    pub thumbpath: String,
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

fn get_file_tree(path: &String, key_count: &mut i32) -> Vec<FileTree> {
    let dirs = std::fs::read_dir(path).unwrap();
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

pub(crate) fn explore_path(filepath: &str, page: usize) -> Result<Vec<Thumbnail>, String> {
    let dirs = read_dir(filepath).map_err(|_| "failed to open path")?;

    let extensions = vec![
        "jpg", "jpeg", "JPG", "JPEG", "jpe", "jfif", "pjpeg", "pjp", "png", "PNG", "gif", "tif",
        "tiff", "bmp", "dib", "webp",
    ];
    let files = dirs
        .skip((page - 1) * CATALOG_PER_PAGE)
        .take(CATALOG_PER_PAGE);
    let mut thumbs = vec![];
    for entry in files.flatten() {
        // TODO: zipの場合は飛ばさないようにする
        if entry.path().is_file() {
            continue;
        }
        let inner = read_dir(entry.path());
        if let Ok(mut inner_file) = inner {
            let mut thumbpath = "".to_string();
            for inn_v in inner_file.by_ref() {
                if let Ok(filepath) = inn_v {
                    let ext = filepath
                        .path()
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string();
                    if extensions.iter().any(|v| *v == ext) {
                        thumbpath = filepath.path().to_str().unwrap_or_default().to_string();
                        break;
                    }
                } else {
                    break;
                }
            }
            thumbs.push(Thumbnail {
                path: entry.path().to_str().unwrap().to_string(),
                filename: entry.file_name().to_str().unwrap().to_string(),
                thumbpath,
            });
        }
    }
    Ok(thumbs)
}

pub(crate) async fn get_page_count(filepath: &str) -> Result<usize, String> {
    let dirs = read_dir(filepath).map_err(|_| "failed to open inner path")?;
    Ok(dirs.count() / CATALOG_PER_PAGE + 1)
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
            }
        })
        .collect())
}
