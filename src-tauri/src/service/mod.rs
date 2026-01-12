pub mod app_state;
pub mod explorer_state;
pub mod types;
pub mod viewer_state;

// 型の再エクスポート（後方互換性）
pub use types::{ActiveTab, ActiveViewer, AppState};
