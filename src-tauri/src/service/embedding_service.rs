//! CLIP 埋め込みサービス
//!
//! Phase 4: tract-onnx (Pure Rust) を使用して CLIP モデルから埋め込みベクトルを生成する
//! 注意: 現在は vision_model.onnx のみをサポート (text_model.onnx は tract と互換性問題あり)

use anyhow::{Context, Result};
use ndarray::Array4;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tract_onnx::prelude::*;

/// CLIP 埋め込みの次元数
pub const EMBEDDING_DIM: usize = 512;

/// 現在の埋め込みモデルバージョン（モデル更新時にインクリメント）
pub const EMBEDDING_VERSION: i32 = 1;

/// 埋め込みベクトルを L2 正規化
fn normalize_embedding(embedding: &[f32]) -> Vec<f32> {
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        embedding.iter().map(|x| x / norm).collect()
    } else {
        embedding.to_vec()
    }
}

/// コサイン類似度を計算
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    // 既に正規化済みなので内積がコサイン類似度
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// 埋め込みベクトルの平均を計算
pub fn average_embeddings(embeddings: &[Vec<f32>]) -> Vec<f32> {
    if embeddings.is_empty() {
        return vec![0.0; EMBEDDING_DIM];
    }

    let mut avg = vec![0.0f32; EMBEDDING_DIM];
    for emb in embeddings {
        for (i, v) in emb.iter().enumerate() {
            if i < EMBEDDING_DIM {
                avg[i] += v;
            }
        }
    }

    let n = embeddings.len() as f32;
    for v in &mut avg {
        *v /= n;
    }

    // 正規化
    normalize_embedding(&avg)
}

/// 埋め込みベクトルを bytes から復元
pub fn embedding_from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// 埋め込みベクトルを bytes に変換
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

// tract の Model 型エイリアス
type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// CLIP 埋め込みサービス (Vision のみ)
/// 注意: text_model.onnx は tract と互換性がないため、現在は画像埋め込みのみをサポート
pub struct EmbeddingService {
    vision_model: Arc<RwLock<TractModel>>,
    /// 処理中フラグ
    is_processing: Arc<RwLock<bool>>,
}

impl EmbeddingService {
    /// ONNX モデルを読み込んでサービスを初期化
    pub fn init(resource_dir: &Path) -> Result<Self> {
        let vision_model_path = resource_dir.join("vision_model.onnx");

        // Vision モデルの読み込み
        let raw_vision_model = tract_onnx::onnx()
            .model_for_path(&vision_model_path)
            .context("Failed to load vision model")?;

        println!(
            "Vision model input count: {}",
            raw_vision_model.inputs.len()
        );

        // 入力: pixel_values [1, 3, 224, 224]
        let vision_model = raw_vision_model
            .with_input_fact(0, f32::fact([1, 3, 224, 224]).into())
            .context("Failed to set vision model input")?
            .into_optimized()
            .context("Failed to optimize vision model")?
            .into_runnable()
            .context("Failed to create runnable vision model")?;

        Ok(Self {
            vision_model: Arc::new(RwLock::new(vision_model)),
            is_processing: Arc::new(RwLock::new(false)),
        })
    }

    /// 処理中かどうかを取得
    pub async fn is_processing(&self) -> bool {
        *self.is_processing.read().await
    }

    /// 処理中フラグを設定
    pub async fn set_processing(&self, processing: bool) {
        *self.is_processing.write().await = processing;
    }

    /// 画像の埋め込みベクトルを生成
    /// 入力: 画像データ (JPEG/PNG など、image crate がデコードできる形式)
    pub async fn generate_image_embedding(&self, image_data: &[u8]) -> Result<Vec<f32>> {
        // 画像をデコードしてリサイズ
        let img = image::load_from_memory(image_data).context("Failed to decode image")?;
        let img = img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
        let rgb = img.to_rgb8();

        // 画像を正規化して NCHW 形式に変換
        // CLIP の正規化: mean=[0.48145466, 0.4578275, 0.40821073], std=[0.26862954, 0.26130258, 0.27577711]
        let mean = [0.48145466f32, 0.4578275, 0.40821073];
        let std = [0.26862954f32, 0.261_302_6, 0.275_777_1];

        let mut input_array = Array4::<f32>::zeros((1, 3, 224, 224));
        for y in 0..224 {
            for x in 0..224 {
                let pixel = rgb.get_pixel(x as u32, y as u32);
                for c in 0..3 {
                    let value = pixel[c] as f32 / 255.0;
                    input_array[[0, c, y, x]] = (value - mean[c]) / std[c];
                }
            }
        }

        // 推論を実行
        let model = self.vision_model.read().await;
        let input_tensor: Tensor = input_array.into();
        let result = model
            .run(tvec!(input_tensor.into()))
            .context("Vision model inference failed")?;

        // 出力から埋め込みを取得
        let output = result[0]
            .to_array_view::<f32>()
            .context("Failed to extract vision output")?;
        let embedding: Vec<f32> = output.iter().copied().collect();

        // 正規化
        let normalized = normalize_embedding(&embedding);

        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_embedding() {
        let emb = vec![3.0, 4.0];
        let normalized = normalize_embedding(&emb);
        assert!((normalized[0] - 0.6).abs() < 0.001);
        assert!((normalized[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 0.001);

        let c = vec![1.0, 0.0];
        let d = vec![1.0, 0.0];
        assert!((cosine_similarity(&c, &d) - 1.0).abs() < 0.001);
    }
}
