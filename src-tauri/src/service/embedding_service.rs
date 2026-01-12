//! CLIP 埋め込みサービス
//!
//! Phase 4: ONNX Runtime を使用して CLIP モデルから埋め込みベクトルを生成する
//!
//! この機能は `clip-recommendation` feature が有効な場合のみビルドされる

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

// ============================================================================
// CLIP 機能 (clip-recommendation feature 有効時のみ)
// ============================================================================

#[cfg(feature = "clip-recommendation")]
mod clip {
    use super::*;
    use anyhow::Result;
    use ort::session::builder::GraphOptimizationLevel;
    use ort::session::Session;
    use ort::value::Tensor;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// CLIP 埋め込みサービス
    pub struct EmbeddingService {
        vision_session: Arc<RwLock<Session>>,
        text_session: Arc<RwLock<Session>>,
        /// 処理中フラグ
        is_processing: Arc<RwLock<bool>>,
    }

    impl EmbeddingService {
        /// ONNX モデルを読み込んでサービスを初期化
        pub fn init(resource_dir: &Path) -> Result<Self> {
            let vision_model_path = resource_dir.join("vision_model.onnx");
            let text_model_path = resource_dir.join("text_model.onnx");

            // ONNX Runtime セッションを作成
            let vision_session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file(&vision_model_path)?;

            let text_session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file(&text_model_path)?;

            Ok(Self {
                vision_session: Arc::new(RwLock::new(vision_session)),
                text_session: Arc::new(RwLock::new(text_session)),
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
        /// 入力: 224x224 RGB 画像データ (JPEG デコード済み)
        pub async fn generate_image_embedding(&self, image_data: &[u8]) -> Result<Vec<f32>> {
            // JPEG データを画像としてデコード
            let img = image::load_from_memory(image_data)?;
            let img = img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
            let rgb = img.to_rgb8();

            // 画像を正規化して NCHW 形式に変換
            // CLIP の正規化: mean=[0.48145466, 0.4578275, 0.40821073], std=[0.26862954, 0.26130258, 0.27577711]
            let mean = [0.48145466f32, 0.4578275, 0.40821073];
            let std = [0.26862954f32, 0.26130258, 0.27577711];

            let mut input_data = vec![0.0f32; 1 * 3 * 224 * 224];
            for y in 0..224 {
                for x in 0..224 {
                    let pixel = rgb.get_pixel(x as u32, y as u32);
                    for c in 0..3 {
                        let value = pixel[c] as f32 / 255.0;
                        input_data[c * 224 * 224 + y * 224 + x] = (value - mean[c]) / std[c];
                    }
                }
            }

            // 推論を実行
            let mut session = self.vision_session.write().await;
            let input_tensor = Tensor::from_array(([1i64, 3, 224, 224], input_data))?;
            let outputs = session.run(ort::inputs![input_tensor])?;

            // 出力から埋め込みを取得
            let output = &outputs[0];
            let (_, embedding_data) = output.try_extract_tensor::<f32>()?;
            let embedding: Vec<f32> = embedding_data.iter().copied().collect();

            // 正規化
            let normalized = normalize_embedding(&embedding);

            Ok(normalized)
        }

        /// テキストの埋め込みベクトルを生成
        /// 入力: テキスト文字列（フォルダパス/タイトル）
        pub async fn generate_text_embedding(&self, text: &str) -> Result<Vec<f32>> {
            // 簡易トークナイザ: CLIP は最大 77 トークン
            let tokens = simple_tokenize(text);

            // attention_mask を作成
            let attention_mask: Vec<i64> =
                tokens.iter().map(|&t| if t != 0 { 1 } else { 0 }).collect();

            // 推論を実行
            let mut session = self.text_session.write().await;
            let input_ids_tensor = Tensor::from_array(([1i64, 77], tokens))?;
            let attention_mask_tensor = Tensor::from_array(([1i64, 77], attention_mask))?;
            let outputs = session.run(ort::inputs![input_ids_tensor, attention_mask_tensor])?;

            // 出力から埋め込みを取得
            let output = &outputs[0];
            let (_, embedding_data) = output.try_extract_tensor::<f32>()?;
            let embedding: Vec<f32> = embedding_data.iter().copied().collect();

            // 正規化
            let normalized = normalize_embedding(&embedding);

            Ok(normalized)
        }
    }

    /// 簡易トークナイザ
    /// CLIP の BPE トークナイザを完全に再現するのは複雑なので、
    /// ここでは ASCII 文字をそのまま使用する簡易版
    fn simple_tokenize(text: &str) -> Vec<i64> {
        const MAX_LEN: usize = 77;
        const BOS_TOKEN: i64 = 49406; // <|startoftext|>
        const EOS_TOKEN: i64 = 49407; // <|endoftext|>
        const PAD_TOKEN: i64 = 0;

        let mut tokens = vec![BOS_TOKEN];

        // テキストを小文字に変換し、単語単位で処理
        for c in text.to_lowercase().chars().take(MAX_LEN - 2) {
            let token = match c {
                'a'..='z' => (c as i64 - 'a' as i64) + 320, // 320-345 for a-z
                '0'..='9' => (c as i64 - '0' as i64) + 346, // 346-355 for 0-9
                ' ' => 256,                                 // space
                '/' => 257,                                 // slash
                '\\' => 258,                                // backslash
                '_' => 259,                                 // underscore
                '-' => 260,                                 // hyphen
                '.' => 261,                                 // dot
                '(' => 262,                                 // open paren
                ')' => 263,                                 // close paren
                '[' => 264,                                 // open bracket
                ']' => 265,                                 // close bracket
                _ => 266, // その他の文字は共通トークンにマッピング
            };
            tokens.push(token);
        }

        tokens.push(EOS_TOKEN);

        // MAX_LEN までパディング
        while tokens.len() < MAX_LEN {
            tokens.push(PAD_TOKEN);
        }

        tokens.truncate(MAX_LEN);
        tokens
    }
}

#[cfg(feature = "clip-recommendation")]
pub use clip::EmbeddingService;

// ============================================================================
// スタブ実装 (clip-recommendation feature 無効時)
// ============================================================================

#[cfg(not(feature = "clip-recommendation"))]
pub struct EmbeddingService;

#[cfg(not(feature = "clip-recommendation"))]
impl EmbeddingService {
    pub fn init(_resource_dir: &std::path::Path) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!(
            "CLIP recommendation feature is not enabled"
        ))
    }

    pub async fn is_processing(&self) -> bool {
        false
    }

    pub async fn set_processing(&self, _processing: bool) {}

    pub async fn generate_image_embedding(&self, _image_data: &[u8]) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "CLIP recommendation feature is not enabled"
        ))
    }

    pub async fn generate_text_embedding(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "CLIP recommendation feature is not enabled"
        ))
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
