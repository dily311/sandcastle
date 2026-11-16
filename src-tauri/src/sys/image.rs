use crate::error::{AppError, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use image::DynamicImage;
use std::{fs, path::PathBuf};

fn save_image(image: DynamicImage, save_dir: &PathBuf) -> Result<()> {
    image
        .save_with_format(&save_dir, image::ImageFormat::WebP)
        .map_err(|e| AppError::io(e.to_string()))?;
    Ok(())
}

pub fn save_image_to_origin(save_dir: PathBuf, file_name: &str, buffer: &[u8]) -> Result<String> {
    // 없는 경우 생성
    fs::create_dir_all(&save_dir)?;

    // 이후 이미지 저장 로직
    let target_path = save_dir.join(format!("{}.webp", file_name));
    let image = image::load_from_memory(buffer).map_err(|e| AppError::io(e.to_string()))?;

    save_image(image, &target_path)?;

    Ok(target_path.to_string_lossy().to_string())
}

pub fn save_image_to_thumbnail(save_dir: PathBuf, file_name: &str, buffer: &[u8], thumbnail_size: u32) -> Result<String> {
    // 없는 경우 생성
    fs::create_dir_all(&save_dir)?;

    // 이후 이미지 저장 로직
    let target_path = save_dir.join(format!("{}.webp", file_name));
    let image = image::load_from_memory(buffer).map_err(|e| AppError::io(e.to_string()))?;
    let thumbnail = image.thumbnail(thumbnail_size, thumbnail_size);

    save_image(thumbnail, &target_path)?;

    Ok(target_path.to_string_lossy().to_string())
}

pub fn image_to_base64(buffer: &[u8]) -> String {
    let encoded = BASE64_STANDARD.encode(buffer);
    let mime_type = infer::get(buffer).map(|kind| kind.mime_type()).unwrap_or("image/jpeg");

    return format!("data:{};base64,{}", mime_type, encoded);
}
