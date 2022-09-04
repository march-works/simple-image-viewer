use std::{io::Read, path::Path, sync::Mutex};

use notify::{recommended_watcher, RecursiveMode, Watcher};
use serde_json::Value;
use sysinfo::{ProcessExt, System, SystemExt};
use tauri::{utils::platform::current_exe, Builder, Manager, State, Window, Wry};

use crate::grpc::{add_tab, new_window, server};

#[tauri::command]
fn subscribe_dir_notification(filepath: String, window: Window) {
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

pub struct ActiveWindow {
    pub label: Mutex<String>,
}

#[tauri::command]
fn change_active_window(window: Window, active: State<ActiveWindow>) {
    match active.label.lock() {
        Ok(mut label) => *label = window.label().to_string(),
        Err(_) => (),
    }
}

fn get_running_count() -> i32 {
    let app_exe = current_exe()
        .unwrap_or_default()
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string();
    let mut cnt = 0;
    System::new_all()
        .processes()
        .into_iter()
        .for_each(|(_, process)| {
            if app_exe
                == process
                    .exe()
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    .to_string()
            {
                cnt += 1;
            }
        });
    cnt
}

pub fn open_new_viewer() -> Builder<Wry> {
    tauri::Builder::default()
        .setup(|app| {
            if get_running_count() > 1 {
                match app.get_cli_matches() {
                    Ok(matches) => match &matches.args.get("filepath").map(|v| v.value.clone()) {
                        Some(Value::String(val)) => {
                            tokio::spawn(add_tab::transfer(val.to_string(), app.app_handle()));
                        }
                        _ => {
                            tokio::spawn(new_window::open(app.app_handle()));
                        }
                    },
                    Err(_) => {
                        app.app_handle().exit(0);
                    }
                }
            } else {
                tokio::spawn(server::run_server(app.app_handle()));
            }
            Ok(())
        })
        .manage(ActiveWindow {
            label: Mutex::new("main".to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            open_file_image,
            get_filenames_inner_zip,
            read_image_in_zip,
            subscribe_dir_notification,
            change_active_window,
        ])
}
