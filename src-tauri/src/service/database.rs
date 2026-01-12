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
        // Phase 2: 基本テーブル
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

        // Phase 4: 埋め込みカラム追加
        // SQLite では ALTER TABLE ADD COLUMN IF NOT EXISTS がないので、
        // カラムが存在しない場合のみ追加する
        let has_image_embedding: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('folder_records') WHERE name='image_embedding'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0) > 0;

        if !has_image_embedding {
            conn.execute_batch(
                r#"
                ALTER TABLE folder_records ADD COLUMN image_embedding BLOB;
                ALTER TABLE folder_records ADD COLUMN path_embedding BLOB;
                ALTER TABLE folder_records ADD COLUMN embedding_version INTEGER DEFAULT 0;
                "#,
            )?;
        }

        // Phase 4.1: フォルダ更新日時カラム追加（差分更新用）
        let has_folder_modified_at: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('folder_records') WHERE name='folder_modified_at'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0) > 0;

        if !has_folder_modified_at {
            conn.execute_batch(
                r#"
                ALTER TABLE folder_records ADD COLUMN folder_modified_at INTEGER;
                "#,
            )?;
        }

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
                SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at,
                       image_embedding, path_embedding, embedding_version, folder_modified_at
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
                        image_embedding: row.get(6)?,
                        path_embedding: row.get(7)?,
                        embedding_version: row.get(8)?,
                        folder_modified_at: row.get(9)?,
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
            SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at,
                   image_embedding, path_embedding, embedding_version, folder_modified_at
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
                    image_embedding: row.get(6)?,
                    path_embedding: row.get(7)?,
                    embedding_version: row.get(8)?,
                    folder_modified_at: row.get(9)?,
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

    /// 埋め込みベクトルを更新する
    pub fn update_embeddings(
        &self,
        folder_path: &str,
        image_embedding: &[u8],
        path_embedding: &[u8],
        version: i32,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        conn.execute(
            r#"
            UPDATE folder_records
            SET image_embedding = ?2, path_embedding = ?3, embedding_version = ?4
            WHERE path = ?1
            "#,
            rusqlite::params![folder_path, image_embedding, path_embedding, version],
        )?;

        Ok(())
    }

    /// フォルダの埋め込みを挿入または更新する（リコメンド再構築用）
    /// レコードが存在しない場合は新規作成、存在する場合は埋め込みのみ更新
    pub fn upsert_folder_embedding(
        &self,
        folder_path: &str,
        thumbnail_blob: Option<&[u8]>,
        image_embedding: &[u8],
        version: i32,
        folder_modified_at: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO folder_records (path, thumbnail_blob, image_embedding, embedding_version, folder_modified_at, created_at, view_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
            ON CONFLICT(path) DO UPDATE SET
                thumbnail_blob = COALESCE(?2, thumbnail_blob),
                image_embedding = ?3,
                embedding_version = ?4,
                folder_modified_at = ?5
            "#,
            rusqlite::params![folder_path, thumbnail_blob, image_embedding, version, folder_modified_at, now],
        )?;

        Ok(())
    }

    /// 指定パスの folder_modified_at を取得（差分更新チェック用）
    pub fn get_folder_modified_at(&self, folder_path: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let result: Option<i64> = conn
            .query_row(
                "SELECT folder_modified_at FROM folder_records WHERE path = ?1",
                [folder_path],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        Ok(result)
    }

    /// 複数パスの folder_modified_at を一括取得（差分更新チェック用）
    pub fn get_folder_modified_at_batch(
        &self,
        paths: &[String],
    ) -> Result<std::collections::HashMap<String, i64>> {
        if paths.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let placeholders: Vec<String> = (1..=paths.len()).map(|i| format!("?{}", i)).collect();
        let query = format!(
            "SELECT path, folder_modified_at FROM folder_records WHERE path IN ({}) AND folder_modified_at IS NOT NULL",
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&query)?;

        let records: std::collections::HashMap<String, i64> = stmt
            .query_map(rusqlite::params_from_iter(paths.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    /// サムネイルがあるが埋め込みがない、または古いバージョンのレコードを取得
    pub fn get_records_needing_embedding(&self, current_version: i32) -> Result<Vec<FolderRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at,
                   image_embedding, path_embedding, embedding_version, folder_modified_at
            FROM folder_records
            WHERE thumbnail_blob IS NOT NULL
              AND (image_embedding IS NULL OR embedding_version < ?1)
            "#,
        )?;

        let records = stmt
            .query_map([current_version], |row| {
                Ok(FolderRecord {
                    path: row.get(0)?,
                    thumbnail_blob: row.get(1)?,
                    thumbnail_hash: row.get(2)?,
                    last_viewed_at: row.get(3)?,
                    view_count: row.get(4)?,
                    created_at: row.get(5)?,
                    image_embedding: row.get(6)?,
                    path_embedding: row.get(7)?,
                    embedding_version: row.get(8)?,
                    folder_modified_at: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    /// 直近閲覧した N 件のフォルダレコード（埋め込み付き）を取得
    pub fn get_recent_viewed_records_with_embeddings(
        &self,
        limit: usize,
    ) -> Result<Vec<FolderRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at,
                   image_embedding, path_embedding, embedding_version, folder_modified_at
            FROM folder_records
            WHERE last_viewed_at IS NOT NULL
              AND image_embedding IS NOT NULL
            ORDER BY last_viewed_at DESC
            LIMIT ?1
            "#,
        )?;

        let records = stmt
            .query_map([limit], |row| {
                Ok(FolderRecord {
                    path: row.get(0)?,
                    thumbnail_blob: row.get(1)?,
                    thumbnail_hash: row.get(2)?,
                    last_viewed_at: row.get(3)?,
                    view_count: row.get(4)?,
                    created_at: row.get(5)?,
                    image_embedding: row.get(6)?,
                    path_embedding: row.get(7)?,
                    embedding_version: row.get(8)?,
                    folder_modified_at: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    /// 指定パスのフォルダの埋め込みスコアを取得（リコメンド用）
    pub fn get_folder_records_by_paths(&self, paths: &[String]) -> Result<Vec<FolderRecord>> {
        if paths.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        // SQLite の IN 句用にプレースホルダを生成
        let placeholders: Vec<String> = (1..=paths.len()).map(|i| format!("?{}", i)).collect();
        let query = format!(
            r#"
            SELECT path, thumbnail_blob, thumbnail_hash, last_viewed_at, view_count, created_at,
                   image_embedding, path_embedding, embedding_version, folder_modified_at
            FROM folder_records
            WHERE path IN ({})
            "#,
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&query)?;

        let records = stmt
            .query_map(rusqlite::params_from_iter(paths.iter()), |row| {
                Ok(FolderRecord {
                    path: row.get(0)?,
                    thumbnail_blob: row.get(1)?,
                    thumbnail_hash: row.get(2)?,
                    last_viewed_at: row.get(3)?,
                    view_count: row.get(4)?,
                    created_at: row.get(5)?,
                    image_embedding: row.get(6)?,
                    path_embedding: row.get(7)?,
                    embedding_version: row.get(8)?,
                    folder_modified_at: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
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
    pub image_embedding: Option<Vec<u8>>,
    pub path_embedding: Option<Vec<u8>>,
    pub embedding_version: Option<i32>,
    pub folder_modified_at: Option<i64>,
}
