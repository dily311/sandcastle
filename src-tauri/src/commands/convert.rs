//! 캐릭터 ↔ 페르소나, 게임 캐릭터 ↔ 캐릭터 복사.
//! 원본은 그대로 두고 상대 쪽에 새로 만든다. 대화 기록은 옮기지 않는다.
use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    commands::{character_image::app_data_dir, conversation::copy_avatar},
    db::{conversation, game},
    error::{AppError, Result},
    models::{
        conversation::{Character, CharacterInput, Persona, PersonaInput},
        game::{GameCharacters, GameCharactersInput},
    },
    state::AppState,
};

/// 아바타 파일이 실제로 있으면 새 파일로 복사 (원본과 따로 지울 수 있게). 없거나 경로가 아니면 버림
fn copy_avatar_file(app: &AppHandle, avatar: Option<&str>, prefix: &str) -> Option<String> {
    let src = avatar?;
    if !Path::new(src).is_file() {
        return None;
    }
    let dir = app_data_dir(app).ok()?.join("characters").join("avatars");
    copy_avatar(&dir, src, prefix).ok()
}

fn none_if_blank(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

#[tauri::command]
#[specta::specta]
pub async fn copy_character_to_persona(app: AppHandle, state: State<'_, AppState>, character_id: String) -> Result<Persona> {
    let c = conversation::get_character(&state.db, &character_id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {character_id}")))?;
    let input = PersonaInput {
        name: c.name,
        avatar: copy_avatar_file(&app, c.avatar.as_deref(), "persona"),
        description: c.description,
        world_info_ids: c.world_info_ids.clone(),
        tags: c.tags,
        color: c.color.unwrap_or_default(),
        is_system_default: false,
    };
    let mut tx = state.db.begin().await?;
    let p = conversation::create_persona(&mut *tx, input).await?;
    conversation::replace_persona_worldinfo(&mut tx, &p.id, &c.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_persona(&state.db, &p.id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_persona_to_character(app: AppHandle, state: State<'_, AppState>, persona_id: String) -> Result<Character> {
    let p = conversation::get_persona(&state.db, &persona_id)
        .await
        .map_err(|_| AppError::not_found(format!("페르소나를 찾을 수 없습니다: {persona_id}")))?;
    let input = CharacterInput {
        name: p.name,
        avatar: copy_avatar_file(&app, p.avatar.as_deref(), "character"),
        description: p.description,
        alternate_descriptions: vec![],
        first_message: None,
        alternate_first_message: vec![],
        example_dialogue: None,
        system_prompt_override: None,
        creator_notes: None,
        world_info_ids: p.world_info_ids.clone(),
        tags: p.tags,
        color: none_if_blank(Some(p.color)),
        category: None,
        default_persona_id: None,
        default_prompt_preset_id: None,
        favorite: false,
    };
    let mut tx = state.db.begin().await?;
    let c = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &c.id, &p.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_character(&state.db, &c.id).await?)
}

/// 캐릭터를 게임에 넣음 (게임 캐릭터는 원본 캐릭터를 기억함). 아바타는 게임 에셋으로 따로 넣어야 함
#[tauri::command]
#[specta::specta]
pub async fn copy_character_to_game(state: State<'_, AppState>, character_id: String, game_id: String) -> Result<GameCharacters> {
    let c = conversation::get_character(&state.db, &character_id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {character_id}")))?;
    game::get_game_project(&state.db, &game_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임을 찾을 수 없습니다: {game_id}")))?;
    let id = uuid::Uuid::new_v4().to_string();
    let input = GameCharactersInput {
        id: Some(id.clone()),
        game_id: game_id.clone(),
        character_id: Some(c.id),
        name: c.name,
        avatar: None,
        description: Some(c.description).filter(|s| !s.trim().is_empty()),
        first_message: c.first_message,
        example_dialogue: c.example_dialogue,
        system_prompt_override: c.system_prompt_override,
        behavior_type: None,
        world_info_ids: vec![],
        tags: Some(serde_json::to_string(&c.tags).unwrap_or_else(|_| "[]".into())),
        settings_json: serde_json::json!({}),
    };
    game::upsert_game_character(&state.db, &game_id, input).await?;
    Ok(game::get_game_character(&state.db, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_game_character_to_character(app: AppHandle, state: State<'_, AppState>, game_character_id: String) -> Result<Character> {
    let g = game::get_game_character(&state.db, &game_character_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임 캐릭터를 찾을 수 없습니다: {game_character_id}")))?;
    let input = CharacterInput {
        name: g.name,
        avatar: copy_avatar_file(&app, g.avatar.as_deref(), "character"),
        description: g.description.unwrap_or_default(),
        alternate_descriptions: vec![],
        first_message: none_if_blank(g.first_message),
        alternate_first_message: vec![],
        example_dialogue: none_if_blank(g.example_dialogue),
        system_prompt_override: none_if_blank(g.system_prompt_override),
        creator_notes: None,
        world_info_ids: g.world_info_ids.clone(),
        tags: g.tags.unwrap_or_default(),
        color: None,
        category: None,
        default_persona_id: None,
        default_prompt_preset_id: None,
        favorite: false,
    };
    let mut tx = state.db.begin().await?;
    let c = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &c.id, &g.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_character(&state.db, &c.id).await?)
}

// ---------------------------------------------------------------------------
// 게임 → 캐릭터 + 로어북 + 대화 (5-F-3, ADR-023). 원본 게임은 그대로 두는 복사
// ---------------------------------------------------------------------------

/// 복사 결과
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCopy {
    pub character: Character,
    /// 새로 만든 로어북 (NPC·아이템·장소·이벤트)
    pub book_id: String,
    /// 플레이 기록을 옮겼으면 그 대화
    pub conversation: Option<crate::models::conversation::Conversation>,
}

fn join_text(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .flatten()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 게임 데이터 → 분류가 붙은 로어북 항목 (이름이 키, 내용이 빈 것은 뺌)
pub(crate) fn game_entries(
    book_id: &str,
    characters: &[GameCharacters],
    items: &[crate::models::game::GameItems],
    locations: &[crate::models::game::GameLocations],
    events: &[crate::models::game::GameEvents],
) -> Vec<crate::models::conversation::WorldInfoEntryInput> {
    let entry = |name: &str, content: String, category: &str, priority: i64| crate::models::conversation::WorldInfoEntryInput {
        id: None,
        book_id: book_id.to_string(),
        keys: vec![name.trim().to_string()],
        secondary_keys: vec![],
        content,
        enabled: true,
        priority,
        constant: false,
        comment: name.trim().to_string(),
        category: category.to_string(),
    };
    let mut out = vec![];
    for c in characters.iter().filter(|c| !c.name.trim().is_empty()) {
        let examples = c
            .example_dialogue
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|e| format!("[말투 예시]\n{e}"));
        let content = join_text(&[c.description.as_deref(), examples.as_deref()]);
        if !content.is_empty() {
            out.push(entry(&c.name, content, "캐릭터", 100));
        }
    }
    for i in items.iter().filter(|i| !i.name.trim().is_empty()) {
        let content = join_text(&[i.kind.as_deref(), i.description.as_deref()]);
        if !content.is_empty() {
            out.push(entry(&i.name, content, "아이템", 60));
        }
    }
    for l in locations.iter().filter(|l| !l.name.trim().is_empty()) {
        if let Some(d) = l.description.as_deref().filter(|d| !d.trim().is_empty()) {
            out.push(entry(&l.name, d.trim().to_string(), "장소", 80));
        }
    }
    for e in events.iter().filter(|e| !e.name.trim().is_empty() && !e.description.trim().is_empty()) {
        out.push(entry(&e.name, e.description.trim().to_string(), "이벤트", 40));
    }
    out
}

/// 게임을 일반 대화용 "세계" 캐릭터로 복사한다. NPC·아이템·장소·이벤트는 분류를 붙인 새 로어북에, 게임에 연결된 로어북은 그대로 연결.
/// session_id를 주면 그 플레이 기록을 "여러 캐릭터"를 켠 대화로 옮긴다 (연출 태그는 걷음).
/// 변수 정의는 세계 캐릭터에, 그 기록의 변수 값·관계는 대화 변수로 한 번 옮긴다 (5-F-4-6, ADR-029 ⑥ — 계속 나누지는 않음)
#[tauri::command]
#[specta::specta]
pub async fn copy_game_to_character(app: AppHandle, state: State<'_, AppState>, game_id: String, session_id: Option<String>) -> Result<GameCopy> {
    let project = game::get_game_project(&state.db, &game_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임을 찾을 수 없습니다: {game_id}")))?;
    let avatar = copy_avatar_file(&app, project.cover_image.as_deref(), "character");
    let (char_id, book_id, conv_id) = copy_game_core(&state.db, &game_id, session_id.as_deref(), avatar).await?;
    // NPC 스탠딩 → 캐릭터 에셋 (파일 복사, 실패해도 복사 자체는 끝난 것으로)
    let base = app_data_dir(&app)?;
    if let Err(e) = copy_npc_standings(&state.db, &base.join("games").join(&game_id), &base, &game_id, &char_id).await {
        eprintln!("[convert] NPC 스탠딩 복사 실패: {e}");
    }
    let conversation = match conv_id {
        Some(id) => Some(conversation::get_conversation(&state.db, &id).await?),
        None => None,
    };
    Ok(GameCopy {
        character: conversation::get_character(&state.db, &char_id).await?,
        book_id,
        conversation,
    })
}

/// 복사 본체 (앱 핸들 없이 — 시험용). (캐릭터 id, 로어북 id, 대화 id)
pub(crate) async fn copy_game_core(
    db: &crate::db::conn::DbPool,
    game_id: &str,
    session_id: Option<&str>,
    avatar: Option<String>,
) -> Result<(String, String, Option<String>)> {
    let project = game::get_game_project(db, game_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임을 찾을 수 없습니다: {game_id}")))?;
    let characters = game::get_all_game_characters(db, game_id).await?;
    let items = game::get_all_game_items(db, game_id).await?;
    let locations = game::get_all_game_locations(db, game_id).await?;
    let events = game::get_all_game_events(db, game_id).await?;
    // 시작 씬의 설명 → 첫 메시지
    let first_message = match &project.start_scene_key {
        Some(k) => game::get_all_game_scenes(db, game_id)
            .await?
            .into_iter()
            .find(|s| &s.scene_key == k)
            .and_then(|s| none_if_blank(s.description)),
        None => None,
    };
    let messages = match session_id {
        Some(sid) => Some(game::get_session_game_messages(db, sid).await?),
        None => None,
    };
    // 변수 (5-F-4-6): 게임 정의 → 대화 변수 정의, 플레이 기록의 값·관계 → 대화 변수
    let state_defs = game::get_all_game_state_defs(db, game_id).await?;
    let session_state: Option<(Option<String>, Option<String>)> = match session_id {
        Some(sid) => {
            sqlx::query_as("SELECT flags_json, relationships_json FROM game_player_sessions WHERE id = ?")
                .bind(sid)
                .fetch_optional(db)
                .await?
        }
        None => None,
    };
    let (var_defs, var_values) = game_vars(&state_defs, &characters, session_state);

    let mut tx = db.begin().await?;
    let book = conversation::create_worldinfo_book(
        &mut *tx,
        crate::models::conversation::WorldInfoBookInput {
            id: None,
            name: format!("{} 세계", project.title),
            description: Some(format!("게임 '{}'에서 복사한 인물·아이템·장소·이벤트", project.title)),
        },
    )
    .await?;
    for e in game_entries(&book.id, &characters, &items, &locations, &events) {
        conversation::upsert_worldinfo_entry(&mut *tx, e).await?;
    }
    let mut books = vec![book.id.clone()];
    for b in &project.world_info_ids {
        if !books.contains(b) {
            books.push(b.clone());
        }
    }
    let input = CharacterInput {
        name: project.title.clone(),
        avatar,
        description: project.description.clone().unwrap_or_default(),
        alternate_descriptions: vec![],
        first_message,
        alternate_first_message: vec![],
        example_dialogue: None,
        system_prompt_override: None,
        creator_notes: Some(format!(
            "게임 '{}'에서 복사함. 등장인물은 로어북 '{}'의 '캐릭터' 분류",
            project.title, book.name
        )),
        world_info_ids: books.clone(),
        tags: vec![],
        color: None,
        category: None,
        default_persona_id: None,
        default_prompt_preset_id: None,
        favorite: false,
    };
    let c = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &c.id, &books).await?;
    if !var_defs.is_empty() {
        conversation::upsert_character_var_defs(&mut *tx, &c.id, &var_defs).await?;
    }

    let mut conv_id = None;
    if let Some(msgs) = messages {
        use crate::service::import::sillytavern::export::strip_game_tags;
        let persona_id = conversation::get_system_default_persona(&mut *tx).await.unwrap_or_else(|_| "default".into());
        let conv = conversation::create_conversation(
            &mut *tx,
            crate::models::conversation::ConversationInput {
                title: format!("{} 플레이", project.title),
                character_id: c.id.clone(),
                persona_id,
                world_info_ids: vec![],
                mode: "chat".into(),
                meta: Some(serde_json::json!({ "multiChar": true, "importedFrom": "game", "game": { "gameId": game_id, "sessionId": session_id }, "variables": var_values })),
                images_enabled: None,
            },
        )
        .await?;
        let mut seq = 0;
        for m in msgs {
            let content = strip_game_tags(&m.content);
            if content.trim().is_empty() {
                continue;
            }
            seq += 1;
            let role = if m.role == "user" || m.role == "player" {
                "user"
            } else if m.role == "system" {
                "system"
            } else {
                "assistant"
            };
            conversation::insert_message_at(
                &mut *tx,
                crate::models::conversation::MessageInput {
                    conversation_id: conv.id.clone(),
                    seq,
                    role: role.into(),
                    name: m.name.clone(),
                    content,
                    character_id: (role == "assistant").then(|| c.id.clone()),
                    meta: Some(serde_json::json!({ "sender": { "type": role, "name": m.name } })),
                },
                m.created_at,
            )
            .await?;
        }
        conv_id = Some(conv.id);
    }
    tx.commit().await?;
    Ok((c.id, book.id, conv_id))
}

/// 게임 변수 → 대화 변수 (5-F-4-6, ADR-029 ⑥).
/// 정의: 게임 정의(설명·종류·처음 값·AI 읽기/쓰기·범위·한 번 폭)를 그대로 — 게임의 array는 글자로.
/// 값: 플레이 기록의 flags를 글자로. 관계: "누가>누구" 축마다 `관계.이름.축`(플레이어를 향한 것) 또는 `관계.이름>이름.축` 수 변수 —
/// 게임과 같은 범위(-100~100)·AI 한 번 폭으로 정의도 만든다. 정의가 확인에 걸리면(이름 겹침·개수 상한) 정의 없이 값만 옮긴다
fn game_vars(
    defs: &[crate::models::game::GameStateDef],
    characters: &[crate::models::game::GameCharacters],
    session: Option<(Option<String>, Option<String>)>,
) -> (Vec<crate::models::conversation::VarDef>, serde_json::Map<String, serde_json::Value>) {
    use crate::models::conversation::{VarDef, VarKind};
    use serde_json::Value;
    let mut out: Vec<VarDef> = defs
        .iter()
        .map(|d| {
            let rule = crate::game::engine::VarRule::from_config(&d.config_json);
            let kind = match d.def_type.as_str() {
                "number" => VarKind::Number,
                "boolean" => VarKind::Boolean,
                _ => VarKind::String,
            };
            VarDef {
                key: d.key.clone(),
                description: d.description.clone().unwrap_or_default(),
                kind,
                default_value: Some(d.default_value.clone()).filter(|v| !v.trim().is_empty()),
                min: rule.min,
                max: rule.max,
                max_step: rule.max_step,
                ai_read: rule.readable,
                ai_write: rule.writable,
            }
        })
        .collect();
    let mut values = serde_json::Map::new();
    if let Some((flags, rels)) = session {
        let text = |v: &Value| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if let Some(Value::Object(f)) = flags.as_deref().and_then(|s| serde_json::from_str(s).ok()) {
            for (k, v) in f {
                if !v.is_null() {
                    values.insert(k, Value::String(text(&v)));
                }
            }
        }
        let name = |id: &str| -> String {
            if id == "player" {
                return "플레이어".into();
            }
            characters
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.trim().to_string())
                .unwrap_or_else(|| id.to_string())
        };
        if let Some(Value::Object(r)) = rels.as_deref().and_then(|s| serde_json::from_str(s).ok()) {
            for (pair, axes) in r {
                let Some((from, to)) = pair.split_once('>') else { continue };
                let who = if to == "player" { name(from) } else { format!("{}>{}", name(from), name(to)) };
                let Value::Object(axes) = axes else { continue };
                for (axis, v) in axes {
                    if v.as_f64().is_none() {
                        continue;
                    }
                    let key = format!("관계.{who}.{axis}");
                    values.insert(key.clone(), Value::String(text(&v)));
                    out.push(VarDef {
                        key,
                        description: format!("{who}의 {axis} (게임 관계)"),
                        kind: VarKind::Number,
                        default_value: Some("0".into()),
                        min: Some(crate::game::engine::RELATION_MIN as f64),
                        max: Some(crate::game::engine::RELATION_MAX as f64),
                        max_step: Some(crate::game::engine::RELATION_AI_MAX_STEP as f64),
                        ai_read: true,
                        ai_write: to == "player",
                    });
                }
            }
        }
    }
    let defs = match crate::service::chat::vars::normalize(out) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[convert] 게임 변수 정의를 옮기지 못함 (값만 옮김): {}", e.to_response().message);
            vec![]
        }
    };
    (defs, values)
}

/// 게임 NPC의 스탠딩 그림을 캐릭터 에셋으로 (공통 폴더, 키 `NPC이름_감정`). 원본 게임 파일은 그대로. 넣은 수
pub(crate) async fn copy_npc_standings(db: &crate::db::conn::DbPool, game_dir: &Path, app_data_dir: &Path, game_id: &str, character_id: &str) -> Result<usize> {
    let npcs = game::get_all_game_characters(db, game_id).await?;
    let items: Vec<(std::path::PathBuf, String, Option<String>)> = game::get_all_character_assets(db, game_id)
        .await?
        .into_iter()
        .filter(|a| a.usage == "standing")
        .filter_map(|a| {
            let npc = npcs.iter().find(|n| Some(&n.id) == a.game_character_id.as_ref())?;
            let emotion = a.emotion.as_deref().map(str::trim).filter(|e| !e.is_empty()).unwrap_or("default");
            let path = game_dir.join(&a.base.file_path);
            path.is_file().then(|| (path, String::new(), Some(format!("{}_{}", npc.name.trim(), emotion))))
        })
        .collect();
    if items.is_empty() {
        return Ok(0);
    }
    Ok(crate::service::chat::chat_images::add_images_keyed(db, app_data_dir, character_id, &items)
        .await?
        .len())
}

#[cfg(test)]
mod game_copy_tests {
    #[tokio::test]
    async fn game_becomes_a_world_character_with_categorized_lorebook_and_chat() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().to_path_buf()).await.unwrap();
        for sql in [
            "INSERT INTO game_projects (id, title, description, template_id, start_scene_key, created_at, updated_at) VALUES ('g', '숲의 수수께끼', '엘도리아 숲 이야기', 'vn', 'intro', 1, 1)",
            "INSERT INTO game_scenes (id, game_id, scene_name, description, scene_key, created_at, updated_at) VALUES ('sc', 'g', '시작', '숲 입구에서 눈을 뜬다.', 'intro', 1, 1)",
            "INSERT INTO game_characters (id, game_id, name, description, example_dialogue, created_at, updated_at) VALUES ('n1', 'g', '유나', '밝은 견습 기사', '유나: 안녕!', 1, 1)",
            "INSERT INTO game_characters (id, game_id, name, created_at, updated_at) VALUES ('n2', 'g', '빈 사람', 1, 1)",
            "INSERT INTO game_items (id, game_id, name, description, created_at, updated_at) VALUES ('i1', 'g', '은열쇠', '오래된 문을 연다', 1, 1)",
            "INSERT INTO game_events (id, game_id, event_type, trigger_type, repeat_policy, name, description, created_at, updated_at) VALUES ('e1', 'g', 'story', 'manual', 'once', '늑대 습격', '밤에 늑대가 온다', 1, 1)",
            // 변수 (5-F-4-6): 정의 하나, 플레이 기록의 값·관계
            r#"INSERT INTO game_state_defs (id, game_id, description, key, type, default_value, config_json, created_at, updated_at) VALUES ('d1', 'g', '용기', 'courage', 'number', '0', '{"ai_writable": true, "min": 0, "max": 10, "max_step": 2}', 1, 1)"#,
            r#"INSERT INTO game_player_sessions (id, game_id, player_id, head_seq, flags_json, relationships_json) VALUES ('s', 'g', NULL, 0, '{"courage": 4}', '{"n1>player": {"affection": 12}}')"#,
            "INSERT INTO game_conversations (id, game_id, session_id, participants_key, created_at, updated_at) VALUES ('gc', 'g', 's', 'n1', 1, 1)",
            "INSERT INTO game_messages (id, seq, conversation_id, role, name, content, created_at) VALUES ('m1', 1, 'gc', 'user', '나', '안녕', 10), ('m2', 2, 'gc', 'assistant', '유나', '유나: 반가워! <emotion::유나>happy</emotion>', 20), ('m3', 3, 'gc', 'assistant', NULL, '<bgm>calm</bgm>', 30)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&db).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
        }
        let (cid, book, conv) = super::copy_game_core(&db, "g", Some("s"), None).await.unwrap();
        let c = crate::db::conversation::get_character(&db, &cid).await.unwrap();
        assert_eq!(c.name, "숲의 수수께끼");
        assert_eq!(c.first_message.as_deref(), Some("숲 입구에서 눈을 뜬다."), "시작 씬 설명이 첫 메시지");
        assert!(c.world_info_ids.contains(&book));
        let mut entries: Vec<(String, String)> = crate::db::conversation::get_all_worldinfo_entries(&db, &book)
            .await
            .unwrap()
            .into_iter()
            .map(|e| (e.comment, e.category))
            .collect();
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("늑대 습격".into(), "이벤트".into()),
                ("유나".into(), "캐릭터".into()),
                ("은열쇠".into(), "아이템".into())
            ],
            "설명이 빈 인물은 빠짐"
        );
        let conv = conv.expect("플레이 기록을 옮긴 대화");
        let msgs = crate::db::conversation::get_messages(&db, &conv, 100, None).await.unwrap();
        assert_eq!(
            msgs.iter().map(|m| (m.role.as_str(), m.content.as_str())).collect::<Vec<_>>(),
            vec![("user", "안녕"), ("assistant", "유나: 반가워!")]
        );
        let meta = crate::db::conversation::get_conversation(&db, &conv).await.unwrap().meta.unwrap();
        assert_eq!(meta["multiChar"], true);
        // 변수 (5-F-4-6): 값·관계는 대화 변수로, 정의는 세계 캐릭터에 (관계는 게임과 같은 범위·한 번 폭)
        assert_eq!(meta["variables"], serde_json::json!({ "courage": "4", "관계.유나.affection": "12" }));
        let defs = crate::db::conversation::get_character_var_defs(&db, &cid).await.unwrap();
        assert_eq!(
            defs.iter().map(|d| (d.key.as_str(), d.ai_write, d.max, d.max_step)).collect::<Vec<_>>(),
            vec![("courage", true, Some(10.0), Some(2.0)), ("관계.유나.affection", true, Some(100.0), Some(5.0))]
        );
        assert_eq!(defs[0].description, "용기");

        // NPC 스탠딩 → 캐릭터 에셋 (키 `이름_감정`, 공통 폴더)
        let game_dir = dir.path().join("games").join("g");
        std::fs::create_dir_all(game_dir.join("assets/character")).unwrap();
        image::RgbImage::new(4, 4).save(game_dir.join("assets/character/a.webp")).unwrap();
        for sql in [
            "INSERT INTO game_assets (id, game_id, key, name, category, kind, file_path, created_at, updated_at) VALUES ('a1', 'g', 'yuna_happy', '유나 웃음', 'character', 'image', 'assets/character/a.webp', 1, 1), ('a2', 'g', 'yuna_map', '유나 지도', 'character', 'image', 'assets/character/a.webp', 1, 1), ('a3', 'g', 'gone', '없는 파일', 'character', 'image', 'assets/character/none.webp', 1, 1)",
            "INSERT INTO character_assets (assets_id, game_character_id, usage, emotion) VALUES ('a1', 'n1', 'standing', 'happy'), ('a2', 'n1', 'map', NULL), ('a3', 'n1', 'standing', 'sad')",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&db).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
        }
        let n = super::copy_npc_standings(&db, &game_dir, dir.path(), "g", &cid).await.unwrap();
        assert_eq!(n, 1, "지도 그림·파일이 없는 것은 빠짐");
        let imgs = crate::db::character_image::get_character_images(&db, &cid, false).await.unwrap();
        assert_eq!(
            imgs.iter().map(|i| (i.key.as_str(), i.folder.as_str())).collect::<Vec<_>>(),
            vec![("유나_happy", "")]
        );
    }
}
