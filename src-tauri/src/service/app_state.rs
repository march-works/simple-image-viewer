use serde::{Deserialize, Serialize};
use tauri::{api::dialog::blocking::FileDialogBuilder, State};
use tokio::sync::Mutex;

use crate::utils::file_utils::{
    get_any_extensions, get_filename_without_extension, get_parent_dir, get_parent_dir_name,
    is_compressed_file, is_executable_file, is_image_file, is_video_file,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWindow {
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
pub struct TabState {
    pub title: String,
    pub key: String,
    pub path: String,
    pub viewing: Option<File>,
    pub tree: Vec<FileTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub label: String,
    pub count: i32,
    pub active: Option<ActiveTab>,
    pub tabs: Vec<TabState>,
}

pub struct AppState {
    pub count: Mutex<i32>,
    pub active: Mutex<ActiveWindow>,
    pub windows: Mutex<Vec<WindowState>>,
}

pub(crate) async fn add_window_state<'a>(state: &State<'a, AppState>) -> Result<String, String> {
    let mut windows = state.windows.lock().await;
    let label = format!("label-{}", *state.count.lock().await);
    (*windows).push(WindowState {
        label: label.clone(),
        count: 0,
        active: None,
        tabs: vec![],
    });
    *state.count.lock().await += 1;
    Ok(label)
}

pub(crate) async fn remove_window_state<'a>(
    label: String,
    state: State<'a, AppState>,
) -> Result<(), String> {
    let mut windows = state.windows.lock().await;
    let index = (*windows)
        .iter()
        .position(|w| w.label == label)
        .ok_or_else(|| "window not found".to_string())?;
    (*windows).remove(index);
    Ok(())
}

pub(crate) async fn add_tab_state<'a>(
    path: &String,
    label: &String,
    state: &State<'a, AppState>,
) -> Result<WindowState, String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "window not found".to_string())?;
    window_state.count += 1;
    let key = format!("tab-{}", window_state.count);
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
    let tab = TabState {
        title,
        key: key.clone(),
        path: new_path,
        viewing,
        tree,
    };
    window_state.tabs.push(tab.clone());
    window_state.active = Some(ActiveTab { key: key.clone() });
    Ok(window_state.clone())
}

pub(crate) async fn remove_tab_state(
    label: &String,
    key: &String,
    state: &State<'_, AppState>,
) -> Result<WindowState, String> {
    let mut windows = state.windows.lock().await;
    let window_state = (*windows)
        .iter_mut()
        .find(|w| w.label == *label)
        .ok_or_else(|| "window not found".to_string())?;
    let index = window_state
        .tabs
        .iter()
        .position(|t| t.key == *key)
        .ok_or_else(|| "tab not found".to_string())?;
    window_state.tabs.remove(index);
    if window_state.tabs.is_empty() {
        window_state.active = None;
    } else if window_state.active.is_some() && window_state.active.as_ref().unwrap().key == *key {
        let new_key = window_state.tabs[std::cmp::min(index, window_state.tabs.len() - 1)]
            .key
            .clone();
        window_state.active = Some(ActiveTab { key: new_key });
    }
    Ok(window_state.clone())
}

pub(crate) fn open_file_pick_dialog() -> Result<String, String> {
    let extensions = get_any_extensions();
    let extensions: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
    let filepath = FileDialogBuilder::new()
        .add_filter("File", &extensions)
        .pick_file();
    return match filepath {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("no file selected".to_string()),
    };
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

fn get_compressed_file_tree(filepath: &String) -> Vec<FileTree> {
    let mut key_count = 0;
    let file = std::fs::read(filepath).unwrap_or_default();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(file));
    let mut files = zip
        .map(|f| f.file_names().map(|s| s.into()).collect::<Vec<String>>())
        .unwrap_or_default();
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

pub(crate) fn get_next_in_tree(viewing: &String, tree: &Vec<FileTree>) -> Option<File> {
    let (files, dirs): (Vec<_>, Vec<_>) = tree.iter().partition(|v| match v {
        FileTree::File(_) => true,
        _ => false,
    });
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
    if idx.is_some() {
        let idx = idx.unwrap();
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

pub(crate) fn get_prev_in_tree(viewing: &String, tree: &Vec<FileTree>) -> Option<File> {
    let (files, dirs): (Vec<_>, Vec<_>) = tree.iter().partition(|v| match v {
        FileTree::File(_) => true,
        _ => false,
    });
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
    if idx.is_some() {
        let idx = idx.unwrap();
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
