use serde::{Deserialize, Serialize};

/// ソートフィールド
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SortField {
    Name,
    #[default]
    DateModified,
    DateCreated,
}

/// ソート順序
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// ソート設定
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SortConfig {
    pub field: SortField,
    pub order: SortOrder,
}

/// Explorer のクエリパラメータ
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExplorerQuery {
    pub sort: SortConfig,
    pub search: Option<String>,
}
