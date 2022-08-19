#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::io::Read;

#[tauri::command]
fn open_file_image(filepath: String) -> String {
  let img = std::fs::read(filepath).unwrap();
  base64::encode(&img)
}

#[tauri::command]
fn get_filenames_inner_zip(filepath: String) -> Vec<String> {
  let file = std::fs::read(filepath).unwrap();
  let zip = zip::ZipArchive::new(std::io::Cursor::new(file)).unwrap();
  let mut files: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
  files.sort();
  files
}

#[tauri::command]
fn read_image_in_zip(path: String, filename: String) -> String {
  let file = std::fs::read(path).unwrap();
  let mut zip = zip::ZipArchive::new(std::io::Cursor::new(file)).unwrap();
  let mut inner = zip.by_name(&filename).unwrap();
  let size: usize = inner.size().try_into().unwrap();
  let mut buf = vec![0 as u8; size];
  inner.read_exact(&mut buf).unwrap();
  base64::encode(&buf)
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
