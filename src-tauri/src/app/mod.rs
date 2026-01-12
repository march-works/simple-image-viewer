#[macro_use]
pub mod viewer;
#[macro_use]
pub mod explorer;

use tauri::{
    async_runtime::Mutex,
    menu::{MenuBuilder, MenuItemBuilder},
    Builder, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry,
};

use crate::{
    app::{
        explorer::{
            change_active_explorer_tab, change_explorer_page, change_explorer_path,
            change_explorer_search, change_explorer_sort, change_explorer_transfer_path,
            get_recommendation_scores, is_rebuilding_recommendations, move_explorer_backward,
            move_explorer_forward, move_explorer_to_end, move_explorer_to_start, open_new_explorer,
            open_new_explorer_tab, rebuild_recommendations, refresh_explorer_tab,
            remove_explorer_tab, request_restore_explorer_state,
            request_restore_explorer_tab_state, reset_explorer_tab,
            subscribe_explorer_dir_notification, transfer_folder,
            unsubscribe_explorer_dir_notification,
        },
        viewer::{
            change_active_viewer, change_active_viewer_tab, change_viewing,
            close_viewer_tabs_by_directory, get_active_viewer_directory, get_filenames_inner_zip,
            move_backward, move_forward, open_file_image, open_image_dialog, open_new_viewer,
            open_new_viewer_tab, read_image_in_zip, record_folder_view, refresh_viewer_tab_tree,
            remove_viewer_tab, request_restore_viewer_state, request_restore_viewer_tab_state,
            subscribe_dir_notification, unsubscribe_dir_notification,
        },
    },
    service::{
        app_state::{ActiveViewer, AppState},
        database::Database,
        embedding_service::EmbeddingService,
        explorer_state::{remove_explorer_state, ExplorerState},
        viewer_state::{add_viewer_state, add_viewer_tab_state, remove_viewer_state, ViewerState},
    },
};

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
                label: "viewer-0".to_string(),
            },
            viewers: vec![ViewerState {
                label: "viewer-0".to_string(),
                count: 0,
                active: None,
                tabs: vec![],
            }],
            explorers: vec![],
        }
    }
}

/// Returns the app directory name based on build mode
fn get_app_dir_name() -> &'static str {
    if cfg!(debug_assertions) {
        "simple-image-viewer-dev"
    } else {
        "simple-image-viewer"
    }
}

/// Returns the viewer window title based on build mode
fn get_viewer_title() -> &'static str {
    if cfg!(debug_assertions) {
        "Simple Image Viewer [DEV]"
    } else {
        "Simple Image Viewer"
    }
}

/// Returns the explorer window title based on build mode
fn get_explorer_title() -> &'static str {
    if cfg!(debug_assertions) {
        "Image Explorer [DEV]"
    } else {
        "Image Explorer"
    }
}

pub fn create_viewer() -> Builder<Wry> {
    let save_dir = dirs::data_dir().unwrap_or_default();
    let app_dir = save_dir.join(get_app_dir_name());
    let save_path = app_dir.join("state.json");
    let saved_state = if let Ok(data) = std::fs::read_to_string(save_path.clone()) {
        serde_json::from_str::<SavedState>(&data).unwrap_or_default()
    } else {
        SavedState::default()
    };

    // Initialize SQLite database (Phase 2)
    let db_path = app_dir.join("data.db");
    let db = Database::init(&db_path).expect("Failed to initialize database");

    // Initialize CLIP embedding service (Phase 4)
    // モデルファイルが存在する場合のみ初期化
    let embedding_service = {
        let resource_dir = app_dir.clone();
        // 開発時は src-tauri/resources から、本番時は bundled resources から読み込む
        let vision_path = resource_dir.join("resources").join("vision_model.onnx");
        let text_path = resource_dir.join("resources").join("text_model.onnx");

        if vision_path.exists() && text_path.exists() {
            match EmbeddingService::init(&resource_dir.join("resources")) {
                Ok(service) => {
                    println!("CLIP embedding service initialized");
                    Some(std::sync::Arc::new(service))
                }
                Err(e) => {
                    eprintln!("Failed to initialize embedding service: {}", e);
                    None
                }
            }
        } else {
            // 開発時: src-tauri/resources から読み込む
            let dev_resource_dir =
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
            if dev_resource_dir.join("vision_model.onnx").exists() {
                match EmbeddingService::init(&dev_resource_dir) {
                    Ok(service) => {
                        println!("CLIP embedding service initialized (dev mode)");
                        Some(std::sync::Arc::new(service))
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize embedding service: {}", e);
                        None
                    }
                }
            } else {
                println!("CLIP models not found, recommendation feature disabled");
                None
            }
        }
    };

    let app_state = AppState {
        count: Mutex::new(saved_state.count),
        active: Mutex::new(saved_state.active),
        viewers: Mutex::new(saved_state.viewers.clone()),
        explorers: Mutex::new(saved_state.explorers.clone()),
        watchers: Mutex::new(std::collections::HashMap::new()),
        thumbnail_cache: std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        )),
        db: std::sync::Arc::new(db),
        embedding_service,
    };
    let viewers_to_restore = saved_state.viewers.clone();
    let explorers_to_restore = saved_state.explorers.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Callback when another instance tries to start
            // args[0] = executable path, args[1..] = arguments
            let app = app.clone();

            tokio::spawn(async move {
                let state = app.state::<AppState>();

                // Check if filepath argument exists (args[1] if present)
                if args.len() > 1 {
                    let filepath = args[1].clone();
                    let label = state.active.lock().await.label.clone();
                    if let Ok(window_state) = add_viewer_tab_state(&filepath, &label, &state).await
                    {
                        let _ = app.emit_to(&label, "viewer-state-changed", &window_state);
                    }
                    // Focus the active window
                    if let Some(window) = app.get_webview_window(&label) {
                        let _ = window.set_focus();
                    }
                } else {
                    // No filepath argument - open new window
                    if let Ok(label) = add_viewer_state(&state).await {
                        let _ = WebviewWindowBuilder::new(
                            &app,
                            &label,
                            WebviewUrl::App("index.html".into()),
                        )
                        .title(get_viewer_title())
                        .maximized(true)
                        .build();
                    }
                }
            });
        }))
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // Setup menu
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).item(&quit_item).build()?;
            app.set_menu(menu)?;

            // Restore windows from saved state
            viewers_to_restore.iter().for_each(|v| {
                let app_handle = app.app_handle().clone();
                let label = v.label.clone();
                // Skip if window already exists (e.g., viewer-0 from config)
                if app_handle.get_webview_window(&label).is_some() {
                    return;
                }
                tokio::spawn(async move {
                    WebviewWindowBuilder::new(
                        &app_handle,
                        label.clone(),
                        WebviewUrl::App("index.html".into()),
                    )
                    .title(get_viewer_title())
                    .maximized(true)
                    .build()
                    .unwrap();
                });
            });
            explorers_to_restore.iter().for_each(|v| {
                let app_handle = app.app_handle().clone();
                let label = v.label.clone();
                // Skip if window already exists
                if app_handle.get_webview_window(&label).is_some() {
                    return;
                }
                tokio::spawn(async move {
                    WebviewWindowBuilder::new(
                        &app_handle,
                        label.clone(),
                        WebviewUrl::App("explorer.html".into()),
                    )
                    .title(get_explorer_title())
                    .build()
                    .unwrap();
                });
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                let app = app.clone();
                tokio::spawn(async move {
                    let state = app.state::<AppState>();
                    let mut active = state.active.lock().await.clone();
                    let mut viewers = state.viewers.lock().await.clone();
                    if !viewers.is_empty() {
                        active.label = "viewer-0".to_string();
                        viewers[0].label = "viewer-0".to_string();
                    }
                    let explorers = state.explorers.lock().await.clone();
                    let saved_state = SavedState {
                        count: *state.count.lock().await,
                        active,
                        viewers,
                        explorers,
                    };
                    let dir = dirs::data_dir().unwrap_or_default();
                    let app_dir = dir.join(get_app_dir_name());
                    let _ = std::fs::create_dir_all(&app_dir);
                    let path = app_dir.join("state.json");
                    let _ = std::fs::write(
                        path.clone(),
                        serde_json::to_string(&saved_state).unwrap_or_default(),
                    );
                    println!("state saved to {:?}", path);
                    std::process::exit(0);
                });
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let window = window.clone();
                tokio::spawn(async move {
                    let state = window.state::<AppState>();
                    let label = window.label().to_string();
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
            unsubscribe_dir_notification,
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
            change_explorer_sort,
            change_explorer_search,
            reset_explorer_tab,
            move_explorer_forward,
            move_explorer_backward,
            move_explorer_to_end,
            move_explorer_to_start,
            transfer_folder,
            subscribe_explorer_dir_notification,
            unsubscribe_explorer_dir_notification,
            refresh_explorer_tab,
            change_viewing,
            move_forward,
            move_backward,
            request_restore_viewer_tab_state,
            refresh_viewer_tab_tree,
            get_active_viewer_directory,
            close_viewer_tabs_by_directory,
            record_folder_view,
            // Phase 4: Recommendation commands
            rebuild_recommendations,
            is_rebuilding_recommendations,
            get_recommendation_scores,
        ])
}
