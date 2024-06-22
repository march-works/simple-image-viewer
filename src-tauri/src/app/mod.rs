#[macro_use]
pub mod viewer;
#[macro_use]
pub mod explorer;

use serde_json::Value;
use sysinfo::System;
use tauri::{api::path, async_runtime::Mutex, utils::platform::current_exe, Builder, Manager, Wry};

use crate::{
    // app::explorer::open_explorer,
    app::{
        explorer::{
            change_active_explorer_tab, change_explorer_page, change_explorer_path,
            change_explorer_transfer_path, move_explorer_backward, move_explorer_forward,
            move_explorer_to_end, move_explorer_to_start, open_new_explorer, open_new_explorer_tab,
            remove_explorer_tab, request_restore_explorer_state,
            request_restore_explorer_tab_state, reset_explorer_tab, transfer_folder,
        },
        viewer::{
            change_active_viewer, change_active_viewer_tab, change_viewing,
            get_filenames_inner_zip, move_backward, move_forward, open_file_image,
            open_image_dialog, open_new_viewer, open_new_viewer_tab, read_image_in_zip,
            remove_viewer_tab, request_restore_viewer_state, request_restore_viewer_tab_state,
            subscribe_dir_notification,
        },
    },
    grpc::{add_tab, new_window, server},
    service::app_state::{
        remove_explorer_state, remove_viewer_state, ActiveViewer, AppState, ExplorerState,
        ViewerState,
    },
};

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
        .iter()
        .for_each(|(_, process)| {
            if app_exe
                == *process
                    .exe()
                    .map(|v| v.file_name().unwrap_or_default().to_str())
                    .unwrap_or_default()
                    .unwrap_or_default()
            {
                cnt += 1;
            }
        });
    cnt
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    count: i32,
    active: ActiveViewer,
    viewers: Vec<ViewerState>,
    explorers: Vec<ExplorerState>,
}

impl Default for SavedState {
    fn default() -> Self {
        SavedState {
            count: 1,
            active: ActiveViewer {
                label: "label-0".to_string(),
            },
            viewers: vec![ViewerState {
                label: "label-0".to_string(),
                count: 0,
                active: None,
                tabs: vec![],
            }],
            explorers: vec![],
        }
    }
}

pub fn create_viewer() -> Builder<Wry> {
    let save_dir = path::app_data_dir(&tauri::Config::default()).unwrap();
    let save_path = save_dir.join("state.json");
    let saved_state = if let Ok(data) = std::fs::read_to_string(save_path.clone()) {
        serde_json::from_str::<SavedState>(&data).unwrap_or_default()
    } else {
        SavedState::default()
    };
    let app_state = AppState {
        count: Mutex::new(saved_state.count),
        active: Mutex::new(saved_state.active),
        viewers: Mutex::new(saved_state.viewers.clone()),
        explorers: Mutex::new(saved_state.explorers.clone()),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // when other process already running
            if get_running_count() > 1 {
                match app.get_cli_matches() {
                    Ok(matches) => match &matches.args.get("filepath").map(|v| v.value.clone()) {
                        Some(Value::String(val)) => {
                            // when executed with file path
                            tokio::spawn(add_tab::transfer(val.to_string(), app.app_handle()));
                        }
                        _ => {
                            // when executed without file path
                            tokio::spawn(new_window::open(app.app_handle()));
                        }
                    },
                    Err(_) => {
                        app.app_handle().exit(0);
                    }
                }
            } else {
                tokio::spawn(server::run_server(app.app_handle()));
                saved_state.viewers.into_iter().for_each(|v| {
                    let app_handle = app.app_handle();
                    tokio::spawn(async move {
                        let label = v.label.clone();
                        tauri::WindowBuilder::new(
                            &app_handle,
                            label.clone(),
                            tauri::WindowUrl::App("index.html".into()),
                        )
                        .title("Simple Image Viewer")
                        .maximized(true)
                        .build()
                        .unwrap();
                    });
                });
                saved_state.explorers.into_iter().for_each(|v| {
                    let app_handle = app.app_handle();
                    tokio::spawn(async move {
                        let label = v.label.clone();
                        tauri::WindowBuilder::new(
                            &app_handle,
                            label.clone(),
                            tauri::WindowUrl::App("explorer.html".into()),
                        )
                        .title("Image Explorer")
                        .build()
                        .unwrap();
                    });
                });
            }
            Ok(())
        })
        .menu(tauri::Menu::new().add_item(tauri::CustomMenuItem::new("quit", "Quit")))
        .on_menu_event(|event| {
            if event.menu_item_id() == "quit" {
                tokio::spawn(async move {
                    let state = event.window().state::<AppState>();
                    let mut active = state.active.lock().await.clone();
                    let mut viewers = state.viewers.lock().await.clone();
                    if !viewers.is_empty() {
                        active.label = "label-0".to_string();
                        viewers[0].label = "label-0".to_string();
                    }
                    let explorers = state.explorers.lock().await.clone();
                    let saved_state = SavedState {
                        count: *state.count.lock().await,
                        active,
                        viewers,
                        explorers,
                    };
                    let dir = path::app_data_dir(&tauri::Config::default()).unwrap();
                    let path = dir.join("state.json");
                    let _ = std::fs::write(
                        path.clone(),
                        serde_json::to_string(&saved_state).unwrap_or_default(),
                    );
                    println!("state saved to {:?}", path);
                    std::process::exit(0);
                });
            }
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
                tokio::spawn(async move {
                    let state = event.window().state::<AppState>();
                    let label = event.window().label().to_string();
                    if label.starts_with("explorer") {
                        remove_explorer_state(label, state).await
                    } else {
                        remove_viewer_state(label, state).await
                    }
                });
            }
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            open_file_image,
            get_filenames_inner_zip,
            read_image_in_zip,
            subscribe_dir_notification,
            open_new_viewer,
            open_new_viewer_tab,
            open_image_dialog,
            remove_viewer_tab,
            change_active_viewer_tab,
            request_restore_viewer_state,
            change_active_viewer,
            change_active_explorer_tab,
            request_restore_explorer_state,
            request_restore_explorer_tab_state,
            open_new_explorer,
            open_new_explorer_tab,
            remove_explorer_tab,
            change_explorer_page,
            change_explorer_transfer_path,
            change_explorer_path,
            reset_explorer_tab,
            move_explorer_forward,
            move_explorer_backward,
            move_explorer_to_end,
            move_explorer_to_start,
            transfer_folder,
            change_viewing,
            move_forward,
            move_backward,
            request_restore_viewer_tab_state,
        ])
}
