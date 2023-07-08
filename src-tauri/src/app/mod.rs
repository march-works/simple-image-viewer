#[macro_use]
pub mod viewer;
#[macro_use]
pub mod explorer;

use std::sync::Mutex;

use serde_json::Value;
use sysinfo::{ProcessExt, System, SystemExt};
use tauri::{utils::platform::current_exe, Builder, Manager, Wry};

use crate::{
    app::explorer::add_tab,
    // app::explorer::open_explorer,
    app::explorer::explore_path,
    app::explorer::get_page_count,
    app::explorer::show_devices,
    app::viewer::change_active_window,
    app::viewer::get_filenames_inner_zip,
    app::viewer::open_file_image,
    app::viewer::read_image_in_zip,
    app::viewer::subscribe_dir_notification,
    grpc::{add_tab, new_window, server},
};

use self::viewer::ActiveWindow;

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
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
            {
                cnt += 1;
            }
        });
    cnt
}

pub fn open_new_viewer() -> Builder<Wry> {
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
            explore_path,
            show_devices,
            get_page_count,
            add_tab,
        ])
}
