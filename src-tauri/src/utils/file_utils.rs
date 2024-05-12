use std::path::Path;

pub(crate) fn get_parent_dir(path: &str) -> String {
    Path::new(path)
        .parent()
        .unwrap_or(Path::new("Unknown"))
        .to_str()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn get_parent_dir_name(path: &str) -> String {
    Path::new(path)
        .parent()
        .unwrap_or(Path::new("Unknown"))
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn get_filename_without_extension(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn is_executable_file(path: &str) -> bool {
    is_image_file(path) || is_video_file(path)
}

pub(crate) fn is_image_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    get_image_extensions().iter().any(|v| *v == ext)
}

pub(crate) fn is_video_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    get_video_extensions().iter().any(|v| *v == ext)
}

pub(crate) fn is_compressed_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    get_compressed_extensions().iter().any(|v| *v == ext)
}

pub(crate) fn get_jpeg_extensions() -> Vec<String> {
    vec!["jpg", "jpeg", "JPG", "JPEG", "jpe", "jfif", "pjpeg", "pjp"]
        .iter()
        .map(|v| v.to_string())
        .collect()
}

pub(crate) fn get_png_extensions() -> Vec<String> {
    vec!["png", "PNG"].iter().map(|v| v.to_string()).collect()
}

pub(crate) fn get_gif_extensions() -> Vec<String> {
    vec!["gif"].iter().map(|v| v.to_string()).collect()
}

pub(crate) fn get_tiff_extensions() -> Vec<String> {
    vec!["tif", "tiff"].iter().map(|v| v.to_string()).collect()
}

pub(crate) fn get_bmp_extensions() -> Vec<String> {
    vec!["bmp", "dib"].iter().map(|v| v.to_string()).collect()
}

pub(crate) fn get_webp_extensions() -> Vec<String> {
    vec!["webp"].iter().map(|v| v.to_string()).collect()
}

pub(crate) fn get_image_extensions() -> Vec<String> {
    let mut extensions = vec![];
    extensions.extend(get_jpeg_extensions());
    extensions.extend(get_png_extensions());
    extensions.extend(get_gif_extensions());
    extensions.extend(get_tiff_extensions());
    extensions.extend(get_bmp_extensions());
    extensions.extend(get_webp_extensions());
    extensions
}

pub(crate) fn get_video_extensions() -> Vec<String> {
    vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm"]
        .iter()
        .map(|v| v.to_string())
        .collect()
}

pub(crate) fn get_compressed_extensions() -> Vec<String> {
    vec!["zip", "tar", "gz", "bz2", "xz", "7z"]
        .iter()
        .map(|v| v.to_string())
        .collect()
}

pub(crate) fn get_any_extensions() -> Vec<String> {
    let mut extensions = vec![];
    extensions.extend(get_image_extensions());
    extensions.extend(get_video_extensions());
    extensions.extend(get_compressed_extensions());
    extensions
}
