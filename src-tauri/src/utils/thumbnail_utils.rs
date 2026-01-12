//! サムネイル画像処理ユーティリティ
//!
//! Phase 2: 224×224 にリサイズしたサムネイル画像データを生成する

use anyhow::Result;
use image::imageops::FilterType;
use image::ImageFormat;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::Path;

/// サムネイル生成結果
pub struct ThumbnailData {
    /// JPEG エンコードされたサムネイル画像データ
    pub blob: Vec<u8>,
    /// サムネイルの SHA256 ハッシュ
    pub hash: String,
}

/// サムネイルのサイズ（幅・高さ）
pub const THUMBNAIL_SIZE: u32 = 224;

/// 画像ファイルから 224×224 のサムネイルを生成する
pub fn generate_thumbnail_data(image_path: &str) -> Result<ThumbnailData> {
    let path = Path::new(image_path);

    // 画像を読み込み
    let img = image::open(path)?;

    // 224×224 にリサイズ（アスペクト比を維持し、余白を黒で埋める）
    let resized = resize_with_padding(&img, THUMBNAIL_SIZE, THUMBNAIL_SIZE);

    // JPEG にエンコード
    let mut buf = Cursor::new(Vec::new());
    resized.write_to(&mut buf, ImageFormat::Jpeg)?;
    let blob = buf.into_inner();

    // SHA256 ハッシュを計算
    let hash = format!("{:x}", Sha256::digest(&blob));

    Ok(ThumbnailData { blob, hash })
}

/// 画像をアスペクト比を維持してリサイズし、余白を黒で埋める
fn resize_with_padding(
    img: &image::DynamicImage,
    target_width: u32,
    target_height: u32,
) -> image::DynamicImage {
    use image::{Rgb, RgbImage};

    let (orig_width, orig_height) = (img.width(), img.height());

    // アスペクト比を計算し、ターゲットに収まるようにリサイズ
    let scale = f64::min(
        target_width as f64 / orig_width as f64,
        target_height as f64 / orig_height as f64,
    );

    let new_width = (orig_width as f64 * scale) as u32;
    let new_height = (orig_height as f64 * scale) as u32;

    // リサイズ
    let resized = img.resize_exact(new_width, new_height, FilterType::Lanczos3);

    // 黒背景の画像を作成
    let mut output = RgbImage::from_pixel(target_width, target_height, Rgb([0, 0, 0]));

    // 中央に配置
    let x_offset = (target_width - new_width) / 2;
    let y_offset = (target_height - new_height) / 2;

    // リサイズした画像を貼り付け
    let resized_rgb = resized.to_rgb8();
    for y in 0..new_height {
        for x in 0..new_width {
            let pixel = resized_rgb.get_pixel(x, y);
            output.put_pixel(x + x_offset, y + y_offset, *pixel);
        }
    }

    image::DynamicImage::ImageRgb8(output)
}

/// 画像のハッシュを計算する（変更検知用）
/// サムネイル生成前に変更があるかどうかを確認するために使用
pub fn calculate_image_hash(image_path: &str) -> Result<String> {
    let data = std::fs::read(image_path)?;
    let hash = format!("{:x}", Sha256::digest(&data));
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_size() {
        assert_eq!(THUMBNAIL_SIZE, 224);
    }
}
