use std::fs::read_dir;

use base64::engine::{general_purpose, Engine as _};
use serde::Serialize;

use sysinfo::{System, SystemExt, DiskExt};
use tauri::{AppHandle, Manager};
#[allow(unused_imports)]
use tokio_stream::StreamExt;

use super::viewer::ActiveWindow;

#[derive(Clone, Serialize)]
pub(crate) struct Thumbnail {
    path: String,
    filename: String,
    thumbnail: String,
    thumbpath: String,
}

pub struct StreamActivation(bool);

const CATALOG_PER_PAGE: usize = 50;

#[tauri::command]
pub(crate) fn explore_path(filepath: String, page: usize) -> Result<Vec<Thumbnail>, String> {
    let dirs = read_dir(filepath).map_err(|_| "failed to open path")?;

    let extensions = vec![
        "jpg",
        "jpeg",
        "JPG",
        "JPEG",
        "jpe",
        "jfif",
        "pjpeg",
        "pjp",
        "png",
        "PNG",
        "gif",
        "tif",
        "tiff",
        "bmp",
        "dib",
        "webp",
    ];
    let files = dirs.skip((page - 1) * CATALOG_PER_PAGE).take(CATALOG_PER_PAGE);
    let mut thumbs = vec![];
    for v in files {
        if let Ok(entry) = v {
            // TODO: zipの場合は飛ばさないようにする
            if entry.path().is_file() {
                continue;
            }
            let inner = read_dir(entry.path());
            if let Ok(mut inner_file) = inner {
                let mut thumb = "".to_string();
                let mut thumbpath = "".to_string();
                for inn_v in inner_file.by_ref() {
                    if let Ok(filepath) = inn_v {
                        let ext = filepath.path().extension().unwrap_or_default().to_str().unwrap_or_default().to_string();
                        if extensions.iter().any(|v| *v == ext) {
                            let img = std::fs::read(filepath.path()).unwrap_or_default();
                            thumb = general_purpose::STANDARD_NO_PAD.encode(img);
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
                    thumbnail: thumb,
                    thumbpath,
                });
            } else {
                println!("failed to open inner path: {:?}", inner);
            }
        }
    }
    Ok(thumbs)
}

#[tauri::command]
pub(crate) fn show_devices() -> Result<Vec<Thumbnail>, String> {
    let mut system = System::new();
    system.refresh_disks_list();
    Ok(system.disks().iter().map(|v| {
        let file = v.mount_point().to_str().unwrap_or_default().to_string();
        Thumbnail {
            path: file.clone(),
            filename: file,
            thumbnail: "".to_string(),
            thumbpath: "".to_string(),
        }
    }).collect())
}

#[tauri::command]
pub(crate) async fn get_page_count(filepath: String) -> Result<usize, String> {
    let dirs = read_dir(filepath).map_err(|_| "failed to open inner path")?;
    Ok(dirs.count() / CATALOG_PER_PAGE)
}

#[tauri::command]
pub(crate) async fn add_tab(filepath: String, app: AppHandle) -> Result<(), String> {
    let active = app.state::<ActiveWindow>();
    active.label.lock().map_or_else(
        |_| {
            app
                .emit_all("image-file-opened", filepath.clone())
                .unwrap_or(())
        },
        |label| {
            app
                .emit_to(
                    label.as_str(),
                    "image-file-opened",
                    filepath.clone(),
                )
                .unwrap_or_else(|_| {
                    app
                        .emit_all("image-file-opened", filepath.clone())
                        .unwrap_or(())
                })
        },
    );
    Ok(())
}
