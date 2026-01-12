//! 状態管理の共通型定義

use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::database::Database;
use super::embedding_service::EmbeddingService;
use super::explorer_state::ExplorerState;
use super::viewer_state::ViewerState;

// ========================================
// 共通型定義
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveViewer {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTab {
    pub key: String,
}

// ========================================
// アプリケーション状態
// ========================================

pub struct AppState {
    pub count: Mutex<i32>,
    pub active: Mutex<ActiveViewer>,
    pub viewers: Mutex<Vec<ViewerState>>,
    pub explorers: Mutex<Vec<ExplorerState>>,
    /// ディレクトリ監視のwatcher管理 (path -> (watcher, 参照カウント))
    pub watchers: Mutex<HashMap<String, (RecommendedWatcher, usize)>>,
    /// サムネイルキャッシュ (folder_path -> thumbnail_path)
    pub thumbnail_cache: Arc<RwLock<HashMap<String, String>>>,
    /// SQLite データベース (Phase 2: リコメンド基盤)
    pub db: Arc<Database>,
    /// CLIP 埋め込みサービス (Phase 4: ML リコメンド)
    pub embedding_service: Option<Arc<EmbeddingService>>,
}
