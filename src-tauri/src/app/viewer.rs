use std::{io::Read, path::Path, sync::Mutex};
use base64::{engine::general_purpose, Engine as _};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use tauri::{State, Window};

#[tauri::command]
pub(crate) fn subscribe_dir_notification(filepath: String, window: Window) {
    let path_inner = filepath.clone();
    recommended_watcher(move |res| match res {
        Ok(_) => {
            window
                .emit("directory-tree-changed", &path_inner)
                .unwrap_or_default();
        }
        Err(_) => {
            window
                .emit(
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

pub struct ActiveWindow {
    pub label: Mutex<String>,
}

#[tauri::command]
pub(crate) fn change_active_window(window: Window, active: State<ActiveWindow>) {
    match active.label.lock() {
        Ok(mut label) => *label = window.label().to_string(),
        Err(_) => (),
    }
}
