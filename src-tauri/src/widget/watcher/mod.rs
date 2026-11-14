use std::path::Path;
use std::sync::Mutex;
use std::{path::PathBuf, time::Duration};
use notify::RecommendedWatcher;
use notify::Watcher;
use tauri::{AppHandle, Manager, State};

use crate::db;
use crate::state::AppState;
use crate::sys::music::{get_mtime, is_audio_file, parse_music_metadata};
use crate::{db::conn::DbPool, error::AppError};

pub struct MusicWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl MusicWatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
        }
    }
}

// 1. 음악 폴더 감시 시작 커맨드
#[tauri::command]
pub async fn start_music_watcher(
    app: AppHandle,
    state: State<'_, AppState>,
    music_state: State<'_, MusicWatcherState>,
    folder_path: String,
) -> Result<(), AppError> {
    let mut watcher_guard = music_state.watcher.lock().map_err(|e| AppError::io(e.to_string()))?;
    
    // 이미 감시 중인 워처가 없다면 새로 생성
    if watcher_guard.is_none() {
        let mut pool = state.db.clone();
        let cache_dir = app.path().cache_dir()?;
        let watcher = folder_watching(&mut pool, folder_path, &cache_dir).await?;
        // 상태에 워처 저장
        *watcher_guard = Some(watcher);
    }

    Ok(())
}

// 2. 음악 폴더 감시 중지 커맨드 (Drop을 통한 자원 해제)
#[tauri::command]
pub fn stop_music_watcher(state: State<'_, MusicWatcherState>) -> Result<(), AppError> {
    let mut watcher_guard = state.watcher.lock().map_err(|_| AppError::io("music watcher lock poisoned"))?;

    // Option을 None으로 바꾸면 기존 RecommendedWatcher가 Drop되면서 파일 감시가 완전히 종료됨
    *watcher_guard = None;

    Ok(())
}
pub async fn folder_watching(
    pool: &mut DbPool,
    folder_path: String,
    cache_dir: &PathBuf,
) -> Result<notify::RecommendedWatcher, AppError> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher: notify::RecommendedWatcher = notify::Watcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(2)),
    )
    .map_err(|e| AppError::io(e.to_string()))?;

    // 재귀감시 모드
    watcher.watch(Path::new(&folder_path), notify::RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                handle_fs_event(pool, event, cache_dir).await;
            }
            Ok(Err(e)) => eprintln!("파일 감시 오류: {:?}", e),
            Err(e) => {
                AppError::io(format!("채널 수신 오류: {:?}", e));
                break;
            }
        }
    }

    Ok(watcher)
}

/// 파일 시스템 이벤트 핸들러
async fn handle_fs_event(pool: &mut DbPool, event: notify::Event, cache_dir: &PathBuf) {
    // 이벤트 경로들 순회
    for path in event.paths {
        if !is_audio_file(&path) {
            continue; // 음악 파일이 아니면 무시
        }

        let path_str = path.to_string_lossy();
        match event.kind {
            // 파일 생성 또는 수정 시
            notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                if path.exists() {
                    let mtime = get_mtime(&path);

                    // DB에 UPSERT (있으면 수정 시간 갱신, 없으면 추가)
                    let parsed_result =
                        parse_music_metadata(&path, mtime, &cache_dir).map_err(|e| e.to_string());
                    match parsed_result {
                        Ok(data) => {
                            let result = db::widget::upsert_music_meta(&*pool, data).await;
                            if result.is_ok() {
                                eprintln!("🔄 [동기화 완료] 추가/갱신됨: {}", path_str);
                            }
                        }

                        Err(e) => eprintln!("태그 파싱 오류 [{}]: {}", path_str, e),
                    }
                }
            }
            // 파일 삭제 시
            notify::EventKind::Remove(_) => {
                let result = db::widget::delete_music_meta(&*pool, &path.to_string_lossy()).await;

                match result {
                    Ok(r) => {
                        if r {
                            eprintln!("🗑️ [동기화 완료] 삭제됨: {}", path_str);
                        } else {
                            AppError::database("삭제할 데이터가 존재하지 않음");
                        }
                    }
                    Err(e) => eprintln!("DB 삭제 오류 [{}]: {}", path_str, e),
                }
            }
            _ => {} // 기타 이벤트(Access 등) 무시
        }
    }
}
