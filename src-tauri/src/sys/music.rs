use crate::{
    db::{self, conn::DbPool},
    error::AppError,
    models::widget::MusicMetaInput,
    sys::{
        image::{save_image_to_origin, save_image_to_thumbnail},
        library::{grouped_by_folder, LibraryRoot},
    },
};
use futures::{
    lock::Mutex,
    stream::{self, StreamExt},
};
use lofty::picture::Picture;
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Tag,
};
use std::{collections::HashSet, io::BufReader, sync::Arc};
use std::{fs, path::PathBuf};
use std::{
    fs::File,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

// 폴더 인덱싱 동시 작업 제한
const LIMIT: usize = 16;

/// 지원하는 오디오 확장자 필터링
pub fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "mp3" | "flac" | "wav" | "m4a" | "ogg"
        )
    } else {
        false
    }
}

fn get_timestamp(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .unwrap()
        .as_secs() as i64
}
pub fn get_mtime(path: &Path) -> i64 {
    let metadata = fs::metadata(&path).ok();
    let mtime = metadata
        .and_then(|m| m.modified().ok())
        .map(|t| get_timestamp(t))
        .unwrap_or(0);
    mtime
}

fn delete_album_art(cache_dir: &PathBuf, file_name: &str) -> Result<(), AppError> {
    fs::remove_file(cache_dir.clone())?;
    fs::remove_file(cache_dir.clone().join("512").join(file_name))?;
    fs::remove_file(cache_dir.clone().join("128").join(file_name))?;
    Ok(())
}

pub fn save_album_art(
    picture: &Picture,
    file_name: &str,
    cache_dir: &PathBuf,
) -> Result<String, AppError> {
    save_album_art_with_options(picture, file_name, cache_dir, false)
}
/// 앨범아트 캐싱 함수
pub fn save_album_art_with_options(
    picture: &Picture,
    file_name: &str,
    cache_dir: &PathBuf,
    overwrite: bool,
) -> Result<String, AppError> {
    let save_dir = cache_dir.join("album_arts");
    // 이미 파일이 존재할 경우를 판정하기 위함
    let target_path = save_dir.join(format!("{}.webp", file_name));
    if !overwrite && target_path.exists() {
        return Ok(target_path.to_string_lossy().to_string());
    }
    let buffer = picture.data();

    save_image_to_thumbnail(save_dir.clone().join("512"), &file_name, buffer, 512)?;
    save_image_to_thumbnail(save_dir.clone().join("128"), &file_name, buffer, 128)?;
    save_image_to_origin(save_dir.clone(), &file_name, buffer)
}

pub fn parse_music_metadata(
    path: &PathBuf,
    mtime: i64,
    cache_dir: &PathBuf,
) -> Result<MusicMetaInput, AppError> {
    let tagged_file = Probe::open(path)
        .map_err(|e| AppError::io(e.to_string()))?
        .read()
        .map_err(|e| AppError::io(e.to_string()))?;
    let properties = tagged_file.properties();
    let duration = properties.duration().as_secs_f64();

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let meta = MusicMetaInput::default(path, tag, cache_dir, duration, mtime);
    Ok(meta)
}

fn visit_dirs(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), AppError> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, paths)?;
            } else if is_audio_file(&path) {
                paths.push(path);
            }
        }
    }
    Ok(())
}

/// 폴더 인덱싱
pub async fn folder_indexing(
    pool: &DbPool,
    folder_path: String,
    cache_dir: &PathBuf,
) -> Result<LibraryRoot, AppError> {
    let cache_dir = Arc::new(cache_dir.clone());
    let pool = Arc::new(pool.clone());
    let found_paths = Arc::new(Mutex::new(HashSet::new()));
    // 캐싱리스트
    let caching_list = Arc::new(db::widget::get_music_list(&*pool, &folder_path.clone()).await?);

    let folder_path_clone = folder_path.clone();
    let music_paths = tokio::task::spawn_blocking(move || {
        let mut paths = Vec::new();
        let _ = visit_dirs(Path::new(&folder_path_clone.clone()), &mut paths)
            .map_err(|e| AppError::validation(format!("폴더를 읽는 중 오류: {}", e), None));
        Ok::<Vec<PathBuf>, String>(paths)
    })
    .await
    .map_err(|e| AppError::validation(format!("태스크 실행 오류: {}", e), None))?
    .map_err(|e| AppError::validation(format!("폴더를 읽는 중 오류: {}", e), None))?;

    futures::stream::iter(music_paths)
        .for_each_concurrent(Some(LIMIT), |path| {
            let found_paths = Arc::clone(&found_paths);
            let caching_list = Arc::clone(&caching_list);
            let db_pool = Arc::clone(&pool);
            let cache_dir = Arc::clone(&cache_dir);

            async move {
                let audio_src = path.to_string_lossy().to_string();

                {
                    // 경로 기록
                    let mut path_lock = found_paths.lock().await;
                    path_lock.insert(audio_src.clone());
                }

                // 캐싱에서 현재 파일 찾기
                let cache = caching_list.iter().find(|p| p.path == audio_src).cloned();

                // 파일 메타데이터 및 수정시간 확인
                let path_for_meta = path.clone();
                let res = tokio::task::spawn_blocking(move || {
                    let mtime = get_mtime(&path_for_meta);

                    let needs_update = match &cache {
                        Some(c) => !c.updated_at.eq(&mtime),
                        None => true,
                    };
                    Ok::<(i64, bool), Box<dyn std::error::Error + Send + Sync>>((
                        mtime,
                        needs_update,
                    ))
                })
                .await;

                let (mtime, needs_update) = match res {
                    Ok(Ok(val)) => val,
                    Ok(Err(e)) => {
                        eprintln!("메타데이터 읽기 실패 [{}]: {}", audio_src, e);
                        return;
                    }
                    Err(e) => {
                        eprint!("태스크 조인 실패: {}", e);
                        return;
                    }
                };

                if needs_update {
                    let cache_dir = Arc::clone(&cache_dir);
                    let p = path.clone();
                    let parsed_result = tokio::task::spawn_blocking(move || {
                        parse_music_metadata(&p, mtime, &cache_dir).map_err(|e| e.to_string())
                    })
                    .await;

                    match parsed_result {
                        Ok(Ok(pared_data)) => {
                            let _ = db::widget::upsert_music_meta(&*db_pool, pared_data).await;
                        }
                        Ok(Err(e)) => eprintln!("태그 파싱 오류 [{}]: {}", audio_src, e),
                        Err(e) => eprintln!("파싱 태스크 조인 오류: {}", e),
                    }
                }
            }
        })
        .await;

    // DB에는 있지만 폴더에 없는 경우 삭제
    let final_found_paths = found_paths.lock().await;
    for cache_item in caching_list.iter() {
        if !final_found_paths.contains(&cache_item.path) {
            db::widget::delete_music_meta(&*pool, &cache_item.path).await?;
            if !cache_item.album_art.is_empty() {
                delete_album_art(&*cache_dir, &cache_item.album_art)?;
            }
        }
    }
    let music_list = db::widget::get_folder_music_data(&*pool, &folder_path).await?;
    let file_list: Vec<(String, serde_json::Value)> = music_list
        .into_iter()
        .filter_map(|item| {
            let path = item.path.clone();
            let value = serde_json::to_value(item.clone()).ok().unwrap_or_default();
            Some((path, value))
        })
        .collect();
    let result = grouped_by_folder(file_list, Some(&folder_path));
    Ok(result)
}

/// 앨범아트 설정
/// 지금은 한장만 교체하는 방식인데, 이후에 여러장을 추가해서 볼 수 있는 기능이 추가되면 좋겠네
pub async fn set_album_art(
    file_path: String,
    img_path: String,
    cache_dir: &PathBuf,
) -> Result<String, AppError> {
    // 파일 열기 및 태그 로드
    let mut tagged_file = Probe::open(&file_path)
        .map_err(|e| AppError::validation(format!("파일 여는 중 오류: {}", e.to_string()), None))?
        .read()
        .map_err(|e| {
            AppError::validation(format!("파일 여는 중 오류: {}", e.to_string()), None)
        })?;

    // 파일명 fallback
    let path = PathBuf::from(&file_path);
    let file_name_fallback = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    // 태그 없으면 생성
    if tagged_file.primary_tag().is_none() {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
    }

    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| AppError::io("태그 생성 실패"))?;

    // 이미지 파일 읽기
    let image_file = File::open(&img_path)
        .map_err(|e| AppError::io(format!("이미지 파일 로드 실패: {}", e.to_string())))?;
    let mut reader = BufReader::new(image_file);

    // Picture 객체 생성
    let picture = Picture::from_reader(&mut reader)
        .map_err(|e| AppError::io(format!("객체 생성 중 실패: {}", e.to_string())))?;
    let picture_size = picture.data().len();
    if picture_size > 1_000_000 {
        // 에러를 생성하고 명시적으로 반환(return)하도록 수정
        return Err(AppError::validation(
            format!("파일 크기가 너무 큼 (1MB 초과): {} bytes", picture_size),
            None
        ));
    }

    // 기존 이미지 교체 및 저장
    tag.set_picture(0, picture.clone());

    tagged_file
        .save_to_path(&file_path, WriteOptions::default())
        .map_err(|e| AppError::io(format!("파일 저장 실패: {}", e)))?;

    save_album_art_with_options(&picture, &file_name_fallback, cache_dir, true)
}
