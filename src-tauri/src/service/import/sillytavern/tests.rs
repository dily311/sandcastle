use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

use super::*;
use crate::models::conversation::Character;
use crate::{
    models::ai_service::{ChatContext, GenerationType, PromptRequest, PromptRole},
    service::prompt::PromptManager,
};

async fn pool() -> DbPool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    pool
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[tokio::test]
async fn imports_png_card_with_book_regex_and_avatar() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let encoded = BASE64_STANDARD.encode(card::tests::v2_card().to_string());
    let path = write(dir.path(), "aria.png", &card::tests::png_with_text("chara", &encoded));

    let SillyTavernImport::Character {
        character,
        world_info_book_id,
        regex_count,
        warnings,
    } = import_file(&db, dir.path(), &path, None).await.unwrap()
    else {
        panic!("캐릭터여야 함")
    };
    assert_eq!(character.name, "Aria");
    assert_eq!(regex_count, 1);
    assert_eq!(warnings.len(), 1);
    let avatar = character.avatar.expect("아바타 저장");
    assert!(std::path::Path::new(&avatar).is_file());

    // 로어북이 캐릭터에 연결되어 있음
    let book_id = world_info_book_id.unwrap();
    let linked = db::conversation::get_character_world_info_ids(&db, &character.id).await.unwrap();
    assert_eq!(linked, vec![book_id.clone()]);
    let entries = db::conversation::get_all_worldinfo_entries(&db, &book_id).await.unwrap();
    assert_eq!(entries[0].content, "Her sword.");

    // 정규식은 캐릭터 범위
    let scripts = db::regex::get_regex_scripts_for(&db, Some(&character.id), None).await.unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].source_format, "sillytavern");
}

#[tokio::test]
async fn imported_preset_drives_prompt_assembly() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let preset_json = json!({
        "temperature": 0.5, "openai_max_tokens": 200, "openai_max_context": 4000, "stream_openai": false,
        "prompts": [
            { "identifier": "main", "name": "Main", "role": "system", "content": "You are {{char}}." },
            { "identifier": "chatHistory", "marker": true },
            { "identifier": "jailbreak", "role": "system", "content": "Reply in Korean." },
            { "identifier": "nudge", "role": "system", "content": "Keep going", "injection_trigger": ["continue"] }
        ],
        "prompt_order": [ { "character_id": 100001, "order": [
            { "identifier": "main", "enabled": true },
            { "identifier": "chatHistory", "enabled": true },
            { "identifier": "nudge", "enabled": true },
            { "identifier": "jailbreak", "enabled": true }
        ]}],
        "extensions": { "regex_scripts": [ { "scriptName": "bad", "findRegex": "/(unclosed/", "placement": [2] } ] }
    });
    let path = write(dir.path(), "My Preset.json", preset_json.to_string().as_bytes());

    let SillyTavernImport::Preset {
        preset_id,
        name,
        block_count,
        regex_count,
        warnings,
    } = import_file(&db, dir.path(), &path, None).await.unwrap()
    else {
        panic!("프리셋이어야 함")
    };
    assert_eq!(name, "My Preset");
    assert_eq!(block_count, 4);
    assert_eq!(regex_count, 0, "컴파일 안 되는 정규식은 건너뜀");
    assert_eq!(warnings.len(), 1);

    let loaded = PromptManager::load_preset(&db, Some(&preset_id)).await.unwrap().unwrap();
    let request = |gen| PromptRequest {
        context: ChatContext {
            user_images: vec![],
            char_name: Some("Aria".into()),
            ..Default::default()
        },
        user_message: "hi".into(),
        preset_id: Some(preset_id.clone()),
        context_limit: 0,
        world_info_ratio: 0.5,
        generation_type: gen,
    };
    let p = PromptManager.build(request(GenerationType::Normal), Some(&loaded)).unwrap();
    let texts: Vec<_> = p.messages.iter().map(|m| (m.role, m.content.as_str())).collect();
    assert_eq!(
        texts,
        vec![
            (PromptRole::System, "You are Aria."),
            (PromptRole::User, "hi"),
            (PromptRole::System, "Reply in Korean."),
        ]
    );
    assert_eq!(p.chat_options.max_tokens, Some(200));
    assert!(!p.chat_options.stream);

    // continue 생성에서만 nudge 블록이 들어감 (triggers가 제대로 저장됐는지 확인)
    let p = PromptManager.build(request(GenerationType::Continue), Some(&loaded)).unwrap();
    assert!(p.messages.iter().any(|m| m.content == "Keep going"));
}

#[tokio::test]
async fn imports_standalone_regex_and_world_info() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();

    let path = write(
        dir.path(),
        "regex.json",
        json!([{ "scriptName": "Asterisks", "findRegex": "/\\*/g", "replaceString": "", "placement": [2] }])
            .to_string()
            .as_bytes(),
    );
    let SillyTavernImport::Regex { scripts, .. } = import_file(&db, dir.path(), &path, None).await.unwrap() else {
        panic!("정규식이어야 함")
    };
    assert_eq!(scripts[0].scope, RegexScope::Global);
    assert_eq!(scripts[0].name, "Asterisks");

    let path = write(
        dir.path(),
        "lore.json",
        json!({ "entries": { "0": { "key": ["moon"], "content": "The moon is red.", "order": 3 } } })
            .to_string()
            .as_bytes(),
    );
    let SillyTavernImport::WorldInfo { book, entry_count, .. } = import_file(&db, dir.path(), &path, Some("달".into())).await.unwrap() else {
        panic!("월드인포여야 함")
    };
    assert_eq!(book.name, "달");
    assert_eq!(entry_count, 1);
}

#[test]
fn detect_kinds() {
    let kind = |v: Value| detect(v.to_string().as_bytes()).map(|(k, _)| k);
    assert_eq!(kind(json!({ "spec": "chara_card_v3", "data": {} })).unwrap(), SillyTavernKind::Character);
    assert_eq!(kind(json!({ "name": "a", "first_mes": "b" })).unwrap(), SillyTavernKind::Character);
    assert_eq!(kind(json!({ "prompts": [] })).unwrap(), SillyTavernKind::Preset);
    assert_eq!(kind(json!({ "findRegex": "/a/" })).unwrap(), SillyTavernKind::Regex);
    assert_eq!(kind(json!({ "entries": {} })).unwrap(), SillyTavernKind::WorldInfo);
    assert_eq!(
        detect(
            b"{\"user_name\":\"u\",\"character_name\":\"c\",\"chat_metadata\":{}}
{\"mes\":\"x\"}"
        )
        .unwrap()
        .0,
        SillyTavernKind::Chat
    );
    assert!(kind(json!({ "hello": 1 })).is_err());
    assert!(detect(b"not json").is_err());
}

// =======================================
// 실제 SillyTavern 배포 파일 (tests/sillyTavern)
// =======================================

fn st_sample(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sillyTavern").join(rel)
}

async fn import_seraphina(db: &DbPool, dir: &Path) -> (Character, String) {
    let SillyTavernImport::Character {
        character,
        world_info_book_id,
        regex_count,
        warnings,
    } = import_file(db, dir, &st_sample("character/default_Seraphina.png"), None).await.unwrap()
    else {
        panic!("캐릭터여야 함")
    };
    assert_eq!(regex_count, 0);
    assert!(warnings.is_empty(), "{warnings:?}");
    (character, world_info_book_id.expect("character_book → 로어북"))
}

#[tokio::test]
async fn st_seraphina_card() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let (character, book_id) = import_seraphina(&db, dir.path()).await;

    assert_eq!(character.name, "Seraphina");
    assert!(character.description.starts_with("[Seraphina's Personality="));
    assert!(character.first_message.as_deref().unwrap().starts_with("*You wake with a start"));
    assert!(std::path::Path::new(character.avatar.as_deref().unwrap()).is_file());

    let entries = db::conversation::get_all_worldinfo_entries(&db, &book_id).await.unwrap();
    assert_eq!(entries.len(), 4);
    assert!(entries.iter().all(|e| e.enabled && !e.keys.is_empty()));
    assert_eq!(db::conversation::get_character_world_info_ids(&db, &character.id).await.unwrap(), vec![book_id]);
}

#[tokio::test]
async fn st_default_preset() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let SillyTavernImport::Preset {
        preset_id,
        name,
        block_count,
        warnings,
        ..
    } = import_file(&db, dir.path(), &st_sample("presets/Default.json"), None).await.unwrap()
    else {
        panic!("프리셋이어야 함")
    };
    assert_eq!(name, "Default");
    assert_eq!(block_count, 12);
    assert_eq!(warnings.len(), 1, "{warnings:?}");

    let preset = db::ai_service::get_prompt_preset(&db, &preset_id).await.unwrap();
    let settings = preset.settings_json.unwrap();
    assert_eq!(settings["max_tokens"], 300);
    assert_eq!(settings["context_size"], 4095);
    let compat = preset.compatibility_json.unwrap();
    assert!(compat.get("proxy_password").is_none(), "비밀번호는 저장하지 않음");
    assert!(compat["prompt_order"].is_array());
}

/// Seraphina + Default 프리셋으로 조립한 프롬프트가 ST와 같은 순서인지
#[tokio::test]
async fn st_seraphina_prompt_matches_sillytavern_order() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let (character, book_id) = import_seraphina(&db, dir.path()).await;
    let SillyTavernImport::Preset { preset_id, .. } = import_file(&db, dir.path(), &st_sample("presets/Default.json"), None).await.unwrap() else {
        panic!()
    };
    let preset = PromptManager::load_preset(&db, Some(&preset_id)).await.unwrap().unwrap();
    let p = PromptManager
        .build(
            PromptRequest {
                context: ChatContext {
                    user_images: vec![],
                    char_name: Some(character.name.clone()),
                    user_name: Some("Min".into()),
                    character_info: Some(character.description.clone()),
                    persona_info: "A lost traveler.".into(),
                    world_info: db::conversation::get_all_worldinfo_entries(&db, &book_id).await.unwrap(),
                    history: vec![crate::models::ai_service::HistoryItem {
                        images: vec![],
                        role: "assistant".into(),
                        content: character.first_message.clone().unwrap(),
                    }],
                    ..Default::default()
                },
                user_message: "Where am I? Is this the forest?".into(),
                preset_id: Some(preset_id),
                context_limit: 0,
                world_info_ratio: 0.5,
                generation_type: GenerationType::Normal,
            },
            Some(&preset),
        )
        .unwrap();

    // main → worldInfoBefore → personaDescription → charDescription → chatHistory (빈 블록은 빠짐)
    let m = &p.messages;
    assert_eq!(m.len(), 6, "{:#?}", m.iter().map(|x| &x.content[..x.content.len().min(40)]).collect::<Vec<_>>());
    assert_eq!(m[0].content, "Write Seraphina's next reply in a fictional chat between Seraphina and Min.");
    assert!(m[1].content.starts_with("Min: \"What is Eldoria?\""), "로어북 매크로 치환 + 'forest' 키워드");
    assert_eq!(m[2].content, "A lost traveler.");
    assert!(m[3].content.starts_with("[Seraphina's Personality="));
    assert_eq!((m[4].role, m[5].role), (PromptRole::Assistant, PromptRole::User));
    assert!(m[..4].iter().all(|x| x.role == PromptRole::System));

    assert_eq!(p.chat_options.max_tokens, Some(300));
    assert_eq!(p.chat_options.temperature, Some(1.0));
    assert_eq!(p.chat_options.top_k, None, "top_k 0 = 사용 안 함");
    assert!(p.total_tokens() < 4095 - 300, "프리셋 컨텍스트(4095) 안에 들어감");
}

/// ST 표정 스프라이트 폴더 → 채팅 이미지 (파일 이름 = 키)
#[tokio::test]
async fn st_expression_sprites_become_chat_images() {
    use crate::service::chat::chat_images::{add_images, image_rules, sanitize_cg_tags};

    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let (character, _) = import_seraphina(&db, dir.path()).await;

    let mut sprites: Vec<_> = std::fs::read_dir(st_sample("character/Seraphina"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    sprites.sort();
    let added = add_images(&db, dir.path(), &character.id, &sprites).await.unwrap();
    assert_eq!(added.len(), 28);
    let keys: Vec<_> = added.iter().map(|i| i.key.as_str()).collect();
    assert!(keys.contains(&"joy") && keys.contains(&"anger") && keys.contains(&"neutral"));
    assert!(added.iter().all(|i| std::path::Path::new(&i.path).is_file()));

    let rules = image_rules(&added).unwrap();
    assert!(rules.contains("admiration, amusement, anger"));
    let (text, refs) = sanitize_cg_tags("*smiles* <cg>Joy</cg> <cg>happy</cg>", &added);
    assert_eq!(text, "*smiles* <cg>joy</cg> ");
    assert_eq!(refs.len(), 1);
}

#[tokio::test]
async fn st_persona_avatar_is_not_a_card() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let err = import_file(&db, dir.path(), &st_sample("persona/user-default.png"), None)
        .await
        .err()
        .expect("데이터 없는 PNG는 거부");
    assert!(err.to_response().message.contains("캐릭터 카드 데이터"));
}

fn st_chat_sample() -> std::path::PathBuf {
    let dir = st_sample("chats/Seraphina");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .expect("채팅 샘플")
}

#[tokio::test]
async fn st_chat_attaches_to_character_by_name() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    let (character, _) = import_seraphina(&db, dir.path()).await;

    let SillyTavernImport::Chat {
        conversation,
        message_count,
        hidden_count,
        warnings,
    } = import_file(&db, dir.path(), &st_chat_sample(), None).await.unwrap()
    else {
        panic!("채팅이어야 함")
    };
    assert_eq!(conversation.character_id, character.id);
    assert_eq!(conversation.mode, "chat");
    assert!(conversation.title.starts_with("Seraphina - 2026-03-18"));
    assert_eq!((message_count, hidden_count), (12, 3));
    assert_eq!(conversation.message_count, 12);
    assert_eq!(warnings.len(), 1, "첫 메시지의 swipe_id(5)가 스와이프 수와 맞지 않음: {warnings:?}");
    assert_eq!(conversation.meta.as_ref().unwrap()["importedFrom"], "sillytavern");

    let msgs = db::conversation::get_messages(&db, &conversation.id, 100, None).await.unwrap();
    assert_eq!(
        msgs.iter().map(|m| m.seq).collect::<Vec<_>>(),
        (1..=12).collect::<Vec<_>>(),
        "시간 순서 = 원래 순서"
    );
    assert_eq!(msgs[0].content, "mes content <cg>amusement</cg>");
    assert_eq!(msgs[0].character_id.as_deref(), Some(character.id.as_str()));
    let meta0 = msgs[0].meta.as_ref().unwrap();
    assert_eq!(meta0["hidden"], true);
    assert_eq!(meta0["sender"]["id"], character.id.as_str());
    assert_eq!(msgs[5].role, "user");
    assert_eq!(msgs[5].character_id, None);
    assert_eq!(msgs[2].meta.as_ref().unwrap()["info"]["model"], "gemini-3.7-flash");

    // 5-E-3: 목록 시각은 마지막 메시지 시각, 메시지의 updated_at은 가져온 때 (동기화가 바뀐 행으로 고르게)
    let last = msgs.iter().map(|m| m.created_at).max().unwrap();
    assert_eq!(conversation.last_message_at, Some(last));
    assert!(last < conversation.created_at, "샘플은 예전 대화");
    assert!(msgs.iter().all(|m| m.updated_at >= conversation.created_at));

    // 프롬프트용 기록에서는 숨긴 메시지(3개)가 빠진다
    let ctx = crate::service::chat::context_manager::ContextManager
        .build_context(
            &db,
            crate::models::ai_service::ContextRequest {
                conversation_id: conversation.id.clone(),
                character_id: character.id.clone(),
                persona_id: conversation.persona_id.clone(),
                history_limit: 100,
            },
        )
        .await
        .unwrap();
    assert_eq!(ctx.history.len(), 9);
    assert_eq!(ctx.world_info.len(), 4, "캐릭터에 연결된 Eldoria 로어북");
    assert!(ctx.history.iter().all(|h| !h.content.contains("<cg>amusement")));
}

#[tokio::test]
async fn st_chat_needs_a_character() {
    let db = pool().await;
    let dir = tempfile::tempdir().unwrap();
    // 캐릭터가 없으면 화자 목록과 함께 거부
    let err = import_file(&db, dir.path(), &st_chat_sample(), None).await.err().unwrap().to_response();
    assert_eq!(err.details.unwrap()["speakers"], json!(["Seraphina"]));

    // 직접 고르면 이름이 달라도 그 캐릭터에 붙는다
    sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('other', 'Someone', 0, 0)")
        .execute(&db)
        .await
        .unwrap();
    let SillyTavernImport::Chat { conversation, .. } = import_file_with(
        &db,
        dir.path(),
        &st_chat_sample(),
        ImportOptions {
            name: Some("가져온 대화".into()),
            character_id: Some("other".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap() else {
        panic!()
    };
    assert_eq!((conversation.character_id.as_str(), conversation.title.as_str()), ("other", "가져온 대화"));
    let msgs = db::conversation::get_messages(&db, &conversation.id, 100, None).await.unwrap();
    assert_eq!(msgs[0].character_id.as_deref(), Some("other"));
}

// =======================================
// SillyTavern 전체 데이터 zip (5-E-20-4-1)
// =======================================

#[tokio::test]
async fn st_data_zip_imports_everything_and_links_by_file_name() {
    use std::io::Write;
    let db = pool().await;
    let app = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let u = src.path();
    for d in [
        "characters",
        "chats/default_Seraphina",
        "worlds",
        "OpenAI Settings",
        "TextGen Settings",
        "User Avatars",
        "groups",
    ] {
        std::fs::create_dir_all(u.join(d)).unwrap();
    }
    std::fs::copy(st_sample("character/default_Seraphina.png"), u.join("characters/default_Seraphina.png")).unwrap();
    write(
        &u.join("characters"),
        "kai.json",
        json!({ "spec": "chara_card_v2", "data": { "name": "카이", "first_mes": "안녕", "extensions": { "world": "Moon" } } })
            .to_string()
            .as_bytes(),
    );
    std::fs::copy(st_chat_sample(), u.join("chats/default_Seraphina/chat 1.jsonl")).unwrap();
    write(
        &u.join("worlds"),
        "Moon.json",
        json!({ "entries": { "0": { "key": ["달"], "content": "달이 붉다" } } }).to_string().as_bytes(),
    );
    std::fs::copy(st_sample("presets/Default.json"), u.join("OpenAI Settings/Default.json")).unwrap();
    write(&u.join("TextGen Settings"), "Sampler.json", b"{}");
    std::fs::copy(st_sample("persona/user-default.png"), u.join("User Avatars/user-default.png")).unwrap();
    write(&u.join("groups"), "g.json", b"{}");
    write(
        u,
        "settings.json",
        json!({
            "extension_settings": { "regex": [{ "scriptName": "Asterisks", "findRegex": "/\\*/g", "replaceString": "", "placement": [2] }] },
            "power_user": {
                "personas": { "user-default.png": "Min" },
                "persona_descriptions": { "user-default.png": { "description": "숲을 헤매는 여행자" } }
            }
        })
        .to_string()
        .as_bytes(),
    );

    // 폴더째 압축한 모양: backup/data/default-user/...
    let zip_path = app.path().join("st.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
    let mut stack = vec![u.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path.strip_prefix(u).unwrap().to_string_lossy().replace('\\', "/");
            zip.start_file(format!("backup/data/default-user/{rel}"), zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&std::fs::read(&path).unwrap()).unwrap();
        }
    }
    zip.start_file("../evil.txt", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"x").unwrap();
    zip.finish().unwrap();

    let r = data::import_data_zip(&db, app.path(), &zip_path).await.unwrap();
    assert_eq!(
        (r.characters, r.chats, r.books, r.presets, r.regex, r.personas),
        (2, 1, 1, 1, 1, 1),
        "{:?}",
        r.warnings
    );
    assert!(r.warnings.iter().any(|w| w.starts_with("TextGen Settings")), "{:?}", r.warnings);
    assert!(r.warnings.iter().any(|w| w.contains("단체 채팅")), "{:?}", r.warnings);
    assert!(!app.path().parent().unwrap().join("evil.txt").exists(), "zip 밖으로 풀지 않음");

    let chars = crate::db::conversation::get_all_characters(&db).await.unwrap();
    let kai = chars.iter().find(|c| c.name == "카이").unwrap();
    let books = crate::db::conversation::get_all_worldinfo_books(&db).await.unwrap();
    let moon = books.iter().find(|b| b.name == "Moon").unwrap();
    assert_eq!(kai.world_info_ids, vec![moon.id.clone()], "카드의 extensions.world → 로어북 연결");
    let sera = chars.iter().find(|c| c.name == "Seraphina").unwrap();
    let convs = crate::db::conversation::get_all_conversations(&db).await.unwrap();
    assert!(convs.iter().any(|c| c.character_id == sera.id), "chats/<카드 파일 이름> → 그 캐릭터");
    let personas = crate::db::conversation::get_all_personas(&db).await.unwrap();
    let min = personas.iter().find(|p| p.name == "Min").unwrap();
    assert_eq!(min.description, "숲을 헤매는 여행자");
    assert!(min.avatar.as_deref().is_some_and(|a| std::path::Path::new(a).is_file()));
}

#[test]
fn st_user_root_is_found_under_wrapping_folders() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/data/default-user/characters")).unwrap();
    std::fs::create_dir_all(dir.path().join("a/other")).unwrap();
    assert_eq!(data::find_user_root(dir.path()).unwrap(), dir.path().join("a/data/default-user"));
    assert!(data::find_user_root(&dir.path().join("a/other")).is_none());
}

#[test]
fn st_zip_unpacks_only_what_it_imports_and_skips_oversized() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("st.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
    let opt = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("u/characters/a.png", vec![1u8; 10]),
        ("u/characters/a/joy.png", vec![1; 10]),
        ("u/chats/a/c.jsonl", vec![1; 10]),
        ("u/backgrounds/bg.png", vec![1; 10]),
        ("u/settings.json", b"{}".to_vec()),
        ("u/worlds/big.json", vec![b' '; 5000]),
    ] {
        zip.start_file(name, opt).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
    let out = dir.path().join("out");
    let limits = data::ZipLimits {
        file_bytes: 1000,
        ..data::ZIP_LIMITS
    };
    let skipped = data::extract_zip(&zip_path, &out, limits).unwrap();
    assert!(out.join("u/characters/a.png").is_file());
    assert!(out.join("u/chats/a/c.jsonl").is_file());
    assert!(out.join("u/settings.json").is_file());
    assert!(!out.join("u/characters/a/joy.png").exists(), "표정 그림은 풀지 않음");
    assert!(!out.join("u/backgrounds").exists(), "배경은 풀지 않음");
    assert!(!out.join("u/worlds/big.json").exists());
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(skipped[0].starts_with("u/worlds/big.json"));

    let too_big = data::ZipLimits {
        zip_bytes: 10,
        ..data::ZIP_LIMITS
    };
    assert!(data::extract_zip(&zip_path, &dir.path().join("o2"), too_big).is_err());
}
