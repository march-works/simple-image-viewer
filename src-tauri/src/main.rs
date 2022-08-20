#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::io::Read;

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
        .map(|f| {
            f.file_names()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
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
                    let size: usize = f.size().try_into().unwrap_or_default();
                    let mut buf = vec![0 as u8; size];
                    f.read_exact(&mut buf).unwrap_or_default();
                    base64::encode(&buf)
                }
                Err(_) => "".to_string(),
            }
        }
        Err(_) => "".to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_file_image,
            get_filenames_inner_zip,
            read_image_in_zip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
