//! 게임 패키지 생성/불러오기/저장/삭제/내보내기를 담당.
//! 게임 실행 중 데이터(위치/시간/날씨/세이브 등)는 절대 안 건드린다 — 그건 GameRuntime 몫.
//!
//! "패키지"라는 단위가 가리키는 것:
//!   - DB의 game_projects 및 그 아래 테이블들 (정의 데이터 전체)
//!   - <app_data>/games/<game_id>/ 아래의 scripts/assets 파일들 (protocol.rs가 game://로 서빙)
use crate::db::game::delete_game_project;
use crate::error::AppError;
use crate::game::package::{self, PackageData};
use crate::game::template;
use crate::game::runtime::repository::{GameRepository, SqlxGameRepository};
use crate::game::validate::{self, Issue};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct GameManager;

/// save_package의 결과: 에디터에서 지워서 DB에서도 지워진 행 수, 그리고 저장 후 검증 결과.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReport {
    pub deleted: BTreeMap<&'static str, usize>,
    /// 저장 후 상태의 진단 결과 (open_game의 diagnostics와 같은 형식)
    pub diagnostics: Vec<Issue>,
}

impl GameManager {
    /// 템플릿으로 새 게임을 만든다: 번들된 템플릿 패키지(.tgame)를 import하고 제목/template_id를 덮어쓴다.
    /// import 경로를 그대로 타서 id가 전부 새로 발급되므로 같은 템플릿으로 여러 게임을 만들어도 안 겹친다.
    /// 템플릿 패키지에 기본 시간대와 location이 들어 있어서 만들자마자 시작할 수 있다.
    pub async fn create_game(pool: &SqlitePool, games_root: &Path, title: String, template_id: String) -> Result<String, AppError> {
        let title = title.trim().to_string();
        if title.is_empty() || title.chars().count() > 100 {
            return Err(AppError::validation("제목 길이가 올바르지 않습니다", None));
        }
        let template = template::find(&template_id).ok_or_else(|| AppError::validation(format!("알 수 없는 템플릿입니다: {template_id}"), None))?;
        let bytes = template::package_bytes(&template.id).ok_or_else(|| AppError::not_found("템플릿 패키지가 없습니다"))?;
        package::import_bytes(pool, games_root, bytes, Some(package::ImportOverrides { title, template_id: template.id })).await
    }

    /// 이미 로컬에 있는(= DB에 game_projects row가 있는) 게임의 폴더를 확인해서 경로를 돌려준다.
    /// active_archive에 올리는 건 호출하는 쪽(command)이 한다. 남이 만든 .tgame을 처음 가져오는 건
    /// 이 함수가 아니라 import_package.
    pub async fn load_package(pool: &SqlitePool, games_root: &Path, game_id: &str) -> Result<PathBuf, AppError> {
        let _ = pool;
        let dir = games_root.join(game_id);
        if !dir.join("index.html").exists() {
            return Err(AppError::not_found("게임 패키지 파일을 찾을 수 없습니다"));
        }
        // TODO: db::game::get_game_project로 존재/소유권 확인도 같이 할 것
        Ok(dir)
    }

    /// 에디터에서 정의 데이터(맵/이벤트/씬 등)를 고친 걸 DB에 반영한다.
    /// **id를 보존**한다 — 받은 데이터를 id 기준으로 upsert하고, 거기 없는 행만 삭제한다
    /// (세이브/세션/이벤트 로그가 정의 행의 id를 참조하므로 "통째로 지우고 다시 쓰기"는 안 된다).
    /// 기존 행은 반드시 id를 달아 보내야 하고, id 없는 행은 새 행으로 취급된다.
    /// 월드인포 북은 전역 테이블이라 여기서 다루지 않는다 (project/characters의 world_info_ids만 저장).
    pub async fn save_package(pool: &SqlitePool, games_root: &Path, game_id: &str, data: PackageData) -> Result<SaveReport, AppError> {
        let repo = SqlxGameRepository::new(pool);
        let existing = repo.load_definition(game_id).await.map_err(|e| AppError::database(e.to_string()))?;
        let (deleted, removed_files) = package::save_definition(pool, game_id, &existing, data).await?;
        // DB가 커밋된 뒤에 지워진 에셋의 파일을 지운다 (DB가 실패하면 파일은 그대로 남는다).
        package::remove_asset_files(games_root, game_id, &removed_files);

        // 저장 후 상태를 다시 읽어 검증해서 돌려준다 (저장은 이미 끝났다 — 이건 안내용).
        let after = repo.load_definition(game_id).await.map_err(|e| AppError::database(e.to_string()))?;
        Ok(SaveReport { deleted, diagnostics: validate::diagnose(&after, Some(games_root.join(game_id).as_path())) })
    }

    /// 게임을 지운다. game_projects가 지워지면 FK CASCADE로 정의 테이블과 세션/세이브/일지/대화가 같이
    /// 지워진다 (003_game_mode.sql 참고) — 플레이 기록까지 사라지니 호출 전에 사용자 확인이 필요하다.
    /// DB를 먼저 지우고 폴더를 지운다 (DB가 실패하면 폴더는 그대로 둔다).
    /// ⚠ 지금 실행 중인 게임이면 호출하는 쪽에서 막아야 한다 (active_archive/active_runtime 확인).
    /// ⚠ 임포트하면서 만들어진 월드인포 북은 전역 테이블이라 같이 지워지지 않는다 (고아로 남음).
    pub async fn delete_package(pool: &SqlitePool, games_root: &Path, game_id: &str) -> Result<(), AppError> {
        delete_game_project(pool, game_id).await?;
        let dir = games_root.join(game_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// 남이 만든 .tgame을 내 라이브러리로 가져온다 (package.rs가 zip 검증/해제 담당).
    pub async fn import_package(pool: &SqlitePool, games_root: &Path, zip_path: &Path) -> Result<String, AppError> {
        package::import(pool, games_root, zip_path).await
    }

    /// 내가 만든 게임을 남에게 줄 .tgame으로 내보낸다.
    pub async fn export_package(pool: &SqlitePool, games_root: &Path, game_id: &str, output_dir: &Path) -> Result<PathBuf, AppError> {
        let repo = SqlxGameRepository::new(pool);
        let definition = repo.load_definition(game_id).await.map_err(|e| AppError::database(e.to_string()))?;
        package::export(pool, games_root, game_id, &definition, output_dir).await
    }
}
