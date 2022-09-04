#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[tokio::main]
async fn main() {
    app::app::viewer::open_new_viewer()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
