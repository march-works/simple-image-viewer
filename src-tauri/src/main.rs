#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{io::Read, sync::mpsc::channel, time::Duration};

use notify::{watcher, RecursiveMode, Watcher};
use tauri::Window;

#[tauri::command]
fn subscribe_dir_notification(filepath: String, window: Window) {
    std::thread::spawn(move || {
        let (sender, receiver) = channel();
        let mut watcher = watcher(sender, Duration::from_secs(10)).unwrap();
        watcher.watch(&filepath, RecursiveMode::Recursive).unwrap();
        loop {
            match receiver.recv() {
                Ok(_) => {
                    window
                        .emit("directory-tree-changed", &filepath)
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
            }
        }
    });
}

#[tauri::command]
fn open_file_image(filepath: String) -> String {
    let img = std::fs::read(filepath).unwrap_or_default();
    base64::encode(&img)
}

#[tauri::command]
fn get_filenames_inner_zip(filepath: String) -> Vec<String> {
    let file = std::fs::read(filepath).unwrap_or_default();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(file));
    let mut files = zip
        .map(|f| f.file_names().map(|s| s.into()).collect::<Vec<String>>())
        .unwrap_or_default();
    files.sort();
    files
}

#[tauri::command]
fn read_image_in_zip(path: String, filename: String) -> String {
    let file = std::fs::read(path).unwrap_or_default();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(file));
    match zip {
        Ok(mut e) => {
            let inner = e.by_name(&filename);
            match inner {
                Ok(mut f) => {
                    let mut buf = vec![];
                    f.read_to_end(&mut buf).unwrap_or_default();
                    base64::encode(&buf)
                }
                Err(_) => "".into(),
            }
        }
        Err(_) => "".into(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_file_image,
            get_filenames_inner_zip,
            read_image_in_zip,
            subscribe_dir_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
