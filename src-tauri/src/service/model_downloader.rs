//! ONNX モデルの遅延ダウンロード機能
//!
//! GitHub Releases からモデルファイルをオンデマンドでダウンロードし、
//! ローカルにキャッシュする機能を提供します。

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

/// モデルファイルのメタデータ
#[derive(Clone)]
pub struct ModelInfo {
    /// ファイル名
    pub filename: &'static str,
    /// ダウンロード URL
    pub url: &'static str,
    /// SHA256 ハッシュ（オプション、検証用）
    pub sha256: Option<&'static str>,
    /// 予想ファイルサイズ（プログレス表示用）
    pub expected_size: u64,
}

/// ダウンロード進捗イベント
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub status: DownloadStatus,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Verifying,
    Completed,
    Failed,
}

/// Vision モデル情報
pub const VISION_MODEL: ModelInfo = ModelInfo {
    filename: "vision_model.onnx",
    url: "https://github.com/march-works/simple-image-viewer/releases/download/models-v1/vision_model.onnx",
    sha256: None, // 初回リリース後に追加可能
    expected_size: 350_000_000, // 約350MB
};

/// Text モデル情報
pub const TEXT_MODEL: ModelInfo = ModelInfo {
    filename: "text_model.onnx",
    url: "https://github.com/march-works/simple-image-viewer/releases/download/models-v1/text_model.onnx",
    sha256: None, // 初回リリース後に追加可能
    expected_size: 254_000_000, // 約254MB
};

/// モデルダウンローダー
pub struct ModelDownloader {
    client: Client,
    cache_dir: PathBuf,
}

impl ModelDownloader {
    /// 新しいダウンローダーを作成
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let cache_dir = get_models_cache_dir(app_handle)?;

        // キャッシュディレクトリを作成
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache dir: {:?}", cache_dir))?;

        Ok(Self {
            client: Client::new(),
            cache_dir,
        })
    }

    /// Vision モデルのパスを取得（必要に応じてダウンロード）
    pub async fn get_vision_model_path(&self, app_handle: &AppHandle) -> Result<PathBuf> {
        self.ensure_model(&VISION_MODEL, app_handle).await
    }

    /// Text モデルのパスを取得（必要に応じてダウンロード）
    pub async fn get_text_model_path(&self, app_handle: &AppHandle) -> Result<PathBuf> {
        self.ensure_model(&TEXT_MODEL, app_handle).await
    }

    /// モデルが存在することを確認し、なければダウンロード
    async fn ensure_model(&self, model: &ModelInfo, app_handle: &AppHandle) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(model.filename);

        // 既にダウンロード済みで有効なファイルがあれば、そのパスを返す
        if self.is_valid_model(&model_path, model) {
            eprintln!("[model_downloader] Model already cached: {:?}", model_path);
            return Ok(model_path);
        }

        // ダウンロードが必要
        eprintln!(
            "[model_downloader] Downloading model: {} from {}",
            model.filename, model.url
        );
        self.download_model(model, &model_path, app_handle).await?;

        Ok(model_path)
    }

    /// モデルファイルが有効かチェック
    fn is_valid_model(&self, path: &PathBuf, model: &ModelInfo) -> bool {
        if !path.exists() {
            return false;
        }

        // ファイルサイズが最低1MB以上あること（LFSポインタファイル対策）
        match std::fs::metadata(path) {
            Ok(meta) => {
                let size = meta.len();
                if size < 1_000_000 {
                    eprintln!(
                        "[model_downloader] Model file too small ({}), may be corrupted: {:?}",
                        size, path
                    );
                    return false;
                }

                // SHA256 検証（設定されている場合）
                if let Some(expected_hash) = model.sha256 {
                    match self.verify_sha256(path, expected_hash) {
                        Ok(valid) => {
                            if !valid {
                                eprintln!("[model_downloader] SHA256 mismatch for: {:?}", path);
                                return false;
                            }
                        }
                        Err(e) => {
                            eprintln!("[model_downloader] Failed to verify SHA256: {}", e);
                            return false;
                        }
                    }
                }

                true
            }
            Err(_) => false,
        }
    }

    /// SHA256 ハッシュを検証
    fn verify_sha256(&self, path: &PathBuf, expected: &str) -> Result<bool> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let result = hasher.finalize();
        let actual = format!("{:x}", result);
        Ok(actual == expected)
    }

    /// モデルをダウンロード
    async fn download_model(
        &self,
        model: &ModelInfo,
        dest_path: &PathBuf,
        app_handle: &AppHandle,
    ) -> Result<()> {
        // 進捗通知: 開始
        self.emit_progress(
            app_handle,
            model.filename,
            0,
            model.expected_size,
            DownloadStatus::Starting,
        );

        // ダウンロードリクエスト
        let response = self
            .client
            .get(model.url)
            .send()
            .await
            .with_context(|| format!("Failed to request: {}", model.url))?
            .error_for_status()
            .with_context(|| format!("HTTP error for: {}", model.url))?;

        let total_size = response.content_length().unwrap_or(model.expected_size);

        // 一時ファイルに書き込み
        let temp_path = dest_path.with_extension("onnx.tmp");
        let mut file = tokio::fs::File::create(&temp_path)
            .await
            .with_context(|| format!("Failed to create temp file: {:?}", temp_path))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        // ストリーミングダウンロード
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.with_context(|| "Failed to read chunk")?;
            file.write_all(&chunk)
                .await
                .with_context(|| "Failed to write chunk")?;

            downloaded += chunk.len() as u64;

            // 進捗通知（1%ごと）
            let _percentage = (downloaded as f64 / total_size as f64) * 100.0;
            if downloaded == chunk.len() as u64
                || (downloaded % (total_size / 100 + 1)) < chunk.len() as u64
            {
                self.emit_progress(
                    app_handle,
                    model.filename,
                    downloaded,
                    total_size,
                    DownloadStatus::Downloading,
                );
            }
        }

        file.flush().await?;
        drop(file);

        // 検証
        self.emit_progress(
            app_handle,
            model.filename,
            downloaded,
            total_size,
            DownloadStatus::Verifying,
        );

        // ファイルサイズ検証
        let meta = tokio::fs::metadata(&temp_path).await?;
        if meta.len() < 1_000_000 {
            tokio::fs::remove_file(&temp_path).await.ok();
            self.emit_progress(
                app_handle,
                model.filename,
                0,
                total_size,
                DownloadStatus::Failed,
            );
            anyhow::bail!(
                "Downloaded file too small: {} bytes (expected ~{})",
                meta.len(),
                model.expected_size
            );
        }

        // 成功: 一時ファイルを正式な場所に移動
        tokio::fs::rename(&temp_path, dest_path)
            .await
            .with_context(|| format!("Failed to rename {:?} to {:?}", temp_path, dest_path))?;

        // 完了通知
        self.emit_progress(
            app_handle,
            model.filename,
            total_size,
            total_size,
            DownloadStatus::Completed,
        );

        eprintln!(
            "[model_downloader] Model downloaded successfully: {:?}",
            dest_path
        );
        Ok(())
    }

    /// 進捗イベントを emit
    fn emit_progress(
        &self,
        app_handle: &AppHandle,
        filename: &str,
        downloaded: u64,
        total: u64,
        status: DownloadStatus,
    ) {
        let percentage = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let progress = DownloadProgress {
            filename: filename.to_string(),
            downloaded,
            total,
            percentage,
            status,
        };

        if let Err(e) = app_handle.emit("model-download-progress", &progress) {
            eprintln!("[model_downloader] Failed to emit progress: {}", e);
        }
    }
}

/// モデルキャッシュディレクトリを取得
fn get_models_cache_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .with_context(|| "Failed to get app data dir")?;

    Ok(app_data_dir.join("models"))
}

/// 両方のモデルパスを取得（初期化時に使用）
pub async fn ensure_models(app_handle: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    let downloader = ModelDownloader::new(app_handle)?;

    // Vision モデルと Text モデルを順番にダウンロード
    let vision_path = downloader.get_vision_model_path(app_handle).await?;
    let text_path = downloader.get_text_model_path(app_handle).await?;

    Ok((vision_path, text_path))
}
