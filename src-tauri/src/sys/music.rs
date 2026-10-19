use crate::{
    db::{self, conn::DbPool},
    error::{AppError, Result},
    models::widget::{FolderNode, LibraryRoot, MusicMeta, MusicMetaInput},
    sys::image::{save_image_to_origin, save_image_to_thumbnail},
};
use lofty::picture::Picture;

use futures::{
    lock::Mutex,
    stream::{self, StreamExt},
};
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::{Accessor, ItemKey, Tag},
};
use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
    sync::Arc,
};
use std::{fs, path::PathBuf};
use std::{
    fs::File,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn get_timestamp(time: SystemTime) -> Result<i64> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Io(e.to_string()))
        .unwrap()
        .as_secs() as i64)
}

fn delete_album_art(cache_dir: &PathBuf, file_name: &str) -> Result<()> {
    fs::remove_file(cache_dir.clone())?;
    fs::remove_file(cache_dir.clone().join("512").join(file_name))?;
    fs::remove_file(cache_dir.clone().join("128").join(file_name))?;
    Ok(())
}

fn save_album_art(picture: &Picture, file_name: &str, cache_dir: &PathBuf) -> Result<String> {
    save_album_art_with_options(picture, file_name, cache_dir, false)
}

fn save_album_art_with_options(
    picture: &Picture,
    file_name: &str,
    cache_dir: &PathBuf,
    overwrite: bool,
) -> Result<String> {
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

fn parse_music_metadata(path: &PathBuf, mtime: i64, cache_dir: &PathBuf) -> Result<MusicMetaInput> {
    let tagged_file = Probe::open(path)
        .map_err(|e| AppError::Io(e.to_string()))?
        .read()
        .map_err(|e| AppError::Io(e.to_string()))?;
    let properties = tagged_file.properties();
    let duration = properties.duration().as_secs_f64();
    let audio_src = path.to_string_lossy().to_string();

    // 파일명 fallback
    let file_name_fallback = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let meta = match tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        Some(tag) => {
            let album_art = tag
                .pictures()
                .first()
                .and_then(|p| save_album_art(p, &file_name_fallback, cache_dir).ok());
            MusicMetaInput {
                path: audio_src,
                title: tag.title().map(String::from).unwrap_or(file_name_fallback),
                artist: tag.artist().map(String::from),
                album: tag.album().map(String::from),
                album_title: tag.get_string(ItemKey::AlbumTitle).map(String::from),
                album_artist: tag.get_string(ItemKey::AlbumArtist).map(String::from),
                track_number: tag.get_string(ItemKey::TrackNumber).map(String::from),
                genre: tag.genre().map(String::from),
                comment: tag.comment().map(String::from),
                album_art,
                lyrics: tag.get_string(ItemKey::Lyrics).map(String::from),
                duration,
                updated_at: mtime,
            }
        }
        None => MusicMetaInput {
            path: audio_src,
            title: file_name_fallback,
            artist: None,
            album: None,
            album_art: None,
            album_artist: None,
            track_number: None,
            album_title: None,
            comment: None,
            genre: None,
            lyrics: None,
            duration,
            updated_at: mtime,
        },
    };

    // 데이터 조합
    Ok(meta)
}

fn visit_dirs(dir: &Path, allowed: &[&str], paths: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, allowed, paths)?;
            } else if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .is_some_and(|s| allowed.contains(&s.as_str()))
            {
                paths.push(path);
            }
        }
    }
    Ok(())
}

// 🛠️ FolderNode에 재귀적으로 파일을 추가하는 메서드를 작성합니다.
impl FolderNode {
    fn insert_file(&mut self, folder_parts: &[&str], file: MusicMeta) {
        if folder_parts.is_empty() {
            self.files.push(file);
            return;
        }

        let current_folder = folder_parts[0];
        let remaining_parts = &folder_parts[1..];

        let next_node = self
            .folders
            .entry(current_folder.to_string())
            .or_insert_with(|| FolderNode {
                name: current_folder.to_string(),
                folders: HashMap::new(),
                files: Vec::new(),
            });

        // 재귀 호출을 통해 다음 레벨로 가변 참조를 안전하게 넘깁니다.
        next_node.insert_file(remaining_parts, file);
    }
}

pub fn grouped_by_folder(music_list: Vec<MusicMeta>, selected_folder: Option<&str>) -> LibraryRoot {
    let mut root_tree: HashMap<String, FolderNode> = HashMap::new();
    let mut root_files: Vec<MusicMeta> = Vec::new();

    let root_path = selected_folder
        .filter(|f| !f.is_empty())
        .map(|f| format!("{}\\", f))
        .unwrap_or_default();

    for file in music_list {
        let path = if !root_path.is_empty() {
            file.path.strip_prefix(&root_path).unwrap_or(&file.path)
        } else {
            &file.path
        };

        let parts: Vec<&str> = path.split('\\').collect();
        if parts.is_empty() {
            continue;
        }

        let folder_parts = &parts[..parts.len() - 1];

        if folder_parts.is_empty() {
            root_files.push(file);
            continue;
        }

        // 🛠️ 재귀 메서드 호출로 복잡한 루프 대여 문제를 한 줄로 해결
        let first_folder = folder_parts[0];
        let remaining_parts = &folder_parts[1..];

        root_tree
            .entry(first_folder.to_string())
            .or_insert_with(|| FolderNode {
                name: first_folder.to_string(),
                folders: HashMap::new(),
                files: Vec::new(),
            })
            .insert_file(remaining_parts, file.clone());
    }

    LibraryRoot {
        folders: root_tree,
        root_files,
    }
}

pub async fn folder_indexing(
    pool: &DbPool,
    folder_path: String,
    cache_dir: &PathBuf,
) -> Result<LibraryRoot> {
    let limit = 16; // 동시 작업 제한
    let allowed = ["mp3", "wav", "flac"];

    let cache_dir = Arc::new(cache_dir.clone());
    let pool = Arc::new(pool.clone());
    let found_paths = Arc::new(Mutex::new(HashSet::new()));
    // 캐싱리스트
    let caching_list = Arc::new(db::widget::get_music_list(&*pool, &folder_path.clone()).await?);

    let folder_path_clone = folder_path.clone();
    let music_paths = tokio::task::spawn_blocking(move || {
        let mut paths = Vec::new();
        let _ = visit_dirs(Path::new(&folder_path_clone.clone()), &allowed, &mut paths)
            .map_err(|e| AppError::Validation(format!("폴더를 읽는 중 오류: {}", e).into()));
        Ok::<Vec<PathBuf>, String>(paths)
    })
    .await
    .map_err(|e| AppError::Validation(format!("태스크 실행 오류: {}", e).into()))?
    .map_err(|e| AppError::Validation(format!("폴더를 읽는 중 오류: {}", e).into()))?;

    futures::stream::iter(music_paths)
        .for_each_concurrent(Some(limit), |path| {
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
                let is_cache = cache.is_some();

                // 파일 메타데이터 및 수정시간 확인
                let path_for_meta = path.clone();
                let res = tokio::task::spawn_blocking(move || {
                    let metadata = fs::metadata(&path_for_meta)?;
                    let mtime_sys: SystemTime = metadata.modified()?;
                    let mtime = get_timestamp(mtime_sys)?;

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
                            if is_cache {
                                let _ = db::widget::update_music_meta(&*db_pool, pared_data).await;
                            } else {
                                let _ = db::widget::create_music_meta(&*db_pool, pared_data).await;
                            };
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
    let result = grouped_by_folder(music_list, Some(&folder_path));
    Ok(result)
}

pub async fn set_album_art(
    file_path: String,
    img_path: String,
    cache_dir: &PathBuf,
) -> Result<String> {
    // 파일 열기 및 태그 로드
    let mut tagged_file = Probe::open(&file_path)
        .map_err(|e| AppError::Validation(format!("파일 여는 중 오류: {}", e.to_string()).into()))?
        .read()
        .map_err(|e| {
            AppError::Validation(format!("파일 여는 중 오류: {}", e.to_string()).into())
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
        .ok_or_else(|| AppError::Io("태그 생성 실패".into()))?;

    // 이미지 파일 읽기
    let image_file = File::open(&img_path)
        .map_err(|e| AppError::Io(format!("이미지 파일 로드 실패: {}", e.to_string()).into()))?;
    let mut reader = BufReader::new(image_file);

    // Picture 객체 생성
    let picture = Picture::from_reader(&mut reader)
        .map_err(|e| AppError::Io(format!("객체 생성 중 실패: {}", e.to_string()).into()))?;
    let picture_size = picture.data().len();
    if picture_size > 1_000_000 {
        // 에러를 생성하고 명시적으로 반환(return)하도록 수정
        return Err(AppError::Validation(
            format!("파일 크기가 너무 큼 (1MB 초과): {} bytes", picture_size).into(),
        ));
    }

    // 기존 이미지 교체 및 저장
    tag.set_picture(0, picture.clone());

    tagged_file
        .save_to_path(&file_path, WriteOptions::default())
        .map_err(|e| AppError::Io(format!("파일 저장 실패: {}", e)))?;

    save_album_art_with_options(&picture, &file_name_fallback, cache_dir, true)
}
