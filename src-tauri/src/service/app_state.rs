use serde::{Deserialize, Serialize};
use tauri::{api::dialog::blocking::FileDialogBuilder, State};
use tokio::sync::Mutex;

use crate::utils::file_utils::{
    get_any_extensions, get_filename_without_extension, get_parent_dir, get_parent_dir_name,
    is_compressed_file, is_executable_file,
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
pub struct TabState {
    pub title: String,
    pub key: String,
    pub path: String,
    pub init_path: Option<String>,
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
    let init_path = if is_executable_file(path) {
        Some(path.clone())
    } else {
        None
    };
    let tab = TabState {
        title,
        key: key.clone(),
        path: new_path,
        init_path,
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
