//! 에셋 파일 추가/삭제. 호스트 에디터가 파일 대화상자로 고른 이미지/오디오를 게임 폴더(games/<id>/assets/)로
//! 복사하고 game_assets(+ 종류별 하위 테이블)에 행을 만든다. 삭제는 행과 파일을 같이 지운다.
//!
//! 규칙 (ADR-018 갱신과 save_package 흐름을 따른다):
//!   - 파일 형식은 확장자가 아니라 **내용(매직 바이트, infer)** 으로 판정하고, 저장할 확장자도 판정 결과에서 정한다
//!     (이름만 .png인 실행 파일 같은 것을 막는다). 서빙 허용 목록(protocol.rs)에 없는 형식(SVG, BMP, 동영상 등)은 거절한다.
//!   - 크기 상한은 서빙 상한과 같다 (protocol::media_limits). 복사한 뒤에도 다시 확인한다 (그 사이 원본이 바뀔 수 있다).
//!   - 저장 경로는 `assets/<category>/<uuid>.<ext>` — 원본 파일 이름은 쓰지 않는다(경로 문자/충돌 방지).
//!     복사할 폴더가 게임 폴더 밖으로 이어지면(심볼릭 링크 등) ARCHIVE_UNSAFE_PATH.
//!   - DB를 먼저(트랜잭션) 쓰고, 실패하면 복사한 파일을 지운다. 삭제는 DB를 먼저 지우고 커밋된 뒤에 파일을 지운다
//!     (save_package와 같은 순서: DB가 실패하면 파일은 그대로 남는다).
//!   - 다른 곳(씬 대본, 이벤트/선택지/아이템 효과의 stage, 표지/아바타/아이콘 경로)이 쓰는 에셋은 기본적으로 지우지 않는다.
//!     force=true면 지우고, 그 결과 생긴 문제는 진단(diagnostics)으로 돌려준다.
//!   - 정의는 open_game 때 메모리에 올라가므로, 플레이 중인 게임에는 다시 open_game을 불러야 반영된다 (save_package와 같다).
use crate::db::game::{delete_game_asset, upsert_audio_asset, upsert_background_assets, upsert_cg_asset, upsert_character_asset, upsert_game_asset};
use crate::error::{AppError, ErrorCode};
use crate::game::runtime::{GameDefinition, GameRepository, SqlxGameRepository};
use crate::game::validate::{self, Issue};
use crate::models::game::{AudioAssetsInput, BackgroundAssetsInput, CgAssetsInput, CharacterAssetsInput, GameAssets, GameAssetsInput};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_KEY_CHARS: usize = 64;
const MAX_NAME_CHARS: usize = 100;
const MAX_TEXT_CHARS: usize = 2000; // 설명/대사 텍스트
const SNIFF_BYTES: usize = 8192;

/// 추가할 에셋의 정보. 카테고리별로 필요한 필드만 쓰고 나머지는 무시한다.
///   voice/sfx/bgm: tag(sfx는 소리 그룹 이름 필수, bgm은 분위기 필수), gameCharacterId(voice의 화자), transcript
///   character: gameCharacterId(필수), usage(standing|map, 필수), emotion(standing이면 필수)
///   cg: description, aiUsable(기본 true), gameCharacterId
///   background: locationId/timeId/weatherId (비우면 그 조건은 전체)
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewGameAsset {
    pub category: String,
    /// 게임 안에서 유일한 키. 비우면 원본 파일 이름으로 만든다(겹치면 _2, _3...).
    pub key: Option<String>,
    pub name: Option<String>,
    pub settings_json: Option<serde_json::Value>,
    pub tag: Option<String>,
    pub game_character_id: Option<String>,
    pub transcript: Option<String>,
    pub usage: Option<String>,
    pub emotion: Option<String>,
    pub description: Option<String>,
    pub ai_usable: Option<bool>,
    pub location_id: Option<String>,
    pub time_id: Option<String>,
    pub weather_id: Option<String>,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AddAssetReport {
    /// 만들어진 에셋 기본 행 (path는 게임 폴더 기준 상대경로: `assets/<category>/<uuid>.<ext>`)
    pub asset: GameAssets,
    /// 추가 후 상태의 진단 결과 (open_game/save_package와 같은 형식)
    pub diagnostics: Vec<Issue>,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAssetReport {
    pub deleted: bool,
    /// 지우기 전에 이 에셋을 쓰고 있던 곳 (force로 지웠을 때만 비어 있지 않다)
    pub references: Vec<String>,
    pub diagnostics: Vec<Issue>,
}

/// 카테고리가 받아들이는 파일 종류. 이 커맨드는 이미지/오디오만 받는다 (font/other는 패키지로만 들어온다).
pub fn category_kind(category: &str) -> Option<&'static str> {
    match category {
        "voice" | "sfx" | "bgm" => Some("audio"),
        "character" | "cg" | "background" => Some("image"),
        _ => None,
    }
}

/// 파일 내용으로 판정한 형식.
#[derive(Debug, PartialEq, Eq)]
pub struct Sniffed {
    /// 저장할 확장자 (판정 결과에서 정한다)
    pub ext: &'static str,
    pub kind: &'static str,
    pub size: u64,
}

/// 크기 상한 확인 (0바이트도 거절).
fn check_size(size: u64, max: u64) -> Result<(), AppError> {
    if size == 0 {
        return Err(AppError::validation("빈 파일입니다", None));
    }
    if size > max {
        return Err(AppError::validation(
            format!("파일이 너무 큽니다 (최대 {}MB)", max / (1024 * 1024)),
            Some(serde_json::json!({ "size": size, "max": max })),
        ));
    }
    Ok(())
}

/// 파일 앞부분으로 형식을 판정하고, 카테고리가 받는 종류인지와 크기 상한을 확인한다.
pub fn sniff(path: &Path, expected_kind: &str) -> Result<Sniffed, AppError> {
    let meta = std::fs::metadata(path).map_err(|_| AppError::not_found("파일을 찾을 수 없습니다"))?;
    if !meta.is_file() {
        return Err(AppError::validation("파일이 아닙니다", None));
    }
    let mut head = Vec::with_capacity(SNIFF_BYTES);
    std::fs::File::open(path)?.take(SNIFF_BYTES as u64).read_to_end(&mut head)?;
    let detected = infer::get(&head).ok_or_else(|| {
        AppError::validation(
            "알 수 없는 파일 형식입니다 (이미지: png/jpg/webp/gif/avif, 오디오: mp3/ogg/opus/wav/m4a/aac/flac)",
            None,
        )
    })?;
    let ext = detected.extension();
    // 서빙 허용 목록과 같은 기준: 목록 밖이면 넣어도 재생/표시되지 않는다.
    let (kind, max) = crate::game::protocol::media_limits(ext)
        .ok_or_else(|| AppError::validation(format!("지원하지 않는 파일 형식입니다 ({})", detected.mime_type()), None))?;
    // mime_guess로 한 번 더: 확장자가 가리키는 최상위 타입도 기대한 종류여야 한다.
    let guessed = mime_guess::from_ext(ext).first().map(|m| m.type_().as_str().to_string());
    if kind != expected_kind || guessed.as_deref() != Some(expected_kind) {
        return Err(AppError::validation(
            format!("이 카테고리에는 {expected_kind} 파일만 넣을 수 있습니다 (받은 파일: {})", detected.mime_type()),
            None,
        ));
    }
    check_size(meta.len(), max)?;
    Ok(Sniffed { ext, kind, size: meta.len() })
}

/// validate.rs의 키 규칙과 같다: 영문 소문자, 숫자, . _ - 만.
pub fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().count() <= MAX_KEY_CHARS
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

/// 파일 이름에서 키를 만든다 (규칙에 안 맞는 문자는 _). 이미 있는 키면 _2, _3...을 붙인다.
pub fn derive_key(stem: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let mut base: String = stem
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    base = base.trim_matches('_').chars().take(MAX_KEY_CHARS - 4).collect();
    if base.is_empty() {
        base = "asset".to_string();
    }
    if !taken(&base) {
        return base;
    }
    (2..10_000)
        .map(|n| format!("{base}_{n}"))
        .find(|k| !taken(k))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn trimmed(v: &Option<String>) -> Option<String> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn too_long(v: &Option<String>, max: usize) -> bool {
    v.as_deref().map_or(false, |s| s.chars().count() > max)
}

/// 카테고리별 필수값과 참조(캐릭터/위치/시간대/날씨)를 정의에 비춰 확인한다. 키는 따로 정한다.
pub fn check_new(def: &GameDefinition, a: &NewGameAsset) -> Result<(), AppError> {
    use crate::game::stage::BGM_MOODS;
    category_kind(&a.category).ok_or_else(|| {
        AppError::validation(
            format!("에셋 카테고리가 올바르지 않습니다: {} (voice, sfx, bgm, character, cg, background)", a.category),
            None,
        )
    })?;
    if too_long(&a.name, MAX_NAME_CHARS) {
        return Err(AppError::validation("이름이 너무 깁니다", None));
    }
    if too_long(&a.transcript, MAX_TEXT_CHARS) || too_long(&a.description, MAX_TEXT_CHARS) {
        return Err(AppError::validation("설명/대사가 너무 깁니다", None));
    }
    let bad_chars = |s: &str| s.contains(['<', '>', '"', ',', '\n']);
    let character = trimmed(&a.game_character_id);
    if let Some(c) = &character {
        if def.character(c).is_none() {
            return Err(AppError::validation("존재하지 않는 캐릭터입니다", None));
        }
    }
    match a.category.as_str() {
        "sfx" => match trimmed(&a.tag) {
            None => return Err(AppError::validation("효과음에는 tag(소리 그룹 이름)가 필요합니다", None)),
            Some(t) if bad_chars(&t) || t.chars().count() > 40 => {
                return Err(AppError::validation("효과음 tag에 < > \" , 줄바꿈은 쓸 수 없고 40자까지입니다", None))
            }
            _ => {}
        },
        "bgm" => match trimmed(&a.tag) {
            Some(t) if BGM_MOODS.contains(&t.as_str()) => {}
            _ => {
                return Err(AppError::validation(
                    format!("배경음 tag(분위기)가 올바르지 않습니다 (가능: {})", BGM_MOODS.join(", ")),
                    None,
                ))
            }
        },
        "character" => {
            if character.is_none() {
                return Err(AppError::validation("캐릭터 이미지에는 캐릭터(gameCharacterId)가 필요합니다", None));
            }
            match a.usage.as_deref() {
                Some("standing") => match trimmed(&a.emotion) {
                    None => return Err(AppError::validation("스탠딩에는 감정명(emotion)이 필요합니다", None)),
                    Some(e) if bad_chars(&e) || e.contains(':') || e.chars().count() > 40 => {
                        return Err(AppError::validation("감정명에 < > : \" , 줄바꿈은 쓸 수 없고 40자까지입니다", None))
                    }
                    _ => {}
                },
                Some("map") => {}
                _ => return Err(AppError::validation("캐릭터 이미지의 usage는 standing 또는 map이어야 합니다", None)),
            }
        }
        "background" => {
            if let Some(l) = trimmed(&a.location_id) {
                if def.location(&l).is_none() {
                    return Err(AppError::validation("존재하지 않는 위치입니다", None));
                }
            }
            if let Some(t) = trimmed(&a.time_id) {
                if !def.time_periods.iter().any(|p| p.id == t) {
                    return Err(AppError::validation("존재하지 않는 시간대입니다", None));
                }
            }
            if let Some(w) = trimmed(&a.weather_id) {
                if !def.weather_types.iter().any(|x| x.id == w) {
                    return Err(AppError::validation("존재하지 않는 날씨입니다", None));
                }
            }
        }
        _ => {}
    }
    if let Some(s) = &a.settings_json {
        if !s.is_object() && !s.is_null() {
            return Err(AppError::validation("settingsJson은 객체여야 합니다", None));
        }
        if serde_json::to_string(s).map_or(0, |t| t.len()) > 16 * 1024 {
            return Err(AppError::validation("settingsJson이 너무 큽니다", None));
        }
    }
    Ok(())
}

/// 행이 지워지면 문제가 생기는 곳. 키로 가리키는 연출 태그(cg/voice)와, 그룹의 마지막 하나일 때만 깨지는 것
/// (sfx 그룹, bgm 분위기, 캐릭터+감정 스탠딩), 그리고 파일 경로를 그대로 쓰는 표지/아바타/아이콘.
pub fn asset_references(def: &GameDefinition, asset_id: &str) -> Vec<String> {
    use crate::game::stage::{spec_to_script, tag_uses};
    let Some(base) = def.assets.all.iter().find(|a| a.id == asset_id) else {
        return vec![];
    };

    // 대본 문법으로 된 곳을 모두 모은다: 씬 대본, 이벤트/선택지/아이템 효과의 stage 동사.
    let mut sources: Vec<(String, String)> = Vec::new();
    for s in &def.scenes {
        if let Some(script) = s.script.as_deref() {
            sources.push((format!("씬 '{}' 대본", s.scene_name), script.to_string()));
        }
        for c in s.choices.iter().flatten() {
            if let Some(spec) = c
                .effect
                .as_deref()
                .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
                .and_then(|v| v.get("stage").cloned())
            {
                sources.push((format!("씬 '{}' › 선택지 '{}'", s.scene_name, c.text), spec_to_script(&spec)));
            }
        }
    }
    let mut events: Vec<_> = def.events.values().collect();
    events.sort_by(|a, b| a.name.cmp(&b.name));
    for e in events {
        if let Some(spec) = e.effect_json.get("stage") {
            sources.push((format!("이벤트 '{}'", e.name), spec_to_script(spec)));
        }
    }
    let mut items: Vec<_> = def.items.values().collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    for i in items {
        if let Some(spec) = i.use_effect.get("stage") {
            sources.push((format!("아이템 '{}' (사용 효과)", i.name), spec_to_script(spec)));
        }
    }

    // 이 에셋이 사라지면 풀리지 않게 되는 태그인가.
    let char_matches = |target: &Option<String>, character_id: &str| {
        let Some(t) = target.as_deref().map(|t| t.trim().to_lowercase()) else {
            return false;
        };
        def.character(character_id)
            .map_or(false, |c| c.name.trim().to_lowercase() == t || c.id.to_lowercase() == t)
    };
    let breaks: Box<dyn Fn(&str, &Option<String>, &str) -> bool + '_> = match base.category.as_str() {
        "cg" => Box::new(|name, _, value| name == "cg" && value == base.key),
        "voice" => Box::new(|name, _, value| name == "voice" && value == base.key),
        "sfx" | "bgm" => {
            let mine = def.assets.audio.iter().find(|a| a.base.id == asset_id);
            let tag = mine.and_then(|a| a.tag.clone()).unwrap_or_default();
            let last = !def
                .assets
                .audio
                .iter()
                .any(|a| a.base.id != asset_id && a.base.category == base.category && a.tag.as_deref() == Some(tag.as_str()));
            let category = base.category.clone();
            Box::new(move |name, _, value| last && name == category && value == tag)
        }
        "character" => {
            let mine = def.assets.characters.iter().find(|a| a.base.id == asset_id);
            match mine
                .filter(|a| a.usage == "standing")
                .and_then(|a| Some((a.game_character_id.clone()?, a.emotion.clone()?)))
            {
                Some((cid, emotion)) => {
                    let e = emotion.trim().to_lowercase();
                    let last = !def.assets.characters.iter().any(|a| {
                        a.base.id != asset_id
                            && a.usage == "standing"
                            && a.game_character_id.as_deref() == Some(cid.as_str())
                            && a.emotion.as_deref().map(|x| x.trim().to_lowercase()).as_deref() == Some(e.as_str())
                    });
                    Box::new(move |name, target, value| last && name == "emotion" && value.trim().to_lowercase() == e && char_matches(target, &cid))
                }
                None => Box::new(|_, _, _| false),
            }
        }
        _ => Box::new(|_, _, _| false),
    };

    let mut refs: Vec<String> = Vec::new();
    for (who, text) in &sources {
        if tag_uses(text).iter().any(|(name, target, value)| breaks(name, target, value)) && !refs.contains(who) {
            refs.push(who.clone());
        }
    }
    // 경로를 그대로 쓰는 곳
    let path = base.file_path.trim_start_matches('/');
    let same = |p: &Option<String>| p.as_deref().map(|x| x.trim_start_matches('/')) == Some(path);
    if same(&def.project.cover_image) {
        refs.push("게임 표지 이미지".to_string());
    }
    let mut chars: Vec<_> = def
        .characters
        .values()
        .filter(|c| same(&c.avatar))
        .map(|c| format!("캐릭터 '{}' 아바타", c.name))
        .collect();
    chars.sort();
    refs.extend(chars);
    let mut icons: Vec<_> = def
        .items
        .values()
        .filter(|i| same(&i.icon))
        .map(|i| format!("아이템 '{}' 아이콘", i.name))
        .collect();
    icons.sort();
    refs.extend(icons);
    refs
}

async fn load(pool: &SqlitePool, game_id: &str) -> Result<GameDefinition, AppError> {
    SqlxGameRepository::new(pool)
        .load_definition(game_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))
}

/// 복사할 폴더가 실제로 게임 폴더 안인지 (심볼릭 링크로 밖을 가리키면 거절).
fn safe_target_dir(game_dir: &Path, rel_dir: &str) -> Result<PathBuf, AppError> {
    let base = game_dir.canonicalize().map_err(|_| AppError::not_found("게임 폴더를 찾을 수 없습니다"))?;
    let dir = game_dir.join(rel_dir);
    std::fs::create_dir_all(&dir)?;
    let real = dir.canonicalize()?;
    if !real.starts_with(&base) {
        return Err(AppError::archive(ErrorCode::ArchiveUnsafePath, "에셋 폴더가 게임 폴더 밖을 가리킵니다"));
    }
    Ok(real)
}

/// 파일을 게임 폴더로 복사하고 에셋 행을 만든다. source는 호스트 UI가 파일 대화상자로 고른 경로다.
pub async fn add_asset(pool: &SqlitePool, games_root: &Path, game_id: &str, source: &Path, new: NewGameAsset) -> Result<AddAssetReport, AppError> {
    crate::db::game::get_game_project(pool, game_id).await?; // 없는 게임이면 NotFound
    let game_dir = games_root.join(game_id);
    if !game_dir.is_dir() {
        return Err(AppError::not_found("게임 폴더를 찾을 수 없습니다"));
    }
    let def = load(pool, game_id).await?;
    check_new(&def, &new)?;
    let expected = category_kind(&new.category).unwrap_or("image");
    let sniffed = sniff(source, expected)?;

    let taken = |k: &str| def.assets.all.iter().any(|a| a.key == k);
    let key = match trimmed(&new.key) {
        Some(k) => {
            if !is_valid_key(&k) {
                return Err(AppError::validation("에셋 키는 영문 소문자, 숫자, . _ - 만 쓸 수 있고 64자까지입니다", None));
            }
            if taken(&k) {
                return Err(AppError::validation(format!("이미 있는 에셋 키입니다: {k}"), None));
            }
            k
        }
        None => derive_key(source.file_stem().and_then(|s| s.to_str()).unwrap_or("asset"), &taken),
    };

    let rel_dir = format!("assets/{}", new.category);
    let file_name = format!("{}.{}", uuid::Uuid::new_v4(), sniffed.ext);
    let rel_path = format!("{rel_dir}/{file_name}");
    crate::game::package::check_asset_path(&rel_path).map_err(|_| AppError::archive(ErrorCode::ArchiveUnsafePath, "에셋 경로가 올바르지 않습니다"))?;
    let dest = safe_target_dir(&game_dir, &rel_dir)?.join(&file_name);

    std::fs::copy(source, &dest)?;
    // 복사한 파일을 다시 판정한다: 검사와 복사 사이에 원본이 바뀌었으면(크기/형식) 받지 않는다.
    match sniff(&dest, expected) {
        Ok(again) if again.ext == sniffed.ext => {}
        Ok(_) | Err(_) => {
            let _ = std::fs::remove_file(&dest);
            return Err(AppError::validation("파일을 복사하는 동안 내용이 바뀌었습니다. 다시 시도하세요", None));
        }
    }

    let base = GameAssetsInput {
        id: None,
        game_id: game_id.to_string(),
        key: key.clone(),
        name: trimmed(&new.name).unwrap_or_else(|| key.clone()),
        category: new.category.clone(),
        kind: sniffed.kind.to_string(),
        file_path: rel_path.clone(),
        settings_json: new.settings_json.clone().filter(|v| !v.is_null()),
    };
    let written: Result<String, AppError> = async {
        let mut tx = pool.begin().await?;
        let id = upsert_game_asset(&mut *tx, base.clone(), game_id).await?;
        match new.category.as_str() {
            "voice" | "sfx" | "bgm" => {
                let character = if new.category == "voice" { trimmed(&new.game_character_id) } else { None };
                upsert_audio_asset(
                    &mut *tx,
                    AudioAssetsInput {
                        base: base.clone(),
                        tag: trimmed(&new.tag),
                        game_character_id: character,
                        transcript: trimmed(&new.transcript),
                    },
                    &id,
                )
                .await?;
            }
            "character" => {
                let usage = new.usage.clone().unwrap_or_default();
                let emotion = trimmed(&new.emotion); // standing은 필수(check_new), map은 선택(변형 이름)
                upsert_character_asset(
                    &mut *tx,
                    CharacterAssetsInput {
                        base: base.clone(),
                        game_character_id: trimmed(&new.game_character_id),
                        usage,
                        emotion,
                    },
                    &id,
                )
                .await?;
            }
            "cg" => {
                upsert_cg_asset(
                    &mut *tx,
                    CgAssetsInput {
                        base: base.clone(),
                        game_character_id: trimmed(&new.game_character_id),
                        description: trimmed(&new.description),
                        prompt: None,
                        negative_prompt: None,
                        seed: None,
                        generation_params: None,
                        ai_usable: new.ai_usable.unwrap_or(true),
                    },
                    &id,
                )
                .await?;
            }
            "background" => {
                upsert_background_assets(
                    &mut *tx,
                    BackgroundAssetsInput {
                        base: base.clone(),
                        location_id: trimmed(&new.location_id),
                        time_id: trimmed(&new.time_id),
                        weather_id: trimmed(&new.weather_id),
                    },
                    &id,
                )
                .await?;
            }
            _ => {}
        }
        tx.commit().await?;
        Ok(id)
    }
    .await;
    let id = match written {
        Ok(id) => id,
        Err(e) => {
            let _ = std::fs::remove_file(&dest); // DB가 실패하면 복사한 파일도 남기지 않는다
            return Err(e);
        }
    };

    let after = load(pool, game_id).await?;
    let asset = after
        .assets
        .all
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| AppError::internal("추가한 에셋을 다시 읽지 못했습니다"))?;
    Ok(AddAssetReport {
        asset,
        diagnostics: validate::diagnose(&after, Some(game_dir.as_path())),
    })
}

/// 에셋을 지운다. 다른 곳이 쓰고 있으면 force가 아닌 한 거절한다(details.references에 쓰는 곳).
pub async fn remove_asset(pool: &SqlitePool, games_root: &Path, game_id: &str, asset_id: &str, force: bool) -> Result<RemoveAssetReport, AppError> {
    let def = load(pool, game_id).await?;
    let asset = def
        .assets
        .all
        .iter()
        .find(|a| a.id == asset_id)
        .ok_or_else(|| AppError::not_found("이 게임의 에셋이 아닙니다"))?;
    let references = asset_references(&def, asset_id);
    if !references.is_empty() && !force {
        return Err(AppError::Validation {
            code: ErrorCode::InvalidInput,
            message: format!("다른 곳에서 쓰는 에셋이라 지울 수 없습니다: [{}]", references.join(", ")),
            details: Some(serde_json::json!({ "reason": "assetInUse", "references": references })),
        });
    }
    let file_path = asset.file_path.clone();

    let mut tx = pool.begin().await?;
    let deleted = delete_game_asset(&mut *tx, asset_id).await?; // 종류별 하위 행은 CASCADE
    tx.commit().await?;
    // DB가 커밋된 뒤에 파일을 지운다. 같은 파일을 가리키는 다른 에셋 행이 있으면 파일은 남긴다.
    if deleted && !def.assets.all.iter().any(|a| a.id != asset_id && a.file_path == file_path) {
        crate::game::package::remove_asset_files(games_root, game_id, &[file_path]);
    }

    let after = load(pool, game_id).await?;
    let game_dir = games_root.join(game_id);
    Ok(RemoveAssetReport {
        deleted,
        references,
        diagnostics: validate::diagnose(&after, Some(game_dir.as_path())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::test_support;

    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0,
    ];
    const MP3: &[u8] = &[b'I', b'D', b'3', 3, 0, 0, 0, 0, 0, 0, 0xFF, 0xFB, 0x90, 0x00];

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn sniff_uses_content_not_extension() {
        let dir = tempfile::tempdir().unwrap();
        // 이름은 mp3지만 내용은 png
        let fake = write(dir.path(), "song.mp3", PNG);
        assert!(sniff(&fake, "audio").is_err(), "내용이 이미지면 오디오 카테고리에 못 넣는다");
        let s = sniff(&fake, "image").unwrap();
        assert_eq!((s.ext, s.kind), ("png", "image"), "저장 확장자는 판정 결과에서 정한다");

        let mp3 = write(dir.path(), "a.bin", MP3);
        assert_eq!(sniff(&mp3, "audio").unwrap().ext, "mp3");

        let text = write(dir.path(), "x.png", b"<svg xmlns='http://www.w3.org/2000/svg'></svg>");
        assert!(sniff(&text, "image").is_err(), "SVG/텍스트는 거절");
        let bmp = write(dir.path(), "x.bmp", &[b'B', b'M', 0, 0, 0, 0, 0, 0, 0, 0, 0x36, 0, 0, 0]);
        assert!(sniff(&bmp, "image").is_err(), "서빙 허용 목록 밖의 형식은 거절");
        assert!(sniff(&dir.path().join("none.png"), "image").is_err());
    }

    #[test]
    fn size_limits() {
        assert!(check_size(0, 10).is_err());
        assert!(check_size(10, 10).is_ok());
        assert!(check_size(11, 10).is_err());
        let (_, image_max) = crate::game::protocol::media_limits("png").unwrap();
        let (_, audio_max) = crate::game::protocol::media_limits("mp3").unwrap();
        assert!(image_max < audio_max);
    }

    #[test]
    fn keys() {
        assert!(is_valid_key("bg.school_01-a"));
        assert!(!is_valid_key("Bg") && !is_valid_key("") && !is_valid_key("a b") && !is_valid_key("../x"));
        let taken = |k: &str| k == "my_song" || k == "my_song_2";
        assert_eq!(derive_key("My Song!", &taken), "my_song_3");
        assert_eq!(derive_key("유나", &|_| false), "asset");
        assert!(is_valid_key(&derive_key("Hello World (1)", &|_| false)));
    }

    #[tokio::test]
    async fn add_then_remove_with_reference_check() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let (pool, game_id) = test_support::dating_sim(root.path()).await;

        // 잘못된 카테고리/필수값/형식은 파일을 남기지 않고 거절된다
        let count = |dir: &str| std::fs::read_dir(root.path().join(&game_id).join(dir)).map(|d| d.count()).unwrap_or(0);
        let before = (count("assets/bgm"), count("assets/character"));
        let png = write(src.path(), "Festival Night.png", PNG);
        let bad = NewGameAsset {
            category: "bgm".into(),
            tag: Some("romance".into()),
            ..Default::default()
        };
        assert!(
            add_asset(&pool, root.path(), &game_id, &png, bad).await.is_err(),
            "이미지를 bgm으로 넣을 수 없다"
        );
        let bad = NewGameAsset {
            category: "character".into(),
            usage: Some("standing".into()),
            ..Default::default()
        };
        assert!(add_asset(&pool, root.path(), &game_id, &png, bad).await.is_err(), "캐릭터 없는 스탠딩");
        assert_eq!((count("assets/bgm"), count("assets/character")), before, "거절되면 파일이 남지 않는다");

        let cg = NewGameAsset {
            category: "cg".into(),
            description: Some("축제 밤".into()),
            ..Default::default()
        };
        let report = add_asset(&pool, root.path(), &game_id, &png, cg).await.unwrap();
        let asset = report.asset;
        assert_eq!(asset.key, "festival_night");
        assert_eq!(asset.kind, "image");
        assert!(asset.file_path.starts_with("assets/cg/") && asset.file_path.ends_with(".png"));
        assert!(root.path().join(&game_id).join(&asset.file_path).is_file());
        let def = test_support::definition(&pool, &game_id).await;
        assert!(def
            .assets
            .cgs
            .iter()
            .any(|c| c.base.id == asset.id && c.ai_usable && c.description.as_deref() == Some("축제 밤")));

        // 같은 키는 거절
        let dup = NewGameAsset {
            category: "cg".into(),
            key: Some("festival_night".into()),
            ..Default::default()
        };
        assert!(add_asset(&pool, root.path(), &game_id, &png, dup).await.is_err());

        // 씬 대본에서 쓰면 그냥은 못 지운다
        let scene_id = def.scenes.first().expect("템플릿에 씬이 있다").id.clone();
        sqlx::query("UPDATE game_scenes SET script = ? WHERE id = ?")
            .bind("<cg>festival_night</cg>불꽃이 터진다.")
            .bind(&scene_id)
            .execute(&pool)
            .await
            .unwrap();
        let err = remove_asset(&pool, root.path(), &game_id, &asset.id, false)
            .await
            .err()
            .expect("참조 중이면 거절");
        match err {
            AppError::Validation { details: Some(d), .. } => assert_eq!(d["reason"], "assetInUse"),
            other => panic!("예상과 다른 오류: {other:?}"),
        }
        assert!(root.path().join(&game_id).join(&asset.file_path).is_file(), "거절되면 파일도 그대로");

        // force면 지우고, 깨진 참조는 진단으로 알린다
        let removed = remove_asset(&pool, root.path(), &game_id, &asset.id, true).await.unwrap();
        assert!(removed.deleted);
        assert_eq!(removed.references.len(), 1);
        assert!(!root.path().join(&game_id).join(&asset.file_path).exists());
        assert!(removed.diagnostics.iter().any(|i| i.message.contains("씬 대본에 쓸 수 없는 연출 태그")));
        assert!(remove_asset(&pool, root.path(), &game_id, &asset.id, false).await.is_err(), "이미 지운 에셋");
    }

    #[tokio::test]
    async fn audio_group_reference_only_breaks_on_last_file() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let (pool, game_id) = test_support::dating_sim(root.path()).await;
        let mp3 = write(src.path(), "door.mp3", MP3);
        let sfx = || NewGameAsset {
            category: "sfx".into(),
            tag: Some("문 소리 테스트".into()),
            ..Default::default()
        };
        let a = add_asset(&pool, root.path(), &game_id, &mp3, sfx()).await.unwrap().asset;
        let b = add_asset(&pool, root.path(), &game_id, &mp3, sfx()).await.unwrap().asset;
        assert_ne!(a.key, b.key);
        let def = test_support::definition(&pool, &game_id).await;
        let scene_id = def.scenes.first().unwrap().id.clone();
        sqlx::query("UPDATE game_scenes SET script = ? WHERE id = ?")
            .bind("<sfx>문 소리 테스트</sfx>똑똑.")
            .bind(&scene_id)
            .execute(&pool)
            .await
            .unwrap();
        let def = test_support::definition(&pool, &game_id).await;
        assert!(asset_references(&def, &a.id).is_empty(), "같은 그룹에 다른 파일이 있으면 참조가 깨지지 않는다");
        remove_asset(&pool, root.path(), &game_id, &a.id, false).await.unwrap();
        let def = test_support::definition(&pool, &game_id).await;
        assert_eq!(asset_references(&def, &b.id).len(), 1, "그룹의 마지막 파일은 참조 중");
    }
}
