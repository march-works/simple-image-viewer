//! SQLite データベース管理
//!
//! Phase 2: リコメンド機能の基盤として、サムネイルデータと閲覧履歴を蓄積する

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// データベース接続のラッパー
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// データベースを初期化し、必要なテーブルを作成する
    pub fn init(db_path: &Path) -> Result<Self> {
        // 親ディレクトリを作成
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // スキーマのマイグレーション
        Self::migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// スキーマのマイグレーションを実行
    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS folder_records (
                path TEXT PRIMARY KEY,
                thumbnail_blob BLOB,
                thumbnail_hash TEXT,
                last_viewed_at INTEGER,
                view_count INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_last_viewed ON folder_records(last_viewed_at DESC);
            "#,
        )?;
        Ok(())
    }

    /// フォルダの閲覧を記録する
    pub fn record_folder_view(
        &self,
        folder_path: &str,
        thumbnail_blob: Option<&[u8]>,
        thumbnail_hash: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO folder_records (path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at)
            VALUES (?1, ?2, ?3, ?4, 1, ?4)
            ON CONFLICT(path) DO UPDATE SET
                thumbnail_blob = COALESCE(?2, thumbnail_blob),
                thumbnail_hash = COALESCE(?3, thumbnail_hash),
                last_viewed_at = ?4,
                view_count = view_count + 1
            "#,
            rusqlite::params![folder_path, thumbnail_blob, thumbnail_hash, now],
        )?;

        Ok(())
    }

    /// サムネイルを更新する（閲覧履歴は更新しない）
    pub fn update_thumbnail(
        &self,
        folder_path: &str,
        thumbnail_blob: &[u8],
        thumbnail_hash: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO folder_records (path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at)
            VALUES (?1, ?2, ?3, NULL, 0, ?4)
            ON CONFLICT(path) DO UPDATE SET
                thumbnail_blob = ?2,
                thumbnail_hash = ?3
            "#,
            rusqlite::params![folder_path, thumbnail_blob, thumbnail_hash, now],
        )?;

        Ok(())
    }

    /// 指定パスのサムネイルハッシュを取得する
    pub fn get_thumbnail_hash(&self, folder_path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let result: Option<String> = conn
            .query_row(
                "SELECT thumbnail_hash FROM folder_records WHERE path = ?1",
                [folder_path],
                |row| row.get(0),
            )
            .ok();

        Ok(result)
    }

    /// 直近閲覧した N 件のフォルダパスを取得する（ディレクトリ問わず）
    pub fn get_recent_viewed_folders(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT path FROM folder_records
            WHERE last_viewed_at IS NOT NULL
            ORDER BY last_viewed_at DESC
            LIMIT ?1
            "#,
        )?;

        let paths = stmt
            .query_map([limit], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(paths)
    }

    /// 指定パスのフォルダレコードを取得する
    pub fn get_folder_record(&self, folder_path: &str) -> Result<Option<FolderRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let result = conn
            .query_row(
                r#"
                SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at
                FROM folder_records WHERE path = ?1
                "#,
                [folder_path],
                |row| {
                    Ok(FolderRecord {
                        path: row.get(0)?,
                        thumbnail_blob: row.get(1)?,
                        thumbnail_hash: row.get(2)?,
                        last_viewed_at: row.get(3)?,
                        view_count: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .ok();

        Ok(result)
    }

    /// 指定ディレクトリ内のすべてのフォルダレコードを取得する
    #[allow(dead_code)]
    pub fn get_folder_records_in_directory(&self, directory: &str) -> Result<Vec<FolderRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        // ディレクトリ直下のフォルダのみを取得（サブディレクトリは除外）
        let pattern = if directory.ends_with(std::path::MAIN_SEPARATOR) {
            format!("{}%", directory)
        } else {
            format!("{}{}%", directory, std::path::MAIN_SEPARATOR)
        };

        let mut stmt = conn.prepare(
            r#"
            SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at
            FROM folder_records
            WHERE path LIKE ?1
            "#,
        )?;

        let records = stmt
            .query_map([pattern], |row| {
                Ok(FolderRecord {
                    path: row.get(0)?,
                    thumbnail_blob: row.get(1)?,
                    thumbnail_hash: row.get(2)?,
                    last_viewed_at: row.get(3)?,
                    view_count: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    /// フォルダレコードを削除する
    #[allow(dead_code)]
    pub fn delete_folder_record(&self, folder_path: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.execute("DELETE FROM folder_records WHERE path = ?1", [folder_path])?;
        Ok(())
    }
}

/// フォルダレコード
#[derive(Debug, Clone)]
pub struct FolderRecord {
    pub path: String,
    pub thumbnail_blob: Option<Vec<u8>>,
    pub thumbnail_hash: Option<String>,
    pub last_viewed_at: Option<i64>,
    pub view_count: i64,
    pub created_at: i64,
}
